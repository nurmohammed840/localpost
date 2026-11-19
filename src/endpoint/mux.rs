use std::{
    cell::RefCell,
    future::poll_fn,
    io,
    pin::Pin,
    task::{Context, Poll, ready},
};

use futures::{
    StreamExt,
    channel::{mpsc, oneshot},
    io::{AsyncRead, AsyncWrite},
};
use yamux::{Config, Connection, ConnectionError, Mode, Result};

use crate::endpoint::frame::{FrameDecoder, FrameEncoder};

pub struct Stream {
    io: RefCell<yamux::Stream>,
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

    pub async fn send_last_msg<T: lipi::Encode>(&self, data: T) -> anyhow::Result<()> {
        FrameEncoder::new(self).encode(data).await?;
        self.close_write().await?;
        Ok(())
    }

    pub async fn close_write(&self) -> io::Result<()> {
        poll_fn(|cx| Pin::new(&mut &*self).poll_close(cx)).await
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

type OpenReply = oneshot::Sender<Result<yamux::Stream>>;
type CloseReply = oneshot::Sender<Result<()>>;

pub struct Endpoint {
    open_tx: mpsc::UnboundedSender<OpenReply>,
    close_tx: mpsc::UnboundedSender<CloseReply>,
    inbound_rx: RefCell<mpsc::UnboundedReceiver<Result<yamux::Stream>>>,
}

impl Endpoint {
    fn new<S>(io: S, mode: Mode) -> Self
    where
        S: AsyncRead + AsyncWrite + Unpin + 'static,
    {
        let (open_tx, open_rx) = mpsc::unbounded();
        let (close_tx, close_rx) = mpsc::unbounded();
        let (inbound_tx, inbound_rx) = mpsc::unbounded();

        let conn = Connection::new(io, Config::default(), mode);
        nio::spawn_local(drive(conn, open_rx, close_rx, inbound_tx));

        Self {
            open_tx,
            close_tx,
            inbound_rx: RefCell::new(inbound_rx),
        }
    }

    pub fn server<S>(io: S) -> Self
    where
        S: AsyncRead + AsyncWrite + Unpin + 'static,
    {
        Endpoint::new(io, Mode::Server)
    }

    pub fn client<S>(io: S) -> Self
    where
        S: AsyncRead + AsyncWrite + Unpin + 'static,
    {
        Endpoint::new(io, Mode::Client)
    }

    pub async fn create_stream(&self) -> Result<Stream> {
        let (tx, rx) = oneshot::channel();
        self.open_tx
            .unbounded_send(tx)
            .map_err(|_| ConnectionError::Closed)?;

        rx.await
            .map_err(|_| ConnectionError::Closed)?
            .map(Stream::new)
    }

    pub async fn accept_stream(&self) -> Result<Option<Stream>> {
        poll_fn(|cx| self.inbound_rx.borrow_mut().poll_next_unpin(cx))
            .await
            .transpose()
            .map(|maybe| maybe.map(Stream::new))
    }

    pub async fn close(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        if self.close_tx.unbounded_send(tx).is_err() {
            return Ok(());
        }
        rx.await.unwrap_or(Ok(()))
    }
}

/// [yamux] does not support concurrent calls to poll_new_outbound and poll_next_inbound from different tasks.
///
/// See: https://github.com/libp2p/rust-yamux/issues/237
async fn drive<S>(
    mut conn: Connection<S>,
    mut open_rx: mpsc::UnboundedReceiver<OpenReply>,
    mut close_rx: mpsc::UnboundedReceiver<CloseReply>,
    inbound_tx: mpsc::UnboundedSender<Result<yamux::Stream>>,
) where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut pending_open: Option<OpenReply> = None;

    let close_signal = poll_fn(|cx| {
        // 1. Close if Endpoint droped
        if let Poll::Ready(signal) = close_rx.poll_next_unpin(cx) {
            return Poll::Ready(signal);
        }

        // 2. serve outbound opens
        loop {
            if pending_open.is_none() {
                match ready_or!(open_rx.poll_next_unpin(cx), break) {
                    None => return Poll::Ready(None),
                    Some(tx) => pending_open = Some(tx),
                }
            }

            if pending_open.as_ref().unwrap().is_canceled() {
                pending_open = None;
                continue;
            }

            let res = ready_or!(conn.poll_new_outbound(cx), break);
            _ = pending_open.take().unwrap().send(res);
        }

        // This drives all I/O.
        while let Some(item) = ready!(conn.poll_next_inbound(cx)) {
            if inbound_tx.unbounded_send(item).is_err() {
                return Poll::Ready(None);
            }
        }

        Poll::Ready(None)
    })
    .await;

    // Close gracefully. This is a no-op if the connection is already closed.
    let result = poll_fn(|cx| conn.poll_close(cx)).await;
    if let Some(tx) = close_signal {
        _ = tx.send(result);
    }
}

macro_rules! ready_or {
    ($poll:expr, $otherwise:expr) => {
        match $poll {
            ::core::task::Poll::Ready(v) => v,
            ::core::task::Poll::Pending => $otherwise,
        }
    };
}

use ready_or;
