use std::io::{Read, Seek};

use crate::{Result, file::FileReader, protocol::DataFrame};

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

    pub fn next_data_frame(&mut self) -> Result<Option<DataFrame>> {
        let mut frame = Vec::from(self.next_frame_no.to_le_bytes());

        let has_data = self.reader.read_next(&mut frame)?;
        if !has_data {
            return Ok(None);
        }

        let frame = DataFrame::from(frame);
        self.next_frame_no += 1;
        Ok(Some(frame))
    }
}
