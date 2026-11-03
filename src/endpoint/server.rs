use std::{
    io::{Seek, Write},
    rc::Rc,
    sync::Arc,
};

use super::*;
use crate::{Result, endpoint::frame::FrameDecoder, protocol::FileInfo, receiver::RecvStatus};

// fn receive_file(&mut self, file_info: FileInfo, acceptor: FileAcceptor) -> Result<()> {
//     let FileAcceptor {
//         file,
//         status,
//         callback,
//     } = acceptor;

//     let writer = FileWriter::new(file, file_info.frame_size - 4);

//     let sender_socket_addr = (self.conn.peer_addr.ip(), file_info.tid.udp_port);
//     let receiver_socket = self.conn.create_udp_socket()?;

//     receiver_socket.connect(sender_socket_addr)?;

//     self.conn.send(TransferID {
//         udp_port: receiver_socket.local_addr()?.port(),
//         file_id: file_info.tid.file_id,
//     })?;

//     let mut receiver = FileReceiver::new(writer, receiver_socket, &file_info)?;

//     thread::spawn(move || {
//         let result = receiver.init_receive(status);

//         if let Err(err) = &result
//             && !err.is::<ErrorFrame>()
//         {
//             let _ = receiver.send_error(err.to_string());
//         }

//         callback(result);
//     });
//     Ok(())
// }

pub async fn run<C: Connection>(port: u16, interface: Rc<C>) -> Result<()> {
    let mut listener = Listener::new(port).await?;
    let mut id = 0;

    let mut connection_handler = async || -> Result<()> {
        id += 1;

        let (mut stream, info) = listener.accept().await?;

        let accept_conn = async move |interface: Rc<C>| -> Result<()> {
            handshake::send_hello(&mut stream).await?;
            let mut endpoint = Endpoint::server(stream);

            while let Some(stream) = endpoint.accept_stream().await? {
                let mut decoder = FrameDecoder::new(stream);
                let file_info: FileInfo = decoder.decode().await?;

                if let Some(_acceptor) = interface.file_request(id, &file_info).await? {
                    // _acceptor
                }
            }

            Ok(())
        };

        let interface = interface.clone();

        nio::spawn_local(async move {
            let should_accept = interface.new_connection(id, info).await;
            if !should_accept {
                return;
            }
            let result = accept_conn(interface.clone()).await;
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
    _file: FileObj,
    status: Arc<RecvStatus>,
    _callback: OnCompleteCallback,
}

impl FileAcceptor {
    fn _new<W, CB>(file: W, cb: CB) -> FileAcceptor
    where
        W: Write + Seek + Send + Sync + 'static,
        CB: FnOnce(Result<()>) + Send + Sync + 'static,
    {
        Self {
            _file: Box::new(file),
            _callback: Box::new(cb),
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
    async fn new_connection(&self, _conn_id: u32, info: SystemInfo) -> bool {
        println!("New Connection: {info:#?}");
        true
    }

    // return `true` to accept File
    async fn file_request(
        &self,
        _conn_id: u32,
        _file_info: &FileInfo,
    ) -> Result<Option<FileAcceptor>> {
        // println!("write req: {file_info:#?}");

        // let file = File::options()
        //     .truncate(false)
        //     .create(true)
        //     .write(true)
        //     .open("file.txt")?;

        // let acceptor = FileAcceptor::new(file, |res| {
        //     if let Err(err) = res {
        //         eprintln!("{err}")
        //     }
        // });

        // Ok(Some(acceptor))
        Ok(None)
    }

    async fn closed(&self, _conn_id: u32, _: Result<()>);
}
