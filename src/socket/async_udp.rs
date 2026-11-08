use super::AsyncDatagramSocket;
use futures::FutureExt;
use std::{
    cell::Cell,
    future::poll_fn,
    io::{self, ErrorKind},
    task::Poll,
    time::Duration,
};

pub struct UdpSocket {
    socket: nio::net::UdpSocket,
    timeout: Cell<Option<Duration>>,
}

impl UdpSocket {
    pub fn from(socket: std::net::UdpSocket) -> io::Result<Self> {
        Ok(Self {
            socket: nio::net::UdpSocket::from_std(socket)?,
            timeout: Cell::new(None),
        })
    }

    async fn recv_now_or_never(&self, buf: &mut [u8]) -> Option<io::Result<usize>> {
        poll_fn(|cx| match self.socket.recv(buf).poll_unpin(cx) {
            Poll::Ready(x) => Poll::Ready(Some(x)),
            Poll::Pending => Poll::Ready(None),
        })
        .await
    }
}

impl AsyncDatagramSocket for UdpSocket {
    fn set_read_timeout(&self, dur: Option<Duration>) -> io::Result<()> {
        self.timeout.set(dur);
        Ok(())
    }

    fn read_timeout(&self) -> io::Result<Option<Duration>> {
        Ok(self.timeout.get())
    }

    async fn send(&self, buf: &[u8]) -> io::Result<()> {
        let len = self.socket.send(buf).await?;
        if len != buf.len() {
            return Err(io::Error::new(
                ErrorKind::WriteZero,
                "datagram partially sent",
            ));
        }
        Ok(())
    }

    async fn recv(&self, buf: &mut [u8]) -> io::Result<usize> {
        let recv = self.socket.recv(buf);

        match self.read_timeout()? {
            None => recv.await,
            Some(duration) => match nio::timeout(duration, recv).await {
                None => Err(io::Error::new(ErrorKind::TimedOut, "read timed out")),
                Some(result) => result,
            },
        }
    }

    async fn recv_many(&self, bufs: &mut Vec<Box<[u8]>>, size: u16, limit: u8) -> io::Result<()> {
        assert!(limit > 1);
        let start = bufs.len();

        let mut buf = vec![0; size as usize];
        let len = self.recv(&mut buf).await?;
        bufs.push(buf[..len].into());

        loop {
            if bufs.len() - start >= limit.into() {
                break Ok(());
            }

            match self.recv_now_or_never(&mut buf).await {
                Some(len) => bufs.push(buf[..len?].into()),
                None => break Ok(()),
            }
        }
    }
}
