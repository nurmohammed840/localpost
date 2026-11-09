use crate::{file::FileReader, protocol::DataFrame};
use anyhow::Result;
use std::io::{Read, Seek};

pub struct FrameReader<T> {
    next_frame_no: u32,
    reader: FileReader<T>,
}

impl<T> FrameReader<T>
where
    T: Seek + Read,
{
    pub fn new(reader: FileReader<T>) -> Self {
        Self {
            next_frame_no: 0,
            reader,
        }
    }

    pub fn next_frame(&mut self) -> Result<Option<DataFrame>> {
        let mut frame = Vec::with_capacity(4 + self.reader.frame_size() as usize);

        frame.extend_from_slice(&self.next_frame_no.to_le_bytes());
        let has_data = self.reader.read_next(&mut frame)?;

        if !has_data {
            return Ok(None);
        }

        let frame = DataFrame { buf: frame.into() };
        self.next_frame_no += 1;
        Ok(Some(frame))
    }
}
