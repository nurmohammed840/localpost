use crate::Result;
use lipi::{Decode, Encode};
use std::{env::consts as SYS, fmt, ops::RangeBounds};

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

impl DataFrame {
    pub fn new(no: u32, data: &[u8]) -> Self {
        let mut buf = Vec::with_capacity(4 + data.len());
        buf.extend_from_slice(&no.to_le_bytes());
        buf.extend_from_slice(data);
        Self {
            buf: buf.into_boxed_slice(),
        }
    }

    pub fn ack(&self) -> AckFrame {
        AckFrame {
            buf: self.buf[0..4].try_into().unwrap(),
        }
    }

    pub fn from(buf: Vec<u8>) -> Self {
        assert!(buf.len() > 4);
        Self {
            buf: buf.into_boxed_slice(),
        }
    }

    pub fn no(&self) -> u32 {
        let bytes = self.buf[0..4].try_into().unwrap();
        u32::from_le_bytes(bytes)
    }

    pub fn data(&self) -> &[u8] {
        &self.buf[4..]
    }

    pub fn to_bytes(&self) -> &[u8] {
        &self.buf
    }
}

pub struct AckFrame {
    buf: [u8; 4],
}

impl AckFrame {
    pub fn new(no: u32) -> Self {
        Self {
            buf: no.to_le_bytes(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
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

    pub fn parse_from(frame: &DataFrame) -> Result<Option<Self>> {
        if frame.no() != u32::MAX {
            return Ok(None);
        }
        let mut reader = frame.data();
        let code: u8 = lipi::Decode::decode(&mut reader)?;
        let msg = String::from_utf8(reader.to_vec())?;
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
    fn test_data_frame() {
        let frame = DataFrame::from(vec![0, 1, 2, 3, 4]);
        assert_eq!(frame.no(), u32::from_le_bytes([0, 1, 2, 3]));
        assert_eq!(frame.data(), [4]);
    }

    #[test]
    fn test_error_frame_parsing() -> Result<()> {
        let err = ErrorFrame::new(42, "Hello, World!");
        let frame = DataFrame::from(err.to_bytes());

        let err = ErrorFrame::parse_from(&frame)?.unwrap();
        assert_eq!(err.code, ErrorCode::Unknown(42));
        assert_eq!(err.msg, "Hello, World!");
        Ok(())
    }
}
