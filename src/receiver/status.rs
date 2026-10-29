use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering::Relaxed},
};

#[derive(Default, Debug)]
pub struct RecvStatus {
    total: AtomicU32,
    received: AtomicU32,
}

impl RecvStatus {
    pub fn new() -> Arc<Self> {
        Default::default()
    }

    pub fn inc_total_recv_count(&self, num: u32) -> u32 {
        self.total.fetch_add(num, Relaxed)
    }

    pub fn inc_recv_count(&self) -> u32 {
        self.received.fetch_add(1, Relaxed)
    }

    pub fn total_recv_count(&self) -> u32 {
        self.total.load(Relaxed)
    }

    pub fn recv_count(&self) -> u32 {
        self.received.load(Relaxed)
    }
}
