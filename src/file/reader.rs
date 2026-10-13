use std::{fs::File, io::*, path::Path, time::UNIX_EPOCH};

use crate::protocol::FileInfo;

#[derive(Debug)]
pub struct FileReader {
    file: BufReader<File>,
    size: u64,
    frame_size: u16,
    offset: u64,
}

impl FileReader {
    pub fn open(path: impl AsRef<Path>, frame_size: u16) -> Result<(Self, FileInfo)> {
        Self::open_with_capacity(path, frame_size, 8 * 1024)
    }

    pub fn open_with_capacity(
        path: impl AsRef<Path>,
        frame_size: u16,
        capacity: usize,
    ) -> Result<(Self, FileInfo)> {
        let name = path
            .as_ref()
            .file_name()
            .unwrap()
            .as_encoded_bytes()
            .to_vec();

        let file = File::options().read(true).open(path)?;
        let metadata = file.metadata()?;
        let size = metadata.len();

        let last_modified = metadata
            .modified()?
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok((
            FileReader {
                file: BufReader::with_capacity(capacity, file),
                size,
                frame_size,
                offset: 0,
            },
            FileInfo {
                id: fastrand::u64(..),
                name,
                size,
                frame_size,
                last_modified,
            },
        ))
    }

    pub fn num_of_frames(&self) -> u64 {
        self.size.div_ceil(self.frame_size as u64)
    }

    pub fn pos(&self) -> u64 {
        self.offset / self.frame_size as u64
    }

    pub fn seek_at(&mut self, no: u64) -> Result<()> {
        let offset = no * self.frame_size as u64;
        self.offset = self.file.seek(SeekFrom::Start(offset))?;
        Ok(())
    }

    pub fn seek_relative(&mut self, no: i64) -> Result<()> {
        self.file.seek_relative(no * self.frame_size as i64)?;
        self.offset = self.file.stream_position()?;
        Ok(())
    }

    pub fn next_chunk(&mut self) -> Result<Option<Vec<u8>>> {
        let remaining = self.size.saturating_sub(self.offset);
        if remaining == 0 {
            return Ok(None);
        }

        let buf_size = remaining.min(self.frame_size as u64) as usize;
        let mut buf = vec![0u8; buf_size];

        self.file.read_exact(&mut buf)?;
        self.offset += buf_size as u64;
        Ok(Some(buf))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{env, io::Write};

    fn tmp_file(buf: &[u8], frame_size: u16) -> Result<FileReader> {
        let path = env::temp_dir().join("file_reader_test.tmp");

        let mut file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;

        file.write_all(buf)?;

        let metadata = file.metadata()?;
        let mut file = FileReader {
            file: BufReader::with_capacity(4, file),
            size: metadata.len(),
            frame_size,
            offset: 0,
        };

        file.seek_at(0)?;
        Ok(file)
    }
    #[test]
    fn test_all() -> Result<()> {
        test_next()?;
        test_empty_file()?;
        test_frame_boundary()?;
        Ok(())
    }

    fn test_next() -> Result<()> {
        let mut file = tmp_file(b"0123456789", 4)?;

        assert_eq!(file.size, 10);
        assert_eq!(file.num_of_frames(), 3);

        assert_eq!(file.next_chunk()?, Some(b"0123".into()));
        assert_eq!(file.next_chunk()?, Some(b"4567".into()));
        assert_eq!(file.next_chunk()?, Some(b"89".into()));
        assert_eq!(file.next_chunk()?, None);

        file.seek_at(1)?;
        assert_eq!(file.next_chunk()?, Some(b"4567".into()));

        file.seek_at(1)?;
        file.seek_relative(1)?; // skip next
        assert_eq!(file.next_chunk()?, Some(b"89".into()));

        Ok(())
    }

    fn test_empty_file() -> Result<()> {
        let mut file = tmp_file(b"", 4)?;
        assert_eq!(file.next_chunk()?, None);
        Ok(())
    }

    fn test_frame_boundary() -> Result<()> {
        let mut file = tmp_file(b"0123456789012345", 4)?;

        assert_eq!(file.num_of_frames(), 4);

        file.seek_relative(1)?;
        file.seek_relative(1)?;

        assert_eq!(file.next_chunk()?, Some(b"8901".into()));
        assert_eq!(file.next_chunk()?, Some(b"2345".into()));
        assert_eq!(file.next_chunk()?, None);

        Ok(())
    }
}
