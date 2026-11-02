// pub mod client;
// pub mod server;
pub mod mux;
pub mod handshake;
pub mod frame;

use crate::{Result, endpoint::mux::Endpoint};
use nio::net::{TcpListener, TcpStream};
use std::net::{Ipv4Addr, ToSocketAddrs};

pub struct Listener {
    tcp: TcpListener,
}

impl Listener {
    pub async fn new(port: u16) -> Result<Self> {
        match local_ip_address::local_ip() {
            Ok(local_ip) => Self::listen((local_ip, port)).await,
            Err(_) => Self::listen((Ipv4Addr::UNSPECIFIED, port)).await,
        }
    }

    pub async fn listen(addr: impl ToSocketAddrs) -> Result<Self> {
        Ok(Listener {
            tcp: TcpListener::bind(addr).await?,
        })
    }

    pub async fn accept(&mut self) -> Result<Endpoint<TcpStream>> {
        // SystemInfo
        let conn = self.tcp.accept().await?;
        let tcp_stream = conn.connect().await?;

        // let mut conn = Conn::new(stream, addr)?;
        // let info = conn.read_hello()?;
        // Ok((info, server::Endpoint { conn }))
        Ok(Endpoint::server(tcp_stream))
    }
}

// pub struct Conn {
//     local_addr: SocketAddr,
//     peer_addr: SocketAddr,
//     reader: BufReader<TcpStream>,
//     writer: BufWriter<TcpStream>,

//     len_buf: [u8; 4],
// }

// impl Conn {
//     fn new(stream: TcpStream, peer_addr: SocketAddr) -> io::Result<Self> {
//         let local_addr = stream.local_addr()?;

//         let reader = BufReader::new(stream.try_clone()?);
//         let writer = BufWriter::new(stream);

//         Ok(Conn {
//             local_addr,
//             peer_addr,
//             reader,
//             writer,
//             len_buf: [0; 4],
//         })
//     }

//     pub fn open(addr: impl ToSocketAddrs) -> Result<(SystemInfo, Self)> {
//         let stream = TcpStream::connect(addr)?;
//         let addr = stream.peer_addr()?;

//         let mut conn = Conn::new(stream, addr)?;
//         conn.send(Hello::message())?;
//         let info = conn.read_hello()?;

//         Ok((info, conn))
//     }

//     pub fn read(&mut self) -> Result<Message> {
//         let buf = self.read_frame()?;
//         Message::decode(&mut buf.as_ref())
//     }

//     pub fn send(&mut self, msg: impl Into<Message>) -> Result<()> {
//         self.send_frame(&msg.into().to_bytes()?)
//     }

//     pub fn next_req(&mut self) -> Result<FileInfo> {
//         match self.read()? {
//             Message::Write(info) => Ok(info),
//             msg => Err(format!("expected File Info; found: {msg}").into()),
//         }
//     }
// }

// impl Conn {
//     fn create_udp_socket(&mut self) -> io::Result<UdpSocket> {
//         let socket = UdpSocket::bind((self.local_addr.ip(), 0))?;
//         socket.set_read_timeout(Some(Duration::from_secs(7)))?;
//         Ok(socket)
//     }

//     pub fn send_file_to_server(&mut self, path: impl AsRef<Path>, frame_size: u16) -> Result<()> {
//         let (reader, mut file_info) = FileReader::open(path, frame_size - 4)?;
//         let file_id = file_info.tid.file_id;

//         let socket = self.create_udp_socket()?;
//         file_info.tid.udp_port = socket.local_addr()?.port();
//         self.send(file_info)?;

//         let tid = match self.read()? {
//             Message::TransferID(tid) => tid,
//             msg => return Err(format!("expected TID; found: {msg}").into()),
//         };

//         assert_eq!(tid.file_id, file_id);
//         socket.connect((self.peer_addr.ip(), tid.udp_port))?;

//         thread::spawn(move || -> Result<()> {
//             let mut sender = FileSender::new(reader, socket);
//             sender.init_send()?;
//             Ok(())
//         });

//         Ok(())
//     }
// }
