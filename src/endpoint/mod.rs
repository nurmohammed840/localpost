pub mod client;
pub mod frame;
pub mod handshake;
pub mod mux;
pub mod server;

use crate::{Result, protocol::SystemInfo};
use nio::net::TcpStream;
use std::{
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    time::Duration,
};

type Endpoint = mux::Endpoint<TcpStream>;

pub fn create_udp_socket(local_addr: SocketAddr) -> Result<UdpSocket> {
    let socket: UdpSocket = UdpSocket::bind((local_addr.ip(), 0))?;
    socket.set_read_timeout(Some(Duration::from_secs(7)))?;
    Ok(socket)
}
