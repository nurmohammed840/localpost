pub mod frame;
pub mod handshake;
pub mod mux;

use crate::{
    Result,
    file::{FileReader, FileWriter},
    protocol::{ErrorFrame, FileInfo, SystemInfo, TransferID},
    receiver::{FileReceiver, RecvStatus},
    sender::FileSender,
};
use lipi::{Decode, Encode};
use nio::net::{TcpListener, TcpStream};
use std::{
    fs::File,
    io::{Seek, Write},
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use yamux::Mode;

type MuxEndpoint = mux::Endpoint<TcpStream>;

pub fn create_udp_socket(local_addr: SocketAddr) -> Result<UdpSocket> {
    let socket: UdpSocket = UdpSocket::bind((local_addr.ip(), 0))?;
    socket.set_read_timeout(Some(Duration::from_secs(7)))?;
    Ok(socket)
}

pub struct Listener {
    tcp: TcpListener,
}

impl Listener {
    pub async fn new(port: u16) -> Result<Self> {
        let local_ip = local_ip_address::local_ip()?;
        Self::listen((local_ip, port)).await
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
    async fn handshake(mut self) -> Result<Endpoint> {
        handshake::send_hello(&mut self.stream).await?;
        Ok(Endpoint {
            local_addr: self.stream.local_addr()?,
            peer_addr: self.stream.peer_addr()?,
            endpoint: MuxEndpoint::server(self.stream),
            mode: Mode::Server,
        })
    }
}

pub struct Endpoint {
    endpoint: MuxEndpoint,
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    mode: Mode,
}

impl Endpoint {
    pub async fn open(addr: impl ToSocketAddrs) -> Result<(Endpoint, SystemInfo)> {
        let mut stream = TcpStream::connect(addr).await?;

        handshake::send_hello(&mut stream).await?;
        let hello = handshake::read_hello(&mut stream).await?;

        Ok((
            Endpoint {
                local_addr: stream.local_addr()?,
                peer_addr: stream.peer_addr()?,
                endpoint: MuxEndpoint::client(stream),
                mode: Mode::Client,
            },
            hello.info,
        ))
    }

    pub async fn accept_request(&mut self) -> Result<Option<FileRequest>> {
        let req = self
            .endpoint
            .accept_stream()
            .await?
            .map(|stream| FileRequest {
                stream,
                info: None,
                local_addr: self.local_addr,
                peer_addr: self.peer_addr,
                mode: self.mode,
            });

        Ok(req)
    }

    pub async fn send_file(&mut self, path: PathBuf, frame_size: u16) -> Result<()> {
        let mut stream = self.endpoint.create_stream().await?;

        let (reader, mut file_info) =
            nio::spawn_blocking(move || FileReader::open(path, frame_size)).await??;

        let sender_socket = create_udp_socket(self.local_addr)?;
        file_info.tid.udp_port = sender_socket.local_addr()?.port();

        match self.mode {
            Mode::Client => {
                stream.send(file_info).await?;
                let tid: TransferID = stream.recv().await?;
                let receiver_socket_addr = (self.peer_addr.ip(), tid.udp_port);
                sender_socket.connect(receiver_socket_addr)?;

                let task = nio::spawn_blocking(|| -> Result<()> {
                    FileSender::new(reader, sender_socket).init_send()
                });

                task.await?
            }
            Mode::Server => {
                let random_id = fastrand::u32(..);
                file_info.tid.id = Some(random_id);

                let task = nio::spawn_blocking(move || -> Result<()> {
                    wait_for_signal(&sender_socket, random_id)?;
                    FileSender::new(reader, sender_socket).init_send()
                });

                stream.send(file_info).await?;
                task.await?
            }
        }
    }
}

pub struct FileRequest {
    info: Option<FileInfo>,
    stream: mux::Stream,
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    mode: Mode,
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
        let mut file_info = self.file_info().await?.clone();
        let receiver_socket = create_udp_socket(self.local_addr)?;
        let sender_socket_addr = (self.peer_addr.ip(), file_info.tid.udp_port);

        receiver_socket.connect(sender_socket_addr)?;
        file_info.tid.udp_port = receiver_socket.local_addr()?.port();

        if self.mode == Mode::Client {
            receiver_socket.send(&file_info.tid.to_bytes()?)?;
        }

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

        if self.mode == Mode::Server {
            self.stream.send(file_info.tid).await?;
        }

        task.await?
    }
}

fn wait_for_signal(socket: &UdpSocket, random_id: u32) -> Result<()> {
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
