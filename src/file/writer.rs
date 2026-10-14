use crate::{file::utils::write_all_vectored, protocol::DataFrame};
use std::{fs::File, io::*, path::Path};

struct Chunk {
    offset: u64,
    frame: DataFrame,
}

pub struct FileWriter<T: Write + Seek> {
    file: T,
    total_bytes: usize,
    chunks: Vec<Chunk>,
    frame_size: u16,
}

impl FileWriter<File> {
    pub fn open(path: impl AsRef<Path>, frame_size: u16) -> Result<Self> {
        let file = File::options()
            .truncate(false)
            .create(true)
            .write(true)
            .open(path)?;

        Ok(FileWriter::new(file, frame_size))
    }
}

impl<T: Write + Seek> FileWriter<T> {
    pub fn new(file: T, frame_size: u16) -> Self {
        Self {
            file,
            frame_size,
            total_bytes: 0,
            chunks: vec![],
        }
    }

    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    pub fn write(&mut self, frame: DataFrame) {
        assert!(frame.data().len() <= self.frame_size as usize);

        let chunk = Chunk {
            offset: frame.no() as u64 * self.frame_size as u64,
            frame,
        };

        let result = self
            .chunks
            .binary_search_by(|c| c.offset.cmp(&chunk.offset));

        match result {
            Ok(index) => self.chunks[index] = chunk,
            Err(index) => {
                self.total_bytes += chunk.frame.data().len();
                self.chunks.insert(index, chunk);
            }
        }
    }

    pub fn flush(&mut self) -> Result<()> {
        let mut chunks = self.chunks.iter().peekable();

        while let Some(buf) = chunks.next() {
            let start_offset = buf.offset;
            let start_data = buf.frame.data();
            let mut bufs = vec![IoSlice::new(start_data)];

            let mut next_offset = start_offset + start_data.len() as u64;
            while let Some(next) = chunks.next_if(|&next| next.offset == next_offset) {
                let data = next.frame.data();
                bufs.push(IoSlice::new(data));
                next_offset += data.len() as u64;
            }

            self.file.seek(SeekFrom::Start(start_offset))?;
            write_all_vectored(&mut self.file, &mut bufs)?;
        }

        self.chunks.clear();
        self.total_bytes = 0;

        Ok(())
    }
}

impl<T: Write + Seek> Drop for FileWriter<T> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn writer(frame_size: u16) -> FileWriter<Cursor<Vec<u8>>> {
        FileWriter::new(Cursor::new(Vec::new()), frame_size)
    }

    fn read(writer: &FileWriter<Cursor<Vec<u8>>>) -> &[u8] {
        writer.file.get_ref()
    }

    #[test]
    fn many_short_write() {
        let mut w = writer(2);
        w.write(DataFrame::new(2, &[2]));
        w.write(DataFrame::new(1, &[1]));
        w.write(DataFrame::new(0, &[0]));

        assert_eq!(w.total_bytes(), 3);
        w.flush().unwrap();
        assert_eq!(read(&w), &[0, 0, 1, 0, 2]);
    }

    #[test]
    fn write_contiguous() {
        let mut w = writer(2);

        w.write(DataFrame::new(0, &[0, 0]));
        w.write(DataFrame::new(1, &[1, 1]));
        w.write(DataFrame::new(2, &[2]));

        assert_eq!(w.total_bytes(), 5);
        w.flush().unwrap();

        assert_eq!(read(&w), &[0, 0, 1, 1, 2]);
        assert_eq!(w.total_bytes(), 0);
    }

    #[test]
    fn write_out_of_order() {
        let mut w = writer(2);

        w.write(DataFrame::new(2, &[2]));
        w.write(DataFrame::new(0, &[0, 0]));
        w.write(DataFrame::new(1, &[1, 1]));

        w.flush().unwrap();
        assert_eq!(read(&w), &[0, 0, 1, 1, 2]);
    }

    #[test]
    fn write_with_a_gap() {
        let mut w = writer(2);

        w.write(DataFrame::new(0, &[0, 0]));
        w.write(DataFrame::new(2, &[2, 2]));

        w.flush().unwrap();
        assert_eq!(read(&w), &[0, 0, 0, 0, 2, 2]);
    }

    #[test]
    fn write_partial() {
        let mut w = writer(4);

        w.write(DataFrame::new(0, &[1, 2]));
        w.write(DataFrame::new(1, &[3, 4]));

        w.flush().unwrap();
        assert_eq!(read(&w), &[1, 2, 0, 0, 3, 4]);
    }

    #[test]
    fn overwrites_existing() {
        let mut w = writer(2);

        w.write(DataFrame::new(0, &[1, 1]));
        w.write(DataFrame::new(0, &[2, 2]));

        assert_eq!(w.total_bytes(), 2);

        w.flush().unwrap();
        assert_eq!(read(&w), &[2, 2]);
    }

    #[test]
    fn can_write_after_flush() {
        let mut w = writer(2);

        w.write(DataFrame::new(0, &[1, 2]));
        w.flush().unwrap();

        w.write(DataFrame::new(2, &[3, 4]));
        w.flush().unwrap();

        assert_eq!(read(&w), &[1, 2, 0, 0, 3, 4]);
    }

    #[test]
    fn write_beyond_eof() {
        let mut w = writer(2);

        w.write(DataFrame::new(3, &[6]));
        w.flush().unwrap();

        assert_eq!(read(&w), &[0, 0, 0, 0, 0, 0, 6]);
    }
}
