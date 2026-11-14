pub mod frame;
pub mod handshake;
pub mod mux;

use crate::{
    file::{FileReader, FileWriter},
    protocol::{FileInfo, SystemInfo, TransferID},
    receiver::{FileReceiver, RecvStatus},
    sender::FileSender,
    socket::{AsyncDatagramSocket, async_udp::UdpSocket},
};
use anyhow::{Result, bail};
use lipi::{Decode, Encode};
use nio::net::{TcpListener, TcpStream};
use std::{
    cell::RefCell,
    fmt::Debug,
    io::{Seek, Write},
    net::{SocketAddr, ToSocketAddrs},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use yamux::Mode;

type MuxEndpoint = mux::Endpoint<TcpStream>;

pub fn create_udp_socket(local_addr: SocketAddr) -> Result<UdpSocket> {
    let socket = std::net::UdpSocket::bind((local_addr.ip(), 0))?;
    let socket = UdpSocket::try_from(socket)?;
    socket.set_read_timeout(Some(Duration::from_secs(30)))?;
    Ok(socket)
}

pub struct Listener {
    pub tcp: TcpListener,
}

impl Listener {
    pub async fn listen(addr: impl ToSocketAddrs) -> Result<Self> {
        Ok(Listener {
            tcp: TcpListener::bind(addr).await?,
        })
    }

    pub async fn accept(&self) -> Result<(Connection, SystemInfo)> {
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
    pub async fn handshake(mut self) -> Result<Endpoint> {
        let local_addr = self.stream.local_addr()?;
        handshake::send_hello(&mut self.stream, local_addr.port()).await?;
        Ok(Endpoint {
            local_addr,
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
    pub async fn open(addr: impl ToSocketAddrs, port: u16) -> Result<(Endpoint, SystemInfo)> {
        let mut stream = TcpStream::connect(addr).await?;

        handshake::send_hello(&mut stream, port).await?;
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

    pub async fn open_with_timeout(
        addr: impl ToSocketAddrs + Debug,
        duration: Duration,
        port: u16,
    ) -> Result<(Endpoint, SystemInfo)> {
        match nio::timeout(duration, Self::open(&addr, port)).await {
            None => bail!("Connection to {addr:?} timed out after {duration:?}"),
            Some(conn) => conn,
        }
    }

    pub async fn disconnect(&self) -> Result<()> {
        self.endpoint.close().await?;
        Ok(())
    }

    pub async fn accept_request(&self) -> Result<Option<FileRequest>> {
        let req = self
            .endpoint
            .accept_stream()
            .await?
            .map(|stream| FileRequest {
                stream,
                info: RefCell::new(None),
                local_addr: self.local_addr,
                peer_addr: self.peer_addr,
                mode: self.mode,
            });

        Ok(req)
    }

    pub async fn send_file(&self, path: PathBuf, frame_size: u16) -> Result<()> {
        let stream = self.endpoint.create_stream().await?;

        let (reader, mut file_info) =
            nio::spawn_blocking(move || FileReader::open(path, frame_size)).await??;

        let sender = create_udp_socket(self.local_addr)?;
        file_info.tid.udp_port = sender.socket.local_addr()?.port();

        match self.mode {
            Mode::Client => {
                stream.send(file_info).await?;
                let tid: TransferID = stream.recv().await?;
                let receiver_socket_addr = (self.peer_addr.ip(), tid.udp_port);
                sender.socket.connect(receiver_socket_addr).await?;

                FileSender::new(reader, sender).init_send().await
            }
            Mode::Server => {
                let random_id = fastrand::u32(..);
                file_info.tid.id = Some(random_id);

                let task = nio::spawn_local(async move {
                    wait_for_signal(&sender, random_id).await?;
                    FileSender::new(reader, sender).init_send().await
                });

                stream.send(file_info).await?;
                task.await?
            }
        }
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }
}

pub struct FileRequest {
    info: RefCell<Option<FileInfo>>,
    stream: mux::Stream,
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    mode: Mode,
}

impl FileRequest {
    pub async fn file_info(&self) -> Result<FileInfo> {
        let cached = self.info.borrow().clone();
        if let Some(info) = cached {
            return Ok(info);
        }

        let info: FileInfo = self.stream.recv().await?;
        *self.info.borrow_mut() = Some(info.clone());
        Ok(info)
    }

    pub async fn receive(
        &self,
        file: impl Write + Seek + Send + 'static,
        status: Arc<RecvStatus>,
    ) -> Result<()> {
        let mut file_info = self.file_info().await?.clone();
        let receiver = create_udp_socket(self.local_addr)?;
        let sender_socket_addr = (self.peer_addr.ip(), file_info.tid.udp_port);

        receiver.socket.connect(sender_socket_addr).await?;
        file_info.tid.udp_port = receiver.socket.local_addr()?.port();

        if self.mode == Mode::Client {
            receiver.send(&file_info.tid.to_bytes()?).await?;
        }

        let writer = FileWriter::new(file, file_info.frame_size - 4);
        let receiver = FileReceiver::new(writer, receiver, &file_info);

        let task = nio::spawn_local(async { receiver.init_receive(status).await });

        if self.mode == Mode::Server {
            self.stream.send(file_info.tid).await?;
        }

        task.await?
    }
}

async fn wait_for_signal(sender: &UdpSocket, random_id: u32) -> Result<()> {
    let mut buf = vec![0; 16];
    for _ in 0..7 {
        let (len, addr) = sender.recv_from(&mut buf).await?;

        let is_tid_matched = TransferID::decode(&mut &buf[..len])
            .ok()
            .is_some_and(|tid| {
                tid.udp_port == addr.port() && tid.id.is_some_and(|id| id == random_id)
            });

        if is_tid_matched {
            sender.socket.connect(addr).await?;
            return Ok(());
        }
    }

    bail!("Unable to connect")
}
