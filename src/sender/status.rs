use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::{fmt, sync::Arc};

#[derive(Default)]
pub struct SendStatus {
    sent: AtomicUsize,
    lost: AtomicUsize,
}

impl SendStatus {
    pub fn new() -> Arc<Self> {
        Default::default()
    }

    pub fn inc_sent(&self) {
        self.sent.fetch_add(1, Relaxed);
    }

    pub fn inc_lost(&self, num: usize) {
        self.lost.fetch_add(num, Relaxed);
    }

    pub fn sent(&self) -> usize {
        self.sent.load(Relaxed)
    }

    pub fn lost(&self) -> usize {
        self.lost.load(Relaxed)
    }
}

impl fmt::Debug for SendStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SendStatus")
            .field("sent", &self.sent())
            .field("lost", &self.lost())
            .finish()
    }
}
