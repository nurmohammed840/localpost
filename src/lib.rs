#[doc(hidden)]
pub mod deps {
    pub use anyhow;
    pub use futures;
    pub use nio;
}

pub mod discover;
pub mod endpoint;
pub mod file;
pub mod protocol;
pub mod receiver;
pub mod sender;
pub mod socket;
