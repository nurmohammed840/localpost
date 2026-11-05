use crate::{
    endpoint::frame::{FrameDecoder, FrameEncoder},
    protocol::Hello,
};
use anyhow::Result;
use futures::{AsyncRead, AsyncWrite};

pub async fn send_hello<S>(stream: &mut S) -> Result<()>
where
    S: AsyncWrite + Unpin,
{
    FrameEncoder::new(stream).encode(Hello::message()).await
}

pub async fn read_hello<S>(stream: &mut S) -> Result<Hello>
where
    S: AsyncRead + Unpin,
{
    let hello: Hello = FrameDecoder::new(stream).decode().await?;
    hello.check_protocol_version(10..=19)?;
    Ok(hello)
}
