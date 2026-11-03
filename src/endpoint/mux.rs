use std::future::poll_fn;

use futures::io::{AsyncRead, AsyncWrite};
use yamux::{Config, Mode, Result, Stream};

pub struct Endpoint<S> {
    tcp_stream: yamux::Connection<S>,
}

impl<S> Endpoint<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn new(tcp_stream: S, mode: Mode) -> Self {
        Self {
            tcp_stream: yamux::Connection::new(tcp_stream, Config::default(), mode),
        }
    }

    pub fn server(tcp_stream: S) -> Self {
        Endpoint::new(tcp_stream, Mode::Server)
    }

    pub fn client(tcp_stream: S) -> Self {
        Endpoint::new(tcp_stream, Mode::Client)
    }

    pub async fn create_stream(&mut self) -> Result<Stream> {
        poll_fn(|cx| self.tcp_stream.poll_new_outbound(cx)).await
    }

    pub async fn accept_stream(&mut self) -> Result<Option<Stream>> {
        poll_fn(|cx| self.tcp_stream.poll_next_inbound(cx))
            .await
            .transpose()
    }

    pub async fn close(&mut self) -> Result<()> {
        poll_fn(|cx| self.tcp_stream.poll_close(cx)).await
    }
}

impl<S> std::ops::Deref for Endpoint<S> {
    type Target = yamux::Connection<S>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.tcp_stream
    }
}

impl<S> std::ops::DerefMut for Endpoint<S> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tcp_stream
    }
}
