pub mod mock_udp;

use std::{io, net::UdpSocket, time::Duration};


pub trait DatagramSocket {
    fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()>;
    fn read_timeout(&self) -> io::Result<Option<Duration>>;

    fn send(&self, buf: &[u8]) -> io::Result<()>;
    fn recv(&self, buf: &mut [u8]) -> io::Result<usize>;
}

impl DatagramSocket for UdpSocket {
    fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        UdpSocket::set_read_timeout(self, dur)
    }

    fn read_timeout(&self) -> io::Result<Option<Duration>> {
        UdpSocket::read_timeout(self)
    }

    fn send(&self, buf: &[u8]) -> io::Result<()> {
        UdpSocket::send(self, buf)?;
        Ok(())
    }

    fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        UdpSocket::recv(self, buf)
    }
}
