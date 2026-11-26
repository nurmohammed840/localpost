mod ack_bitset;
mod status;

pub use ack_bitset::AckBitSet;

use crate::{
    file::{AsyncFileWriter, FileWriter},
    protocol::{AckFrame, DataFrame, DataFrameLike, FileInfo, FrameType},
    socket::AsyncDatagramSocket as DatagramSocket,
};
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
    bitset: AckBitSet,

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
            bitset: AckBitSet::new(file_info.num_of_frames()),
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

    pub async fn send_fin_frame(secket: S, frame: &[u8]) -> Result<()> {
        let _ = secket.send(frame).await;

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
            Ok(_) => FrameType::FinAck,
            Err(ref err) => match err.is::<FrameType>() {
                true => FrameType::AckError,
                false => FrameType::Abort(err.to_string()),
            },
        };

        nio::spawn_local(async move { Self::send_fin_frame(socket, &frame.to_bytes()?).await });

        if result.is_ok() {
            self.writer.finish().await?;
        }

        result
    }

    pub async fn _init_receive(&mut self, status: Arc<RecvStatus>) -> Result<()> {
        loop {
            let packets = self.recv_packets().await?;

            status.inc_total_count(packets.len() as u32);

            // This loop update bitset before sending acknowledgement
            for packet in packets {
                let frame = DataFrame::try_from(packet)?;
                let is_already_set = self.bitset.set(frame.no())?;

                if is_already_set {
                    status.inc_duplicate_count();
                    continue;
                }

                self.writer.write(frame).await?;
            }

            let ack = self.bitset.ack_frame();
            status.ack_counter.increase(&ack);
            self.send_ack(ack).await?;

            if self.bitset.is_full() {
                return Ok(());
            }
        }
    }
}
