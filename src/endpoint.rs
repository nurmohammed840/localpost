use crate::file::utils::write_all_vectored;
use crate::protocol::{FileInfo, Hello, Request, SystemInfo};
use crate::{Result, protocol::Message};

use lipi::{Decode, Encode};
use std::io::{self, *};
use std::net::*;
use std::thread;

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
    _addr: SocketAddr,
    reader: BufReader<TcpStream>,
    writer: BufWriter<TcpStream>,

    len_buf: [u8; 4],
}

impl Conn {
    fn new(stream: TcpStream, addr: SocketAddr) -> io::Result<Self> {
        let reader = BufReader::new(stream.try_clone()?);
        let writer = BufWriter::new(stream);

        Ok(Conn {
            _addr: addr,
            reader,
            writer,
            len_buf: [0; 4],
        })
    }

    pub fn open(addr: impl ToSocketAddrs) -> Result<(SystemInfo, Self)> {
        let stream = TcpStream::connect(addr)?;
        let addr = stream.peer_addr()?;

        let mut conn = Conn::new(stream, addr)?;

        conn.send_hello()?;
        let info = conn.read_hello()?;

        Ok((info, conn))
    }

    pub fn req_send_file(&mut self, info: FileInfo) -> Result<()> {
        self.send(Message::Write(info))
    }

    pub fn send_hello(&mut self) -> Result<()> {
        self.send(Message::Handshake(Hello::message()))
    }

    pub fn read_hello(&mut self) -> Result<SystemInfo> {
        let hello = match self.read()? {
            Message::Handshake(hello) => hello,
            _ => return Err("Expected Hello".into()),
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

    pub fn send(&mut self, msg: Message) -> Result<()> {
        self.send_frame(&msg.to_bytes()?)
    }

    pub fn next_req(&mut self) -> Result<Request> {
        let req = match self.read()? {
            Message::Handshake(_) => return Err("...".into()),
            Message::Write(file_info) => Request::Write(file_info),
            _ => todo!(),
        };
        Ok(req)
    }
}

pub fn run_endpoint(port: u16) -> Result<()> {
    let mut server = EndPoint::new(port)?;

    let mut accept_conn = || -> Result<()> {
        let (info, mut conn) = server.accept()?;

        println!("New Connection: {info:#?}");
        conn.send_hello()?;

        thread::spawn(move || {
            while let Ok(req) = conn.next_req() {
                match req {
                    Request::Write(info) => {
                        println!("write req: {info:#?}");
                        
                        thread::spawn(|| {});
                    }
                }
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
    use crate::file::reader::FileReader;
    use std::thread;

    const PORT: u16 = 54231;

    #[test]
    fn test_endpoint() -> Result<()> {
        let mut server = EndPoint::new(PORT)?;
        let addr = server.tcp.local_addr()?;

        thread::scope(|c| {
            c.spawn(|| {
                let (user, mut conn) = server.accept().unwrap();
                conn.send_hello().unwrap();
                println!("Server: {:#?}; {user:#?}", conn._addr);
            });

            c.spawn(|| {
                let (user, mut conn) = Conn::open(addr).unwrap();
                println!("User: {:#?}; {user:#?}", conn._addr);

                let (_file_reader, info) = FileReader::open("./README.md", 1024).unwrap();
                conn.req_send_file(info).unwrap();
            });
        });

        Ok(())
    }
}
