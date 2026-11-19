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
        // the borrow ends inside each poll, never across an await
        poll_fn(|cx| self.inbound_rx.borrow_mut().poll_next_unpin(cx))
            .await
            .transpose()
            .map(|maybe| maybe.map(Stream::new))
    }

    pub async fn close(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        if self.close_tx.unbounded_send(tx).is_err() {
            return Ok(()); // driver already gone
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
    let mut closing = false;
    let mut close_ack: Option<CloseReply> = None;
    let mut pending_open: Option<OpenReply> = None;

    poll_fn(|cx| {
        // 1. Close if Endpoint droped
        if !closing {
            if let Poll::Ready(ack) = close_rx.poll_next_unpin(cx) {
                close_ack = ack;
                closing = true;
            }
        }

        // 2. serve outbound opens
        while !closing {
            if pending_open.is_none() {
                match open_rx.poll_next_unpin(cx) {
                    Poll::Pending => break,
                    Poll::Ready(None) => {
                        closing = true;
                        break;
                    }
                    Poll::Ready(Some(tx)) => pending_open = Some(tx),
                }
            }

            if pending_open.as_ref().unwrap().is_canceled() {
                pending_open = None;
                continue;
            }

            match conn.poll_new_outbound(cx) {
                Poll::Pending => break, // keep the slot, retry on wake
                Poll::Ready(res) => _ = pending_open.take().unwrap().send(res),
            }
        }

        // 3. closing: driver ends
        if closing {
            let res = ready!(conn.poll_close(cx));
            if let Some(tx) = close_ack.take() {
                let _ = tx.send(res);
            }
            return Poll::Ready(());
        }

        // 4. ALWAYS poll inbound after step 2. This drives all I/O.
        while let Some(item) = ready!(conn.poll_next_inbound(cx)) {
            if inbound_tx.unbounded_send(item).is_err() {
                // Endpoint dropped: re-poll right away so step 1 starts the graceful close.
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        }

        Poll::Ready(())
    })
    .await;
}
