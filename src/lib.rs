pub use local_ip_address;
use std::error::Error;

pub mod discover;
pub mod endpoint;
pub mod file;
pub mod protocol;
pub mod receiver;
pub mod sender;
pub mod socket;

pub type DynError = Box<dyn Error + Send + Sync>;
pub type Result<T, E = DynError> = std::result::Result<T, E>;
