use super::*;
use crate::{Result, file::FileReader, protocol::TransferID, sender::FileSender};
use std::path::PathBuf;

pub async fn open(addr: impl ToSocketAddrs) -> Result<(Endpoint, SystemInfo)> {
    let mut stream = TcpStream::connect(addr).await?;

    handshake::send_hello(&mut stream).await?;
    let hello = handshake::read_hello(&mut stream).await?;

    Ok((Endpoint::client(stream), hello.info))
}

pub struct Client {
    local_addr: SocketAddr,
    peer_addr: SocketAddr,
    endpoint: Endpoint,
}

impl Client {
    pub async fn send_file(&mut self, path: PathBuf, frame_size: u16) -> Result<()> {
        let mut stream = self.endpoint.create_stream().await?;
        let (reader, mut file_info) =
            nio::spawn_blocking(move || FileReader::open(path, frame_size)).await??;

        let sender_socket = create_udp_socket(self.local_addr)?;
        file_info.tid.udp_port = sender_socket.local_addr()?.port();
        stream.send(file_info).await?;

        let tid: TransferID = stream.recv().await?;
        let receiver_socket_addr = (self.peer_addr.ip(), tid.udp_port);
        sender_socket.connect(receiver_socket_addr)?;

        nio::spawn_blocking(|| -> Result<()> {
            let mut sender = FileSender::new(reader, sender_socket);
            sender.init_send()?;
            Ok(())
        });

        Ok(())
    }
}
