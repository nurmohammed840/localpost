use anyhow::{Result, bail};
use lipi::{Decode, Encode};
use std::{env::consts as SYS, error::Error, fmt, ops::RangeBounds};

#[derive(Debug, Encode, Decode, Clone, Copy, Default)]
pub struct TransferID {
    #[key = 0]
    pub udp_port: u16,

    #[key = 1]
    pub id: Option<u32>,
}

impl TransferID {
    pub fn new(udp_port: u16) -> Self {
        Self { udp_port, id: None }
    }
}

#[derive(Debug, Encode, Decode, Clone)]
pub struct FileInfo {
    #[key = 0]
    pub tid: TransferID,
    #[key = 1]
    pub name: Vec<u8>,
    #[key = 2]
    pub size: u64,
    #[key = 3]
    pub frame_size: u16,
    #[key = 4]
    pub last_modified: u64,
}

impl FileInfo {
    pub fn num_of_frames(&self) -> u32 {
        self.size
            .div_ceil(self.frame_size as u64)
            .try_into()
            .unwrap()
    }
}

#[derive(Debug, Encode, Decode)]
pub struct Hello {
    #[key = 0]
    pub protocol: String,
    #[key = 1]
    pub version: u8,
    #[key = 2]
    pub info: SystemInfo,
}

impl Hello {
    pub fn message(port: u16) -> Self {
        Self {
            protocol: "localpost".into(),
            version: 10,
            info: SystemInfo::new(port),
        }
    }

    pub fn check_protocol_version(&self, target: impl RangeBounds<u8>) -> Result<()> {
        if self.protocol != "localpost" {
            bail!("Invalid protocol: {}", self.protocol);
        }
        if !target.contains(&self.version) {
            let version = format_protocol_version(self.version);
            bail!("Invalid protocol version: {version}");
        }
        Ok(())
    }
}

#[derive(Debug, Encode, Decode)]
pub struct SystemInfo {
    #[key = 0]
    pub os_name: String,

    #[key = 1]
    pub os_version: String,

    #[key = 2]
    pub os_type: String,

    #[key = 3]
    pub architecture: String,

    #[key = 4]
    pub hostname: String,

    #[key = 5]
    pub port: u16,
}

impl SystemInfo {
    pub fn new(port: u16) -> Self {
        use sysinfo::System;
        SystemInfo {
            os_name: System::name().unwrap_or_default(),
            os_version: System::os_version().unwrap_or_default(),
            os_type: SYS::OS.to_string(),
            architecture: SYS::ARCH.to_string(),
            hostname: System::host_name().unwrap_or_default(),
            port,
        }
    }
}

pub struct DataFrame {
    pub(crate) buf: Box<[u8]>,
}

impl fmt::Debug for DataFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataFrame").field("no", &self.no()).finish()
    }
}

pub trait DataFrameLike {
    /// length of bytes slice must be >= 4
    fn as_bytes(&self) -> &[u8];

    fn no(&self) -> u32 {
        u32::from_le_bytes(self.as_bytes()[0..4].try_into().unwrap())
    }

    fn data(&self) -> &[u8] {
        &self.as_bytes()[4..]
    }
}

impl<T: AsRef<[u8]>> DataFrameLike for T {
    fn as_bytes(&self) -> &[u8] {
        self.as_ref()
    }
}

impl AsRef<[u8]> for DataFrame {
    fn as_ref(&self) -> &[u8] {
        &self.buf
    }
}

impl DataFrame {
    pub fn new(no: u32, data: &[u8]) -> Self {
        let mut buf = Vec::with_capacity(4 + data.len());
        buf.extend_from_slice(&no.to_le_bytes());
        buf.extend_from_slice(data);
        Self {
            buf: buf.into_boxed_slice(),
        }
    }

    pub fn try_from(buf: Box<[u8]>) -> Result<Self> {
        let frame = Self { buf };
        if let Some(tid) = FrameType::check(&frame)? {
            return Err(FrameType::SignalReady(tid).into());
        }
        Ok(frame)
    }
}

