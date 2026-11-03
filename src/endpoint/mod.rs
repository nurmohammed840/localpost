pub mod client;
pub mod frame;
pub mod handshake;
pub mod mux;
pub mod server;

use crate::{Result, protocol::SystemInfo};
use nio::net::{TcpListener, TcpStream};
use std::{
    net::{Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket},
    time::Duration,
};

type Endpoint = mux::Endpoint<TcpStream>;

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

    pub async fn accept(&mut self) -> Result<(TcpStream, SystemInfo)> {
        let conn = self.tcp.accept().await?;
        let mut stream = conn.connect().await?;

        let hello = handshake::read_hello(&mut stream).await?;
        Ok((stream, hello.info))
    }
}

pub fn create_udp_socket(local_addr: SocketAddr) -> Result<UdpSocket> {
    let socket: UdpSocket = UdpSocket::bind((local_addr.ip(), 0))?;
    socket.set_read_timeout(Some(Duration::from_secs(7)))?;
    Ok(socket)
}
