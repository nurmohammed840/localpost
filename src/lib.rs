use std::error::Error;

pub use local_ip_address;
pub mod discover;
pub mod endpoint;
pub mod file;
pub mod congestion;
pub mod protocol;

pub type DynError = Box<dyn Error + Send + Sync>;
pub type Result<T, E = DynError> = std::result::Result<T, E>;
