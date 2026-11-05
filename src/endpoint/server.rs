use super::*;
use crate::{
    Result,
    file::{FileReader, FileWriter},
    protocol::{ErrorFrame, FileInfo, TransferID},
    receiver::{FileReceiver, RecvStatus},
    sender::FileSender,
};

use lipi::Decode;
use nio::net::TcpListener;
use std::{
    fs::File,
    io::{Seek, Write},
    net::Ipv4Addr,
    path::PathBuf,
    sync::Arc,
};

pub struct Listener {
    tcp: TcpListener,
}

impl Listener {
    pub async fn new(port: u16) -> Result<Self> {
        match local_ip_address::local_ip() {
            Ok(local_ip) => Self::listen((local_ip, port)).await,
            Err(_) => Self::listen((Ipv4Addr::UNSPECIFIED, port)).await,
        }
    }

    pub async fn listen(addr: impl ToSocketAddrs) -> Result<Self> {
        Ok(Listener {
            tcp: TcpListener::bind(addr).await?,
        })
    }

    pub async fn accept(&mut self) -> Result<(Connection, SystemInfo)> {
        let conn = self.tcp.accept().await?;
        let mut stream = conn.connect().await?;

        let hello = handshake::read_hello(&mut stream).await?;
        Ok((Connection { stream }, hello.info))
    }
}
pub struct Connection {
    stream: TcpStream,
}

impl Connection {
    async fn handshake(mut self) -> Result<ServerEndpoint> {
        handshake::send_hello(&mut self.stream).await?;
        Ok(ServerEndpoint {
            peer_addr: self.stream.peer_addr()?,
            local_addr: self.stream.local_addr()?,
            endpoint: Endpoint::server(self.stream),
        })
    }
}

pub struct ServerEndpoint {
    endpoint: Endpoint,
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
}

impl ServerEndpoint {
    pub async fn accept_request(&mut self) -> Result<Option<FileRequest>> {
        Ok(self
            .endpoint
            .accept_stream()
            .await?
            .map(|stream| FileRequest {
                stream,
                info: None,
                local_addr: self.local_addr,
                peer_addr: self.peer_addr,
            }))
    }

    pub async fn send_file(&mut self, path: PathBuf, frame_size: u16) -> Result<()> {
        let random_id = fastrand::u32(..);
        let mut stream = self.endpoint.create_stream().await?;

        let (reader, mut file_info) =
            nio::spawn_blocking(move || FileReader::open(path, frame_size)).await??;

        let sender_socket = create_udp_socket(self.local_addr)?;
        file_info.tid.udp_port = sender_socket.local_addr()?.port();
        file_info.tid.id = Some(random_id);

        let wait_for_signal = move |socket: &UdpSocket| -> Result<()> {
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
                    return Ok(());
                }
            }

            Err("Unable to connect".into())
        };

        nio::spawn_blocking(move || -> Result<()> {
            wait_for_signal(&sender_socket)?;

            let mut sender = FileSender::new(reader, sender_socket);
            sender.init_send()?;
            Ok(())
        });

        stream.send(file_info).await?;

        Ok(())
    }
}

pub struct FileRequest {
    info: Option<FileInfo>,
    stream: mux::Stream,
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
}

impl FileRequest {
    pub async fn file_info(&mut self) -> Result<&FileInfo> {
        if let Some(file_info) = &self.info {
            return Ok(file_info);
        }
        let file_info = self.stream.recv().await?;
        Ok(self.info.insert(file_info))
    }

    pub async fn receive(
        mut self,
        file: impl Write + Seek + Send + 'static,
        status: Arc<RecvStatus>,
    ) -> Result<()> {
        let file_info = self.file_info().await?.clone();
        let receiver_socket = create_udp_socket(self.local_addr)?;
        let sender_socket_addr = (self.peer_addr.ip(), file_info.tid.udp_port);

        receiver_socket.connect(sender_socket_addr)?;

        let tid = TransferID::new(receiver_socket.local_addr()?.port());

        let writer = FileWriter::new(file, file_info.frame_size - 4);
        let mut receiver = FileReceiver::new(writer, receiver_socket, &file_info);

        let task = nio::spawn_blocking(move || -> Result<()> {
            let result = receiver.init_receive(status);

            if let Err(err) = &result
                && !err.is::<ErrorFrame>()
            {
                receiver.send_error(err.to_string())?;
            }

            result
        });

        self.stream.send(tid).await?;
        task.await??;
        Ok(())
    }
}

pub async fn run_server() -> Result<()> {
    let mut listener = Listener::new(50001).await?;

    loop {
        let (conn, info) = match listener.accept().await {
            Ok(conn) => conn,
            Err(_) => continue,
        };

        nio::spawn_local(async move {
            let mut endpoint = conn.handshake().await.unwrap();

            while let Some(mut request) = endpoint.accept_request().await.unwrap() {
                nio::spawn_local(async move {
                    let info = request.file_info().await.unwrap().clone();
                    println!("File Request: {info:?}");

                    let file = nio::spawn_blocking(|| {
                        File::options()
                            .truncate(false)
                            .create(true)
                            .write(true)
                            .open("file.txt")
                    })
                    .await
                    .unwrap()
                    .unwrap();

                    let status = RecvStatus::new();

                    request.receive(file, status).await.unwrap();
                    println!("File Download Complete: {info:?}")
                });
            }

            println!("Connection Closed: {}", info.hostname);
        });
    }
}
