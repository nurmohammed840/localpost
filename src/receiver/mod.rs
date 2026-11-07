mod status;
use std::{
    io::{Seek, Write},
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

pub use crate::receiver::status::RecvStatus;
use crate::{
    file::{FileBitSet, FileWriter},
    protocol::DataFrame,
};
use crate::{
    protocol::{DataFrameLike, ErrorCode, ErrorFrame, FileInfo},
    socket::DatagramSocket,
};
use anyhow::{Context, Result};

pub struct FileReceiver<T, S>
where
    T: Write + Seek,
{
    writer: FileWriter<T>,
    socket: S,
    bitset: FileBitSet,

    frame_size: u16,
    pub flush_threshold: usize,
}

impl<T, S> FileReceiver<T, S>
where
    T: Write + Seek,
    S: DatagramSocket + Send + 'static,
{
    pub fn new(writer: FileWriter<T>, socket: S, file_info: &FileInfo) -> Self {
        Self {
            writer,
            socket,
            bitset: FileBitSet::new(file_info.num_of_frames()),
            frame_size: file_info.frame_size,
            flush_threshold: 32 * 1024 * 1024,
        }
    }

    pub fn recv_packets(&mut self) -> Result<Vec<Box<[u8]>>> {
        let mut packets = Vec::new();

        self.socket
            .recv_many(&mut packets, self.frame_size, u8::MAX)
            .context("failed to receive packets")?;

        Ok(packets)
    }

    pub fn send_ack(&mut self, frame: &DataFrame) -> Result<()> {
        let cumulative = self.bitset.filled_upto();
        self.socket
            .send(&frame.ack(cumulative).to_bytes())
            .context("failed to send acknowledgement")?;

        Ok(())
    }

    pub fn send_fin_frame(secket: S, frame: &[u8]) -> Result<()> {
        let _ = secket.send(frame);

        let mut buf = [0; 4];
        let deadline = Instant::now() + Duration::from_secs(3);

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break Ok(());
            }
            secket.set_read_timeout(Some(remaining))?;
            secket.recv(&mut buf)?;
            secket.send(frame)?;
        }
    }

    pub fn init_receive(mut self, status: Arc<RecvStatus>) -> Result<()> {
        let result = self._init_receive(status);
        let socket = self.socket;

        let frame = match result {
            Ok(_) => ErrorFrame::from(ErrorCode::FinAck),
            Err(ref err) => match err.is::<ErrorFrame>() {
                true => ErrorFrame::from(ErrorCode::AckError),
                false => ErrorFrame::new(ErrorCode::Abort, err.to_string()),
            },
        };

        thread::spawn(move || Self::send_fin_frame(socket, &frame.to_bytes()));
        result
    }

    pub fn _init_receive(&mut self, status: Arc<RecvStatus>) -> Result<()> {
        loop {
            let packets = self.recv_packets()?;
            let mut frames = Vec::with_capacity(packets.len());

            // This loop update bitset before sending acknowledgement
            for packet in packets {
                let frame = DataFrame::try_from(packet)?;
                frames.push((self.bitset.set(frame.no())?, frame));
            }

            status.inc_total_recv_count(frames.len() as u32);

            for (is_already_set, frame) in frames {
                self.send_ack(&frame)
                    .or_else(|err| self.bitset.is_full().ok_or(err))?;

                if is_already_set {
                    continue;
                }

                self.writer.write(frame);
                status.inc_recv_count();
            }

            if self.bitset.is_full() {
                self.writer.flush()?;
                return Ok(());
            }

            if self.writer.total_bytes() >= self.flush_threshold {
                self.writer.flush()?;
            }
        }
    }
}
