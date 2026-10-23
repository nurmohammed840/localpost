use std::{
    io::{ErrorKind, Result},
    net::UdpSocket,
    time::Duration,
};

pub trait DatagramSocket {
    fn set_read_timeout(&self, dur: Option<Duration>) -> Result<()>;
    fn read_timeout(&self) -> Result<Option<Duration>>;

    fn send(&self, buf: &[u8]) -> Result<()>;
    fn recv(&self, buf: &mut [u8]) -> Result<usize>;

    fn recv_many(&self, bufs: &mut Vec<Box<[u8]>>, size: u16, limit: u8) -> Result<()>;
}

impl DatagramSocket for UdpSocket {
    fn set_read_timeout(&self, dur: Option<Duration>) -> Result<()> {
        UdpSocket::set_read_timeout(self, dur)
    }

    fn read_timeout(&self) -> Result<Option<Duration>> {
        UdpSocket::read_timeout(self)
    }

    fn send(&self, buf: &[u8]) -> Result<()> {
        loop {
            match UdpSocket::send(self, buf) {
                Err(err) if err.kind() == ErrorKind::Interrupted => continue,
                result => return result.map(|_| ()),
            }
        }
    }

    fn recv(&self, buf: &mut [u8]) -> Result<usize> {
        loop {
            match UdpSocket::recv(self, buf) {
                Err(err) if err.kind() == ErrorKind::Interrupted => continue,
                len => return len,
            }
        }
    }

    /// `limit` must be greater than 1.
    ///
    /// If an error occurs, `bufs` retains any packets received before the error.
    fn recv_many(&self, bufs: &mut Vec<Box<[u8]>>, size: u16, limit: u8) -> Result<()> {
        assert!(limit > 1);
        let start = bufs.len();

        let mut buf = vec![0; size.into()];

        // Waits for the first packet, respecting the read timeout.
        let len = DatagramSocket::recv(self, &mut buf)?;
        bufs.push(buf[..len].into());

        self.set_nonblocking(true)?;

        let result = loop {
            if bufs.len() - start >= limit.into() {
                break Ok(());
            }

            match DatagramSocket::recv(self, &mut buf) {
                Ok(len) => bufs.push(buf[..len].into()),
                Err(err) if matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                    break Ok(());
                }
                Err(err) => break Err(err),
            }
        };

        result.and(self.set_nonblocking(false))
    }
}
