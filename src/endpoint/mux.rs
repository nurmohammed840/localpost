use std::{
    cell::RefCell,
    future::poll_fn,
    io,
    pin::Pin,
    task::{Context, Poll},
};

use futures::io::{AsyncRead, AsyncWrite};
use yamux::{Config, Connection, Mode, Result};

use crate::endpoint::frame::{FrameDecoder, FrameEncoder};

pub struct Endpoint<S> {
    tcp_stream: RefCell<Connection<S>>,
}

impl<S> Endpoint<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn new(tcp_stream: S, mode: Mode) -> Self {
        Self {
            tcp_stream: RefCell::new(Connection::new(tcp_stream, Config::default(), mode)),
        }
    }

    pub fn server(tcp_stream: S) -> Self {
        Endpoint::new(tcp_stream, Mode::Server)
    }

    pub fn client(tcp_stream: S) -> Self {
        Endpoint::new(tcp_stream, Mode::Client)
    }

    pub async fn create_stream(&self) -> Result<Stream> {
        poll_fn(|cx| self.tcp_stream.borrow_mut().poll_new_outbound(cx))
            .await
            .map(Stream::new)
    }

    pub async fn accept_stream(&self) -> Result<Option<Stream>> {
        poll_fn(|cx| self.tcp_stream.borrow_mut().poll_next_inbound(cx))
            .await
            .transpose()
            .map(|maybe| maybe.map(Stream::new))
    }

    pub async fn close(&self) -> Result<()> {
        poll_fn(|cx| self.tcp_stream.borrow_mut().poll_close(cx)).await
    }
}

pub struct Stream {
    pub io: RefCell<yamux::Stream>,
}

impl Stream {
    pub fn new(io: yamux::Stream) -> Self {
        Self {
            io: RefCell::new(io),
        }
    }

    pub async fn recv<T: lipi::DecodeOwned>(&self) -> anyhow::Result<T> {
        FrameDecoder::new(self).decode().await
    }

    pub async fn send<T: lipi::Encode>(&self, data: T) -> anyhow::Result<()> {
        FrameEncoder::new(self).encode(data).await
    }
}

impl AsyncRead for &Stream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let mut io = self.io.borrow_mut();
        Pin::new(&mut *io).poll_read(cx, buf)
    }
}

impl AsyncWrite for &Stream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        let mut io = self.io.borrow_mut();
        Pin::new(&mut *io).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut io = self.io.borrow_mut();
        Pin::new(&mut *io).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut io = self.io.borrow_mut();
        Pin::new(&mut *io).poll_close(cx)
    }
}
