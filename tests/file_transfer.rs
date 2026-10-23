use localpost::{
    Result,
    file::{FileReader, FileWriter},
    protocol::FileInfo,
    receiver::FileReceiver,
    sender::FileSender,
};
use std::{
    io::{self, Read, Seek, SeekFrom, Write},
    net::UdpSocket,
    thread,
};

#[derive(Default)]
struct MockFile {
    offset: u64,
}

impl Seek for MockFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.offset = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::Current(offset) => self.offset.saturating_add_signed(offset),
            SeekFrom::End(_) => unimplemented!(),
        };
        Ok(self.offset)
    }
}

impl Read for MockFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        for byte in buf.iter_mut() {
            *byte = self.offset as u8;
            self.offset += 1;
        }
        Ok(buf.len())
    }
}

impl Write for MockFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for byte in buf {
            assert_eq!(*byte, self.offset as u8);
            self.offset += 1;
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_file_transfer() -> Result<()> {
    let sender_socket = UdpSocket::bind("127.0.0.1:0")?;
    let receiver_socket = UdpSocket::bind("127.0.0.1:0")?;

    sender_socket.connect(receiver_socket.local_addr()?)?;
    receiver_socket.connect(sender_socket.local_addr()?)?;

    let frame_size = 1400;
    let file_size = 32 * 1024 * 1024;

    thread::spawn(move || {
        let reader = FileReader::new(MockFile::default(), frame_size, file_size);
        let mut sender = FileSender::new(reader, sender_socket);
        sender.init_send().unwrap();
    });

    let file_info = file_info(frame_size, file_size);
    let writer = FileWriter::new(MockFile::default(), frame_size);
    let mut receiver = FileReceiver::new(writer, receiver_socket, &file_info)?;

    receiver.flush_threshold = 8 * 1024;
    receiver.init_receive()?;
    Ok(())
}

fn file_info(frame_size: u16, file_size: u64) -> FileInfo {
    FileInfo {
        size: file_size,
        frame_size,
        tid: Default::default(),
        name: Default::default(),
        last_modified: Default::default(),
    }
}
