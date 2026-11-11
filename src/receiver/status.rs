use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering::Relaxed},
    },
};

const SHIFT: u32 = 32;
const MASK: u64 = u32::MAX as u64;

#[derive(Default)]
pub struct RecvStatus {
    // high 32 bits = total, low 32 bits = received
    state: AtomicU64,
}

impl RecvStatus {
    pub fn new() -> Arc<Self> {
        Default::default()
    }

    pub fn inc_total_recv_count(&self, num: u32) -> u32 {
        Self::unpack(self.state.fetch_add((num as u64) << SHIFT, Relaxed)).0
    }

    pub fn inc_recv_count(&self) -> u32 {
        Self::unpack(self.state.fetch_add(1, Relaxed)).1
    }

    pub fn total_recv_count(&self) -> u32 {
        self.total_and_recv_count().0
    }

    pub fn recv_count(&self) -> u32 {
        self.total_and_recv_count().1
    }

    pub fn total_and_recv_count(&self) -> (u32, u32) {
        Self::unpack(self.state.load(Relaxed))
    }

    fn unpack(s: u64) -> (u32, u32) {
        ((s >> SHIFT) as u32, (s & MASK) as u32)
    }
}

impl fmt::Debug for RecvStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (total, received) = self.total_and_recv_count();
        f.debug_struct("RecvStatus")
            .field("total", &total)
            .field("received", &received)
            .finish()
    }
}
