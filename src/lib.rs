pub use local_ip_address;
use std::error::Error;

pub mod congestion;
pub mod discover;
pub mod rto_estimator;
pub mod endpoint;
pub mod file;
pub mod protocol;
mod utils;

pub type DynError = Box<dyn Error + Send + Sync>;
pub type Result<T, E = DynError> = std::result::Result<T, E>;
