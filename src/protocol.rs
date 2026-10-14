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
