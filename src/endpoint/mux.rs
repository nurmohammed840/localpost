use std::{
    cell::RefCell,
    collections::VecDeque,
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

enum Command {
    Open(oneshot::Sender<Result<yamux::Stream>>),
    Close(oneshot::Sender<Result<()>>),
}

pub struct Endpoint {
    cmd_tx: mpsc::UnboundedSender<Command>,
    inbound_rx: RefCell<mpsc::UnboundedReceiver<Result<yamux::Stream>>>,
}

impl Endpoint {
    fn new<S>(io: S, mode: Mode) -> Self
    where
        S: AsyncRead + AsyncWrite + Unpin + 'static,
    {
        let (cmd_tx, cmd_rx) = mpsc::unbounded();
        let (inbound_tx, inbound_rx) = mpsc::unbounded();
        let conn = Connection::new(io, Config::default(), mode);

        nio::spawn_local(drive(conn, cmd_rx, inbound_tx));

        Self {
            cmd_tx,
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
        self.cmd_tx
            .unbounded_send(Command::Open(tx))
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
        if self.cmd_tx.unbounded_send(Command::Close(tx)).is_err() {
            return Ok(()); // driver gone
        }
        rx.await.unwrap_or(Ok(()))
    }
}

/// [yamux] does not support concurrent calls to poll_new_outbound and poll_next_inbound from different tasks.
/// 
/// See: https://github.com/libp2p/rust-yamux/issues/237
async fn drive<S>(
    mut conn: Connection<S>,
    mut cmd_rx: mpsc::UnboundedReceiver<Command>,
    inbound_tx: mpsc::UnboundedSender<Result<yamux::Stream>>,
) where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut opens: VecDeque<oneshot::Sender<Result<yamux::Stream>>> = VecDeque::new();
    let mut closing = false;
    let mut close_ack: Option<oneshot::Sender<Result<()>>> = None;

    poll_fn(|cx| {
        // 1. collect commands (all handles dropped == close)
        while !closing {
            match cmd_rx.poll_next_unpin(cx) {
                Poll::Ready(Some(Command::Open(tx))) => opens.push_back(tx),
                Poll::Ready(Some(Command::Close(tx))) => {
                    close_ack = Some(tx);
                    closing = true;
                }
                Poll::Ready(None) => closing = true,
                Poll::Pending => break,
            }
        }

        // 2. closing: flush and shut down, then the driver ends
        if closing {
            let res = ready!(conn.poll_close(cx));
            if let Some(tx) = close_ack.take() {
                let _ = tx.send(res);
            }
            return Poll::Ready(());
        }

        // 3. serve outbound opens
        while !opens.is_empty() {
            match conn.poll_new_outbound(cx) {
                Poll::Ready(res) => {
                    let _ = opens.pop_front().unwrap().send(res);
                }
                Poll::Pending => break,
            }
        }

        // 4. ALWAYS poll inbound after step 3. This drives all I/O.
        loop {
            match conn.poll_next_inbound(cx) {
                Poll::Ready(Some(Ok(stream))) => {
                    let _ = inbound_tx.unbounded_send(Ok(stream));
                }
                Poll::Ready(Some(Err(e))) => {
                    let _ = inbound_tx.unbounded_send(Err(e));
                    return Poll::Ready(());
                }
                Poll::Ready(None) => return Poll::Ready(()),
                Poll::Pending => return Poll::Pending,
            }
        }
    })
    .await;
}