#[derive(Clone, PartialEq)]
pub struct AckFrame {
    pub cumulative: u32,
    pub bitset: Box<[u8]>,
}

impl TryFrom<&[u8]> for AckFrame {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        let Some((cumulative, bitset)) = bytes.split_first_chunk::<4>() else {
            bail!("invalid ack frame length: {}", bytes.len());
        };

        Ok(Self {
            cumulative: u32::from_le_bytes(*cumulative),
            bitset: bitset.into(),
        })
    }
}

impl AckFrame {
    /// The length of contiguous sequence of frames received by the receiver in order.
    ///
    /// For example: If frames arrive in the order `2, 1, 4, 0, 5`
    /// Then cumulative length is `3`, Since frames `0, 1, 2` have been received.
    pub fn cumulative_len(&self) -> u32 {
        self.cumulative
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(4 + self.bitset.len());
        buf.extend_from_slice(&self.cumulative.to_le_bytes());
        buf.extend_from_slice(&self.bitset);
        buf
    }
}

#[repr(u8)]
#[derive(Debug, Clone)]
pub enum FrameType {
    Unknown(u8) = 0,

    /// Sent acknowledge on any Error
    Abort(String) = 1,

    /// Final Acknowledgment
    FinAck = 2,

    /// Error was received from the peer.
    /// The current send/receive operation should be stopped.
    /// We may sent acknowledge receipt of the peer's error.
    AckError = 3,

    /// Sent only by the client-side receiver to the server endpoint.
    /// client initiates an outbound connection, allowing the server to send
    /// data through the client's firewall.
    SignalReady(TransferID) = 4,
}

impl FrameType {
    pub fn is_fin_ack(&self) -> bool {
        matches!(self, Self::FinAck)
    }

    fn code(&self) -> u8 {
        match self {
            Self::Abort(_) => 1,
            Self::FinAck => 2,
            Self::AckError => 3,
            Self::SignalReady(_) => 4,
            Self::Unknown(code) => *code,
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let size = match self {
            FrameType::Abort(msg) => msg.as_bytes().len(),
            _ => 0,
        };

        let mut buf = Vec::with_capacity(4 + 1 + size);

        buf.extend_from_slice(&u32::MAX.to_le_bytes());
        buf.push(self.code());

        match self {
            FrameType::Abort(msg) => buf.extend_from_slice(msg.as_bytes()),
            FrameType::SignalReady(tid) => tid.encode(&mut buf)?,
            _ => {}
        };

        Ok(buf)
    }

    pub fn check(frame: impl DataFrameLike) -> Result<Option<TransferID>> {
        if frame.as_bytes().len() < 4 {
            bail!("small frame");
        }

        if frame.no() != u32::MAX {
            return Ok(None);
        }

        let mut data = frame.data();
        let code: u8 = Decode::decode(&mut data).map_err(anyhow::Error::from_boxed)?;

        let error = match code {
            1 => Self::Abort(str::from_utf8(data)?.to_owned()),
            2 => Self::FinAck,
            3 => Self::AckError,
            4 => {
                let tid = TransferID::decode(&mut data).map_err(anyhow::Error::from_boxed)?;
                return Ok(Some(tid));
            }
            _ => Self::Unknown(code),
        };

        Err(error.into())
    }
}

impl Error for FrameType {}
impl fmt::Display for FrameType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

// =============================================================

fn format_protocol_version(num: u8) -> String {
    let major = num / 10;
    let minor = num % 10;
    format!("{}.{}", major, minor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_frame() {
        let frame = DataFrame::try_from((0..10).collect()).unwrap();
        assert_eq!(frame.no(), u32::from_le_bytes([0, 1, 2, 3]));
        assert_eq!(frame.data(), [4, 5, 6, 7, 8, 9]);
    }

    #[test]
    fn error_frame_check() -> Result<()> {
        let err = FrameType::Unknown(42);
        let err = FrameType::check(err.to_bytes()?).err().unwrap();
        let err = err.downcast_ref::<FrameType>().unwrap();
        assert_eq!(err.code(), 42);
        Ok(())
    }
}
