pub mod client;
// pub mod server;
pub mod frame;
pub mod handshake;
pub mod mux;

use crate::{Result, protocol::SystemInfo};
use nio::net::{TcpListener, TcpStream};
use std::net::{Ipv4Addr, ToSocketAddrs};

type Endpoint = mux::Endpoint<TcpStream>;

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

    pub async fn accept(&mut self) -> Result<(Endpoint, SystemInfo)> {
        let conn = self.tcp.accept().await?;
        let mut stream = conn.connect().await?;

        let hello = handshake::read_hello(&mut stream).await?;
        Ok((Endpoint::server(stream), hello.info))
    }
}

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
