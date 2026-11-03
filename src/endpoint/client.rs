use super::*;
use crate::{
    Result, endpoint::mux::Stream, file::FileReader, protocol::TransferID, sender::FileSender,
};
use std::{path::PathBuf, thread};

pub async fn open(addr: impl ToSocketAddrs) -> Result<(Endpoint, SystemInfo)> {
    let mut stream = TcpStream::connect(addr).await?;

    handshake::send_hello(&mut stream).await?;
    let hello = handshake::read_hello(&mut stream).await?;

    Ok((Endpoint::client(stream), hello.info))
}

pub async fn send_file(
    path: PathBuf,
    frame_size: u16,
    peer_addr: SocketAddr,
    local_addr: SocketAddr,
    mut stream: Stream,
) -> Result<()> {
    let (reader, mut file_info) =
        nio::spawn_blocking(move || FileReader::open(path, frame_size)).await??;

    let socket = create_udp_socket(local_addr)?;
    file_info.tid.udp_port = socket.local_addr()?.port();

    stream.send(file_info).await?;

    let tid: TransferID = stream.recv().await?;
    let receiver_socket_addr = (peer_addr.ip(), tid.udp_port);

    socket.connect(receiver_socket_addr)?;

    thread::spawn(|| -> Result<()> {
        let mut sender = FileSender::new(reader, socket);
        sender.init_send()?;
        Ok(())
    });

    Ok(())
}
