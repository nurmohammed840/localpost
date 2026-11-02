use super::*;
use crate::Result;

pub async fn open(addr: impl ToSocketAddrs) -> Result<(Endpoint, SystemInfo)> {
    let mut stream = TcpStream::connect(addr).await?;

    handshake::send_hello(&mut stream).await?;
    let hello = handshake::read_hello(&mut stream).await?;

    Ok((Endpoint::client(stream), hello.info))
}
