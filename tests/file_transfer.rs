use localpost::{
    Result,
    file::{FileReader, FileWriter},
    protocol::{FileInfo, TransferID},
    receiver::FileReceiver,
    sender::FileSender,
};
use std::{
    io::{self, ErrorKind, Read, Seek, SeekFrom, Write},
    net::UdpSocket,
    thread,
};

struct MockFile {
    offset: u64,
}

impl MockFile {
    fn new() -> Self {
        Self { offset: 0 }
    }
}

impl Seek for MockFile {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.offset = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::Current(offset) => self.offset.saturating_add_signed(offset),
            SeekFrom::End(_) => {
                return Err(io::Error::new(
                    ErrorKind::Unsupported,
                    "SeekFrom::End is not supported",
                ));
            }
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
            let expected = self.offset as u8;
            if *byte != expected {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("expected: {expected}; found: {byte}"),
                ));
            }
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
        let file = MockFile::new();
        let mut sender =
            FileSender::new(FileReader::new(file, frame_size, file_size), sender_socket);

        sender.init_send().unwrap();
    });

    let file = MockFile::new();
    let file_info = file_info(frame_size, file_size);

    let mut receiver = FileReceiver::new(
        FileWriter::new(file, frame_size),
        receiver_socket,
        &file_info,
    )?;

    receiver.flush_threshold = 8 * 1024;

    let proto_err = receiver.init_receive()?;
    assert!(proto_err.is_none());
    Ok(())
}

fn file_info(frame_size: u16, file_size: u64) -> FileInfo {
    let file_info = FileInfo {
        size: file_size,
        frame_size,
        tid: TransferID {
            udp_port: Default::default(),
            file_id: Default::default(),
        },
        name: Default::default(),
        last_modified: Default::default(),
    };
    file_info
}
