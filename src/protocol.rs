use anyhow::{Result, anyhow, bail};
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
    pub fn message() -> Self {
        Self {
            protocol: "localpost".into(),
            version: 10,
            info: SystemInfo::get(),
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
}

impl SystemInfo {
    pub fn get() -> Self {
        use sysinfo::System;
        SystemInfo {
            os_name: System::name().unwrap_or_default(),
            os_version: System::os_version().unwrap_or_default(),
            os_type: SYS::OS.to_string(),
            architecture: SYS::ARCH.to_string(),
            hostname: System::host_name().unwrap_or_default(),
        }
    }
}

pub struct DataFrame {
    buf: Box<[u8]>,
}

impl fmt::Debug for DataFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DataFrame").field("no", &self.no()).finish()
    }
}

pub trait DataFrameLike {
    /// length of bytes slice must be >= 8
    fn as_bytes(&self) -> &[u8];

    fn ack(&self, cumulative: u32) -> AckFrame {
        AckFrame {
            current: Some(self.no()),
            cumulative,
        }
    }

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
        ErrorFrame::check(&frame)?;
        Ok(frame)
    }
}


#[derive(Debug, Clone, Copy)]
pub struct AckFrame {
    cumulative: u32,
    current: Option<u32>,
}

impl TryFrom<&[u8]> for AckFrame {
    type Error = anyhow::Error;

    fn try_from(bytes: &[u8]) -> Result<Self> {
        let len = bytes.len();
        if len != 4 && len != 8 {
            bail!("invalid ack frame length: {len} (expected 4 or 8)",);
        }

        Ok(Self {
            cumulative: u32::from_le_bytes(bytes[..4].try_into()?),
            current: if len == 8 {
                Some(u32::from_le_bytes(bytes[4..].try_into()?))
            } else {
                None
            },
        })
    }
}

impl AckFrame {
    pub fn new(cumulative: u32, no: Option<u32>) -> Self {
        Self {
            cumulative,
            current: no,
        }
    }

    pub fn no(&self) -> Option<u32> {
        self.current
    }

    /// The length of contiguous sequence of frames received by the receiver in order.
    ///
    /// For example: If frames arrive in the order `2, 1, 4, 0, 5`
    /// Then cumulative length is `3`, Since frames `0, 1, 2` have been received.
    /// while frame `3` is missing.
    pub fn cumulative_len(&self) -> u32 {
        self.cumulative
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(8);
        buf.extend_from_slice(&self.cumulative.to_le_bytes());
        if let Some(current) = self.current {
            buf.extend_from_slice(&current.to_le_bytes());
        }
        buf
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Unknown(u8) = 0,
    Abort = 1,
    FinAck = 2,
    AckError = 3,
}

impl From<ErrorCode> for u8 {
    fn from(code: ErrorCode) -> Self {
        match code {
            ErrorCode::Abort => 1,
            ErrorCode::FinAck => 2,
            ErrorCode::AckError => 3,
            ErrorCode::Unknown(code) => code,
        }
    }
}

impl From<u8> for ErrorCode {
    fn from(code: u8) -> Self {
        match code {
            1 => Self::Abort,
            2 => Self::FinAck,
            3 => Self::AckError,
            code => Self::Unknown(code),
        }
    }
}

#[derive(Debug)]
pub struct ErrorFrame {
    pub code: ErrorCode,
    pub msg: String,
}

impl From<ErrorCode> for ErrorFrame {
    fn from(code: ErrorCode) -> Self {
        ErrorFrame::new(code, String::new())
    }
}

impl Error for ErrorFrame {}
impl fmt::Display for ErrorFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.msg.fmt(f)
    }
}

impl ErrorFrame {
    pub fn is_fin_ack(&self) -> bool {
        matches!(self.code, ErrorCode::FinAck)
    }

    pub fn new(code: impl Into<ErrorCode>, msg: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            msg: msg.into(),
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(4 + 1 + self.msg.len());
        buf.extend_from_slice(&u32::MAX.to_le_bytes());
        buf.push(self.code.into());
        buf.extend_from_slice(self.msg.as_bytes());
        buf
    }

    pub fn check(frame: impl DataFrameLike) -> Result<()> {
        if frame.as_bytes().len() < 4 {
            bail!("small frame");
        }

        if frame.no() != u32::MAX {
            return Ok(());
        }
        let mut data = frame.data();
        let code: u8 =
            Decode::decode(&mut data).map_err(|e| anyhow!("failed to decode frame: {e}"))?;

        let msg = String::from_utf8(data.to_vec())?;
        Err(Self::new(code, msg).into())
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
        let err = ErrorFrame::new(42, "Hello, World!");
        let err = ErrorFrame::check(err.to_bytes()).err().unwrap();
        let err = err.downcast_ref::<ErrorFrame>().unwrap();
        assert_eq!(err.code, ErrorCode::Unknown(42));
        assert_eq!(err.msg, "Hello, World!");
        Ok(())
    }
}
