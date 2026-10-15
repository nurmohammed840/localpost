use crate::{Result, file::FileReader, protocol::DataFrame};

pub struct FrameReader {
    next_frame_no: u32,
    reader: FileReader,
}

impl FrameReader {
    pub fn new(reader: FileReader) -> Self {
        Self {
            next_frame_no: 0,
            reader,
        }
    }

    pub fn next_data_frame(&mut self) -> Result<DataFrame> {
        let mut frame = Vec::from(self.next_frame_no.to_le_bytes());
        self.reader.read_next(&mut frame)?;

        let frame = DataFrame::from(frame);
        self.next_frame_no += 1;
        Ok(frame)
    }
}
