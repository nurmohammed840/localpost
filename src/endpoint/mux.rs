use std::{cell::RefCell, future::poll_fn, rc::Rc};

use futures::io::{AsyncRead, AsyncWrite};
use yamux::{Config, Connection, Mode, Result};

use crate::endpoint::frame::{FrameDecoder, FrameEncoder};

pub struct Endpoint<S> {
    tcp_stream: Rc<RefCell<Connection<S>>>,
}

impl<S> Endpoint<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn new(tcp_stream: S, mode: Mode) -> Self {
        Self {
            tcp_stream: Rc::new(RefCell::new(Connection::new(
                tcp_stream,
                Config::default(),
                mode,
            ))),
        }
    }

    pub fn server(tcp_stream: S) -> Self {
        Endpoint::new(tcp_stream, Mode::Server)
    }

    pub fn client(tcp_stream: S) -> Self {
        Endpoint::new(tcp_stream, Mode::Client)
    }

    pub async fn create_stream(&mut self) -> Result<Stream> {
        poll_fn(|cx| self.tcp_stream.borrow_mut().poll_new_outbound(cx))
            .await
            .map(Stream::new)
    }

    pub async fn accept_stream(&mut self) -> Result<Option<Stream>> {
        poll_fn(|cx| self.tcp_stream.borrow_mut().poll_next_inbound(cx))
            .await
            .transpose()
            .map(|maybe| maybe.map(Stream::new))
    }

    pub async fn close(&mut self) -> Result<()> {
        poll_fn(|cx| self.tcp_stream.borrow_mut().poll_close(cx)).await
    }
}

impl<S> Clone for Endpoint<S> {
    fn clone(&self) -> Self {
        Self {
            tcp_stream: self.tcp_stream.clone(),
        }
    }
}

pub struct Stream {
    pub io: yamux::Stream,
}

impl Stream {
    pub fn new(io: yamux::Stream) -> Self {
        Self { io }
    }

    pub async fn recv<T: lipi::DecodeOwned>(&mut self) -> crate::Result<T> {
        FrameDecoder::new(&mut self.io).decode().await
    }

    pub async fn send<T: lipi::Encode>(&mut self, data: T) -> crate::Result<()> {
        FrameEncoder::new(&mut self.io).encode(data).await
    }
}
