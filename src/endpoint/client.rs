use super::*;
use crate::Result;

pub struct Endpoint {
    pub conn: Conn,
}

impl Endpoint {
    pub fn open(addr: impl ToSocketAddrs) -> Result<(SystemInfo, Self)> {
        let stream = TcpStream::connect(addr)?;
        let addr = stream.peer_addr()?;

        let mut conn = Conn::new(stream, addr)?;
        conn.send(Hello::message())?;
        let info = conn.read_hello()?;

        Ok((info, Self { conn }))
    }
}
