mod status;
use std::{
    io::{Seek, Write},
    sync::Arc,
};

pub use crate::receiver::status::RecvStatus;
use crate::{
    Result,
    protocol::{DataFrameLike, ErrorCode, ErrorFrame, FileInfo},
    socket::DatagramSocket,
};
use crate::{
    file::{FileBitSet, FileWriter},
    protocol::DataFrame,
};

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
    S: DatagramSocket,
{
    pub fn new(writer: FileWriter<T>, socket: S, file_info: &FileInfo) -> Result<Self> {
        Ok(Self {
            writer,
            socket,
            bitset: FileBitSet::new(file_info.num_of_frames().try_into()?),
            frame_size: file_info.frame_size,
            flush_threshold: 32 * 1024 * 1024,
        })
    }

    pub fn recv_packets(&mut self) -> Result<Vec<Box<[u8]>>> {
        let mut packets = Vec::new();

        self.socket
            .recv_many(&mut packets, self.frame_size, u8::MAX)?;

        Ok(packets)
    }

    pub fn send_ack(&mut self, frame: &DataFrame) -> Result<()> {
        let cumulative = self.bitset.filled_upto();
        self.socket.send(&frame.ack(cumulative).to_bytes())?;
        Ok(())
    }

    pub fn init_receive(&mut self, status: Arc<RecvStatus>) -> Result<()> {
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
                self.send_ack(&frame)?;

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

    pub fn send_error(&mut self, msg: impl Into<String>) -> Result<()> {
        let error_frame = ErrorFrame::new(ErrorCode::Other, msg);
        self.socket.send(&error_frame.to_bytes())?;
        Ok(())
    }
}
