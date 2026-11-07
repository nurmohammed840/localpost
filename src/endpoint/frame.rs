use std::io::IoSlice;

use anyhow::{Result, anyhow, bail};
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use lipi::{DecodeOwned, Encode};

const MAX_FRAME_SIZE: u32 = 2 * 1024 * 1024;

pub struct FrameDecoder<S> {
    max_frame_size: u32,
    stream: S,
}

impl<S> FrameDecoder<S>
where
    S: AsyncRead + Unpin,
{
    pub fn new(stream: S) -> Self {
        Self {
            max_frame_size: MAX_FRAME_SIZE,
            stream,
        }
    }

    pub async fn read_frame(&mut self) -> Result<Vec<u8>> {
        let mut len_buf = [0; 4];
        self.stream.read_exact(&mut len_buf).await?;
        let len = u32::from_le_bytes(len_buf);

        if len > self.max_frame_size {
            bail!("frame too large: {len} bytes");
        }

        let mut buf = vec![0; len as usize];
        self.stream.read_exact(&mut buf).await?;
        Ok(buf)
    }

    pub async fn decode<T>(&mut self) -> Result<T>
    where
        T: DecodeOwned,
    {
        let frame = self.read_frame().await?;
        T::decode(&mut frame.as_ref()).map_err(|e| anyhow!("failed to decode frame: {e}"))
    }
}

pub struct FrameEncoder<S> {
    stream: S,
}

impl<S> FrameEncoder<S>
where
    S: AsyncWrite + Unpin,
{
    pub fn new(stream: S) -> Self {
        Self { stream }
    }

    pub async fn write_frame(&mut self, data: &[u8]) -> Result<()> {
        let len = u32::try_from(data.len())?;

        self.stream
            .write_all_vectored(&mut [IoSlice::new(&len.to_le_bytes()), IoSlice::new(data)])
            .await?;

        self.stream.flush().await?;
        Ok(())
    }

    pub async fn encode<T: Encode>(&mut self, data: T) -> Result<()> {
        self.write_frame(&data.to_bytes()?).await
    }
}
