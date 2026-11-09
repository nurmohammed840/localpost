use anyhow::Result;
use localpost::{
    file::{FileReader, FileWriter},
    protocol::FileInfo,
    receiver::{FileReceiver, RecvStatus},
    sender::FileSender,
    socket::async_udp,
};
use std::{
    io::{self, Read, Seek, SeekFrom, Write},
    net::UdpSocket,
    time::Instant,
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
    block_on(async || {
        let sender_socket = UdpSocket::bind("127.0.0.1:0")?;
        let receiver_socket = UdpSocket::bind("127.0.0.1:0")?;

        sender_socket.connect(receiver_socket.local_addr()?)?;
        receiver_socket.connect(sender_socket.local_addr()?)?;

        let receiver_socket = async_udp::UdpSocket::try_from(receiver_socket)?;

        let frame_size = 1400;
        let file_size = 32 * 1024 * 1024;

        let task = nio::spawn_blocking(move || {
            let reader = FileReader::new(MockFile::default(), frame_size, file_size);
            FileSender::new(reader, sender_socket).init_send()
        });

        let file_info = file_info(frame_size, file_size);
        let writer = FileWriter::new(MockFile::default(), frame_size);
        let mut receiver = FileReceiver::new(writer, receiver_socket, &file_info);

        let time = Instant::now();

        let status = RecvStatus::new();
        receiver.flush_threshold = 8 * 1024;
        receiver.init_receive(status.clone()).await?;

        println!("time.elapsed(): {:#?}", time.elapsed());

        assert_eq!(status.recv_count(), file_info.num_of_frames());
        task.await??;
        Ok(())
    })
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

pub fn block_on<F, Fut>(fut: F) -> Fut::Output
where
    F: FnOnce() -> Fut + Send,
    Fut: Future,
    Fut::Output: Send,
{
    ::nio::RuntimeBuilder::new()
        .worker_threads(1)
        .rt()
        .unwrap()
        .block_on(fut)
}
