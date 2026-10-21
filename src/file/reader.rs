use std::{fs::File, io::*, path::Path, time::UNIX_EPOCH};

use crate::protocol::{FileInfo, TransferID};

#[derive(Debug)]
pub struct FileReader<T> {
    file: BufReader<T>,
    size: u64,
    frame_size: u16,
    offset: u64,
}

impl FileReader<File> {
    pub fn open(path: impl AsRef<Path>, frame_size: u16) -> Result<(Self, FileInfo)> {
        Self::open_with_capacity(path, frame_size, 8 * 1024 * 1024)
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
                tid: TransferID::default(),
                name,
                size,
                frame_size,
                last_modified,
            },
        ))
    }
}

impl<T> FileReader<T>
where
    T: Seek + Read,
{
    pub fn new(file: T, frame_size: u16, file_size: u64) -> Self {
        Self {
            file: BufReader::new(file),
            size: file_size,
            frame_size,
            offset: 0,
        }
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
        let mut chunk = Vec::new();
        let has_data = self.read_next(&mut chunk)?;
        Ok(has_data.then_some(chunk))
    }

    pub fn read_next(&mut self, buf: &mut Vec<u8>) -> Result<bool> {
        let remaining = self.size.saturating_sub(self.offset);
        if remaining == 0 {
            return Ok(false);
        }

        let buf_size = remaining.min(self.frame_size as u64) as usize;

        let old_len = buf.len();
        buf.resize(old_len + buf_size, 0);

        self.file.read_exact(&mut buf[old_len..])?;
        self.offset += buf_size as u64;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_file(buf: &[u8], frame_size: u16) -> Result<FileReader<Cursor<Vec<u8>>>> {
        let size = buf.len() as u64;
        let file = Cursor::new(buf.into());

        let mut file = FileReader {
            file: BufReader::with_capacity(4, file),
            size,
            frame_size,
            offset: 0,
        };

        file.seek_at(0)?;
        Ok(file)
    }

    #[test]
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

    #[test]
    fn test_empty_file() -> Result<()> {
        let mut file = tmp_file(b"", 4)?;
        assert_eq!(file.next_chunk()?, None);
        Ok(())
    }

    #[test]
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
