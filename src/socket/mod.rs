pub mod async_udp;

use std::{io::Result, time::Duration};

#[allow(async_fn_in_trait)]
pub trait AsyncDatagramSocket {
    fn set_read_timeout(&self, dur: Option<Duration>) -> Result<()>;
    fn read_timeout(&self) -> Result<Option<Duration>>;

    async fn send(&self, buf: &[u8]) -> Result<()>;
    async fn recv(&self, buf: &mut [u8]) -> Result<usize>;

    async fn recv_many(&self, bufs: &mut Vec<Box<[u8]>>, size: u16, limit: u8) -> Result<()>;
}
