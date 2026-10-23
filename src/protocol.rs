use crate::Result;
use lipi::{Decode, Encode};
use std::{env::consts as SYS, error::Error, fmt, ops::RangeBounds};

#[repr(u8)]
#[derive(Encode, Decode, Debug)]
pub enum Message {
    Handshake(Hello) = 0,

    Write(FileInfo) = 1,
    TransferID(TransferID) = 2,
}

#[derive(Debug, Encode, Decode, Clone, Copy)]
pub struct TransferID {
    #[key = 0]
    pub udp_port: u16,

    #[key = 1]
    pub file_id: u64,
}

impl Default for TransferID {
    fn default() -> Self {
        Self {
            udp_port: 0,
            file_id: fastrand::u64(..),
        }
    }
}

#[derive(Debug, Encode, Decode)]
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
    pub fn num_of_frames(&self) -> u64 {
        self.size.div_ceil(self.frame_size as u64)
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
            return Err(format!("Invalid protocol: {}", self.protocol).into());
        }
        if !target.contains(&self.version) {
            let version = format_protocol_version(self.version);
            return Err(format!("Invalid protocol version: {version}").into());
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
            current: self.no(),
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
        if buf.len() < 8 {
            return Err("small frame".into());
        }
        Ok(Self { buf })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AckFrame {
    current: u32,
    cumulative: u32,
}

impl AckFrame {
    pub fn new(no: u32, cumulative: u32) -> Self {
        Self {
            current: no,
            cumulative,
        }
    }

    pub fn from(bytes: &[u8]) -> Self {
        Self {
            current: u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
            cumulative: u32::from_le_bytes(bytes[4..8].try_into().unwrap()),
        }
    }

    pub fn no(&self) -> u32 {
        self.current
    }

    pub fn cumulative(&self) -> u32 {
        self.cumulative
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(8);
        buf.extend_from_slice(&self.current.to_le_bytes());
        buf.extend_from_slice(&self.cumulative.to_le_bytes());
        buf
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    Unknown(u8) = 0,
    Other = 1,
}

impl From<ErrorCode> for u8 {
    fn from(code: ErrorCode) -> Self {
        match code {
            ErrorCode::Other => 1,
            ErrorCode::Unknown(code) => code,
        }
    }
}

impl From<u8> for ErrorCode {
    fn from(code: u8) -> Self {
        match code {
            1 => Self::Other,
            code => Self::Unknown(code),
        }
    }
}

#[derive(Debug)]
pub struct ErrorFrame {
    code: ErrorCode,
    msg: String,
}

impl Error for ErrorFrame {}
impl fmt::Display for ErrorFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.msg.fmt(f)
    }
}

impl ErrorFrame {
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

    pub fn parse_from(frame: impl DataFrameLike) -> Result<Option<Self>> {
        if frame.no() != u32::MAX {
            return Ok(None);
        }
        let mut data = frame.data();
        let code: u8 = lipi::Decode::decode(&mut data)?;
        let msg = String::from_utf8(data.to_vec())?;
        Ok(Some(Self::new(code, msg)))
    }
}

// =============================================================

fn format_protocol_version(num: u8) -> String {
    let major = num / 10;
    let minor = num % 10;
    format!("{}.{}", major, minor)
}

macro_rules! impl_from {
    [$($ty:ty => $variant:ident),*] => [$(
        impl From<$ty> for Message {
            fn from(value: $ty) -> Self {
                Self::$variant(value)
            }
        }
    )*];
}

impl_from! {
    Hello => Handshake,
    FileInfo => Write,
    TransferID => TransferID
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Message::Handshake(_) => "Handshake",
            Message::Write(_) => "Write Request",
            Message::TransferID(_) => "Transfer ID",
        })
    }
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
    fn error_frame_parsing() -> Result<()> {
        let err = ErrorFrame::new(42, "Hello, World!");
        let err = ErrorFrame::parse_from(err.to_bytes())?.unwrap();
        assert_eq!(err.code, ErrorCode::Unknown(42));
        assert_eq!(err.msg, "Hello, World!");
        Ok(())
    }
}
