use std::io::{Seek, Write};

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

    pub fn recv_frame(&mut self) -> Result<DataFrame> {
        let mut buf = vec![0; self.frame_size.into()];
        let len = self.socket.recv(&mut buf)?;
        buf.truncate(len);
        DataFrame::try_from(buf)
    }

    pub fn send_ack(&mut self, frame: &DataFrame) -> Result<()> {
        self.socket.send(frame.ack().as_bytes())?;
        Ok(())
    }

    pub fn init_receive(&mut self) -> Result<Option<ErrorFrame>> {
        loop {
            let frame = self.recv_frame()?;

            if let Some(error) = ErrorFrame::parse_from(&frame)? {
                return Ok(Some(error));
            }

            self.send_ack(&frame)?;

            let is_already_set = self.bitset.set(frame.no());
            if is_already_set {
                continue;
            }

            self.writer.write(frame);

            if self.bitset.is_full() {
                self.writer.flush()?;
                return Ok(None);
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
