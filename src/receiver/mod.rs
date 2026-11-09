mod ack_policy;
mod status;

use crate::{
    file::{AsyncFileWriter, FileBitSet, FileWriter},
    protocol::{AckFrame, DataFrame, DataFrameLike, ErrorCode, ErrorFrame, FileInfo},
    socket::AsyncDatagramSocket as DatagramSocket,
};
use ack_policy::AckPolicy;
use anyhow::{Context, Result};
use std::{
    io::{Seek, Write},
    sync::Arc,
    time::{Duration, Instant},
};

pub use status::RecvStatus;

pub struct FileReceiver<S> {
    socket: S,
    writer: AsyncFileWriter,
    bitset: FileBitSet,
    ack_policy: AckPolicy,

    frame_size: u16,
    pub flush_threshold: usize,
}

impl<S> FileReceiver<S>
where
    S: DatagramSocket + 'static,
{
    pub fn new<T>(writer: FileWriter<T>, socket: S, file_info: &FileInfo) -> Self
    where
        T: Write + Seek + Send + 'static,
    {
        Self {
            writer: AsyncFileWriter::new(writer, 32 * 1024 * 1024),
            socket,
            ack_policy: AckPolicy::default(),
            bitset: FileBitSet::new(file_info.num_of_frames()),
            frame_size: file_info.frame_size,
            flush_threshold: 32 * 1024 * 1024,
        }
    }

    pub async fn recv_packets(&mut self) -> Result<Vec<Box<[u8]>>> {
        let mut packets = Vec::new();

        self.socket
            .recv_many(&mut packets, self.frame_size, u8::MAX)
            .await
            .context("failed to receive packets")?;

        Ok(packets)
    }

    async fn send_ack(&self, ack: AckFrame) -> Result<()> {
        self.socket
            .send(&ack.to_bytes())
            .await
            .or_else(|err| self.bitset.is_full().ok_or(err))
            .context("failed to send acknowledgement")
    }

    async fn maybe_send_regular_ack(&mut self, frame: &DataFrame) -> Result<()> {
        let cumulative = self.bitset.filled_upto();
        if let Some(ack) = self.ack_policy.on_frame_received(cumulative, frame.no()) {
            self.send_ack(ack).await?;
        }
        Ok(())
    }

    async fn send_cumulative_ack_if_no_regular_ack_sent(&mut self) -> Result<()> {
        let cumulative = self.bitset.filled_upto();
        if let Some(ack) = self.ack_policy.send_ack_if_not_sent(cumulative) {
            self.send_ack(ack).await?;
        }
        Ok(())
    }

    pub async fn send_fin_frame(secket: S, frame: &[u8]) -> Result<()> {
        let _ = secket.send(frame);

        let mut buf = [0; 4];
        let deadline = Instant::now() + Duration::from_secs(3);

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break Ok(());
            }
            secket.set_read_timeout(Some(remaining))?;
            secket.recv(&mut buf).await?;
            secket.send(frame).await?;
        }
    }

    pub async fn init_receive(mut self, status: Arc<RecvStatus>) -> Result<()> {
        let result = self._init_receive(status).await;
        let socket = self.socket;

        let frame = match result {
            Ok(_) => ErrorFrame::from(ErrorCode::FinAck),
            Err(ref err) => match err.is::<ErrorFrame>() {
                true => ErrorFrame::from(ErrorCode::AckError),
                false => ErrorFrame::new(ErrorCode::Abort, err.to_string()),
            },
        };

        nio::spawn_local(async move { Self::send_fin_frame(socket, &frame.to_bytes()).await });

        if result.is_ok() {
            self.writer.finish().await?;
        }

        result
    }

    pub async fn _init_receive(&mut self, status: Arc<RecvStatus>) -> Result<()> {
        loop {
            let packets = self.recv_packets().await?;
            let mut frames = Vec::with_capacity(packets.len());

            // This loop update bitset before sending acknowledgement
            for packet in packets {
                let frame = DataFrame::try_from(packet)?;
                frames.push((self.bitset.set(frame.no())?, frame));
            }

            status.inc_total_recv_count(frames.len() as u32);

            for (is_already_set, frame) in frames {
                self.maybe_send_regular_ack(&frame).await?;

                if is_already_set {
                    continue;
                }

                self.writer.write(frame).await?;
                status.inc_recv_count();
            }

            self.send_cumulative_ack_if_no_regular_ack_sent().await?;

            if self.bitset.is_full() {
                return Ok(());
            }
        }
    }
}
