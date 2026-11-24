use std::sync::atomic::{AtomicU32, AtomicU64, Ordering::Relaxed};
use std::{fmt, sync::Arc};

use crate::status::AckCounter;

#[derive(Default)]
pub struct SendStatus {
    pub ack_counter: AckCounter,
    sent: AtomicU64,
    lost: AtomicU32,
}

impl SendStatus {
    pub fn new() -> Arc<Self> {
        Default::default()
    }

    pub fn inc_sent(&self) {
        self.sent.fetch_add(1, Relaxed);
    }

    pub fn inc_lost(&self, num: usize) {
        self.lost.fetch_add(num as u32, Relaxed);
    }

    pub fn sent(&self) -> u64 {
        self.sent.load(Relaxed)
    }

    pub fn lost(&self) -> u32 {
        self.lost.load(Relaxed)
    }
}

impl fmt::Debug for SendStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SendStatus")
            .field("sent", &self.sent())
            .field("lost", &self.lost())
            .field("acks", &self.ack_counter)
            .finish()
    }
}
