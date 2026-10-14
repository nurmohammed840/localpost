use crate::{
    Result,
    file::{FileReader, FileWriter, utils::write_all_vectored},
    protocol::*,
};
use lipi::{Decode, Encode};
use std::{
    io::{self, *},
    net::*,
    path::Path,
    thread,
};

pub struct EndPoint {
    tcp: TcpListener,
}

impl EndPoint {
    pub fn new(port: u16) -> Result<Self> {
        match local_ip_address::local_ip() {
            Ok(local_ip) => Self::listen((local_ip, port)),
            Err(_) => Self::listen((Ipv4Addr::UNSPECIFIED, port)),
        }
    }

    pub fn listen(addr: impl ToSocketAddrs) -> Result<Self> {
        Ok(EndPoint {
            tcp: TcpListener::bind(addr)?,
        })
    }

    pub fn local_ip(&self) -> io::Result<IpAddr> {
        self.tcp.local_addr().map(|addr| addr.ip())
    }

    pub fn accept(&mut self) -> Result<(SystemInfo, Conn)> {
        let (stream, addr) = self.tcp.accept()?;
        let mut conn = Conn::new(stream, addr)?;
        let info = conn.read_hello()?;
        Ok((info, conn))
    }
}

pub struct Conn {
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    reader: BufReader<TcpStream>,
    writer: BufWriter<TcpStream>,

    len_buf: [u8; 4],
}

impl Conn {
    fn new(stream: TcpStream, peer_addr: SocketAddr) -> io::Result<Self> {
        let local_addr = stream.local_addr()?;

        let reader = BufReader::new(stream.try_clone()?);
        let writer = BufWriter::new(stream);

        Ok(Conn {
            local_addr,
            peer_addr,
            reader,
            writer,
            len_buf: [0; 4],
        })
    }

    pub fn open(addr: impl ToSocketAddrs) -> Result<(SystemInfo, Self)> {
        let stream = TcpStream::connect(addr)?;
        let addr = stream.peer_addr()?;

        let mut conn = Conn::new(stream, addr)?;
        conn.send(Hello::message())?;
        let info = conn.read_hello()?;

        Ok((info, conn))
    }

    pub fn read_hello(&mut self) -> Result<SystemInfo> {
        let hello = match self.read()? {
            Message::Handshake(hello) => hello,
            msg => return Err(format!("expected Handshake; found: {msg}").into()),
        };

        hello.check_protocol_version(10..=19)?;
        Ok(hello.info)
    }

    pub fn send_frame(&mut self, buf: &[u8]) -> Result<()> {
        let len = u32::try_from(buf.len())?;
        let raw_len = len.to_ne_bytes();

        write_all_vectored(
            &mut self.writer,
            &mut [IoSlice::new(&raw_len), IoSlice::new(buf)],
        )?;

        self.writer.flush()?;
        Ok(())
    }

    pub fn read_frame(&mut self) -> io::Result<Vec<u8>> {
        self.reader.read_exact(&mut self.len_buf)?;
        let len = u32::from_ne_bytes(self.len_buf) as usize;

        let mut buf = vec![0; len];
        self.reader.read_exact(&mut buf)?;
        Ok(buf)
    }

    pub fn read(&mut self) -> Result<Message> {
        let buf = self.read_frame()?;
        Message::decode(&mut buf.as_ref())
    }

    pub fn send(&mut self, msg: impl Into<Message>) -> Result<()> {
        self.send_frame(&msg.into().to_bytes()?)
    }

    pub fn next_req(&mut self) -> Result<FileInfo> {
        match self.read()? {
            Message::Write(info) => Ok(info),
            msg => Err(format!("expected File Info; found: {msg}").into()),
        }
    }
}

impl Conn {
    fn create_udp_socket(&mut self) -> io::Result<UdpSocket> {
        UdpSocket::bind((self.local_addr.ip(), 0))
    }

    pub fn send_file_to_server(&mut self, path: impl AsRef<Path>, frame_size: u16) -> Result<()> {
        let (_file, mut file_info) = FileReader::open(path, frame_size)?;
        let file_id = file_info.tid.file_id;

        let socket = self.create_udp_socket()?;
        file_info.tid.udp_port = socket.local_addr()?.port();
        self.send(file_info)?;

        let tid = match self.read()? {
            Message::TransferID(tid) => tid,
            msg => return Err(format!("expected TID; found: {msg}").into()),
        };

        assert_eq!(tid.file_id, file_id);
        socket.connect((self.peer_addr.ip(), tid.udp_port))?;

        Ok(())
    }

    fn create_transfer_socket(&mut self, file_info: FileInfo) -> Result<()> {
        let _file = FileWriter::open("file.txt", file_info.frame_size - 8)?;

        let tid = file_info.tid;
        let socket = self.create_udp_socket()?;

        socket.connect((self.peer_addr.ip(), tid.udp_port))?;
        self.send(TransferID {
            udp_port: socket.local_addr()?.port(),
            file_id: tid.file_id,
        })?;

        thread::spawn(move || {
            let mut buf = vec![0; file_info.frame_size.into()];
            socket.recv(&mut buf).unwrap();
        });

        Ok(())
    }
}

pub fn run_endpoint(port: u16) -> Result<()> {
    let mut server = EndPoint::new(port)?;

    let mut accept_conn = || -> Result<()> {
        let (info, mut conn) = server.accept()?;

        println!("New Connection: {info:#?}");
        conn.send(Hello::message())?;

        thread::spawn(move || {
            while let Ok(info) = conn.next_req() {
                println!("write req: {info:#?}");
                let _ = conn.create_transfer_socket(info);
            }
        });

        Ok(())
    };

    loop {
        let _ = accept_conn();
    }
}

#[cfg(test)]
mod tests {
    #[allow(warnings)]
    use super::*;
    use crate::file::FileReader;
    use std::thread;

    const PORT: u16 = 54231;

    #[test]
    fn test_endpoint() -> Result<()> {
        let mut server = EndPoint::new(PORT)?;
        let addr = server.tcp.local_addr()?;

        thread::scope(|c| {
            c.spawn(|| {
                let (user, mut conn) = server.accept().unwrap();
                conn.send(Hello::message()).unwrap();
                println!("Server: {:#?}; {user:#?}", conn.peer_addr);
            });

            c.spawn(|| {
                let (user, mut conn) = Conn::open(addr).unwrap();
                println!("User: {:#?}; {user:#?}", conn.peer_addr);

                let (_file_reader, info) = FileReader::open("./README.md", 1024).unwrap();
                conn.send(info).unwrap();
            });
        });

        Ok(())
    }
}
