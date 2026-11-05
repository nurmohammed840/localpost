use super::*;
use crate::{
    Result,
    endpoint::mux::Stream,
    file::{FileReader, FileWriter},
    protocol::{ErrorFrame, FileInfo, TransferID},
    receiver::{FileReceiver, RecvStatus},
    sender::FileSender,
};
use futures::channel::oneshot;
use lipi::Decode;
use std::{
    fs::File,
    io::{Seek, Write},
    net::SocketAddr,
    path::PathBuf,
    rc::Rc,
    sync::Arc,
    thread,
};

pub struct Server {
    local_addr: SocketAddr,
    endpoint: Endpoint,
}

impl Server {
    pub async fn send_file(&mut self, path: PathBuf, frame_size: u16) -> Result<()> {
        let random_id = fastrand::u32(..);
        let mut stream = self.endpoint.create_stream().await?;

        let (reader, mut file_info) =
            nio::spawn_blocking(move || FileReader::open(path, frame_size)).await??;

        let sender_socket = create_udp_socket(self.local_addr)?;
        file_info.tid.udp_port = sender_socket.local_addr()?.port();
        file_info.tid.id = Some(random_id);

        let wait_for_signal = move |socket: &UdpSocket| -> Result<bool> {
            let mut buf = vec![0; 16];
            for _ in 0..7 {
                let (len, addr) = socket.recv_from(&mut buf)?;

                let is_tid_matched = TransferID::decode(&mut &buf[..len])
                    .ok()
                    .is_some_and(|tid| {
                        tid.udp_port == addr.port() && tid.id.is_some_and(|id| id == random_id)
                    });

                if is_tid_matched {
                    socket.connect(addr)?;
                    return Ok(true);
                }
            }
            Ok(false)
        };

        nio::spawn_blocking(move || -> Result<()> {
            let is_connected = wait_for_signal(&sender_socket)?;
            if !is_connected {
                return Err("Unable to connect".into());
            }

            let mut sender = FileSender::new(reader, sender_socket);
            sender.init_send()?;
            Ok(())
        });

        stream.send(file_info).await?;

        Ok(())
    }
}

async fn receive_file(
    file_info: FileInfo,
    acceptor: FileAcceptor,
    peer_addr: SocketAddr,
    local_addr: SocketAddr,
    mut stream: Stream,
) -> Result<()> {
    let FileAcceptor {
        file,
        status,
        callback,
    } = acceptor;

    let receiver_socket = create_udp_socket(local_addr)?;
    let sender_socket_addr = (peer_addr.ip(), file_info.tid.udp_port);
    receiver_socket.connect(sender_socket_addr)?;

    let tid = TransferID::new(receiver_socket.local_addr()?.port());

    let writer = FileWriter::new(file, file_info.frame_size - 4);
    let mut receiver = FileReceiver::new(writer, receiver_socket, &file_info)?;
    thread::spawn(move || {
        let result = receiver.init_receive(status);

        if let Err(err) = &result
            && !err.is::<ErrorFrame>()
        {
            let _ = receiver.send_error(err.to_string());
        }

        callback(result)
    });

    stream.send(tid).await?;
    Ok(())
}

pub async fn run<C: Connection>(port: u16, interface: Rc<C>) -> Result<()> {
    let mut listener = Listener::new(port).await?;
    let mut id = 0;

    let mut connection_handler = async || -> Result<()> {
        id += 1;

        let (mut stream, info) = listener.accept().await?;
        let peer_addr = stream.peer_addr()?;
        let local_addr = stream.local_addr()?;

        let (tx, rx) = oneshot::channel::<Endpoint>();

        let _interface = interface.clone();
        let accept_conn = async move || -> Result<()> {
            handshake::send_hello(&mut stream).await?;

            let mut endpoint = Endpoint::server(stream);
            let _ = tx.send(endpoint.clone());

            while let Some(mut stream) = endpoint.accept_stream().await? {
                let interface = _interface.clone();

                let accept_req = async move || -> Result<()> {
                    let file_info: FileInfo = stream.recv().await?;

                    if let Some(acceptor) = interface.file_request(id, &file_info).await? {
                        receive_file(file_info, acceptor, peer_addr, local_addr, stream).await?;
                    }
                    Ok(())
                };

                nio::spawn_local(async {
                    if let Err(err) = accept_req().await {
                        eprintln!("{err:?}");
                    }
                });
            }

            Ok(())
        };

        let interface = interface.clone();
        nio::spawn_local(async move {
            let should_accept = interface.new_connection(id, info, rx);
            if !should_accept {
                return;
            }
            let result = accept_conn().await;
            interface.closed(id, result).await;
        });

        Ok(())
    };

    loop {
        if let Err(err) = connection_handler().await {
            eprintln!("{err}")
        }
    }
}

pub trait WriteSeek: Write + Seek + Send + Sync {}
impl<T: Write + Seek + ?Sized + Send + Sync> WriteSeek for T {}
pub type FileObj = Box<dyn WriteSeek>;

pub type OnCompleteCallback = Box<dyn FnOnce(Result<()>) + Send + Sync>;

pub struct FileAcceptor {
    file: FileObj,
    status: Arc<RecvStatus>,
    callback: OnCompleteCallback,
}

impl FileAcceptor {
    fn new<W, CB>(file: W, cb: CB) -> FileAcceptor
    where
        W: Write + Seek + Send + Sync + 'static,
        CB: FnOnce(Result<()>) + Send + Sync + 'static,
    {
        Self {
            file: Box::new(file),
            callback: Box::new(cb),
            status: RecvStatus::new(),
        }
    }

    pub fn status(&self) -> Arc<RecvStatus> {
        self.status.clone()
    }
}

#[allow(async_fn_in_trait)]
pub trait Connection: 'static {
    // return `true` to accept connection
    fn new_connection(
        &self,
        conn_id: u32,
        info: SystemInfo,
        _: oneshot::Receiver<Endpoint>,
    ) -> bool {
        println!("#{conn_id} New Connection; {info:#?}");
        true
    }

    // return `true` to accept File
    async fn file_request(
        &self,
        conn_id: u32,
        file_info: &FileInfo,
    ) -> Result<Option<FileAcceptor>> {
        println!("#{conn_id} write req: {file_info:#?}");

        let file = nio::spawn_blocking(|| {
            File::options()
                .truncate(false)
                .create(true)
                .write(true)
                .open("file.txt")
        })
        .await??;

        let acceptor = FileAcceptor::new(file, |res| {
            if let Err(err) = res {
                eprintln!("{err}")
            }
        });

        Ok(Some(acceptor))
    }

    async fn closed(&self, conn_id: u32, _: Result<()>) {
        println!("#{conn_id} Conn Closed");
    }
}
