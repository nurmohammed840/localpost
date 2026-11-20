use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering::Relaxed};
use std::{fmt, sync::Arc};

use crate::protocol::AckFrame;

const REGULAR_ACK_SHIFT: u32 = 32;
const CUMULATIVE_ACK_MASK: u64 = u32::MAX as u64;

#[derive(Default)]
pub struct SendStatus {
    sent: AtomicUsize,
    lost: AtomicUsize,

    // high 32 bits = regular acks received (ack.no() is Some)
    // low  32 bits = cumulative acks received (ack.no() is None)
    acks: AtomicU64,
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

    pub fn inc_ack_count(&self, ack: AckFrame) {
        let is_regular_ack = ack.no().is_some();
        let delta = if is_regular_ack {
            1u64 << REGULAR_ACK_SHIFT
        } else {
            1
        };
        self.acks.fetch_add(delta, Relaxed);
    }

    pub fn sent(&self) -> usize {
        self.sent.load(Relaxed)
    }

    pub fn lost(&self) -> usize {
        self.lost.load(Relaxed)
    }

    pub fn regular_acks(&self) -> u32 {
        self.acks().0
    }

    pub fn cumulative_acks(&self) -> u32 {
        self.acks().1
    }

    /// `(regular, cumulative)` from one consistent snapshot.
    pub fn acks(&self) -> (u32, u32) {
        let acks = self.acks.load(Relaxed);
        (
            (acks >> REGULAR_ACK_SHIFT) as u32,
            (acks & CUMULATIVE_ACK_MASK) as u32,
        )
    }
}

impl fmt::Debug for SendStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (regular, cumulative) = self.acks();

        f.debug_struct("SendStatus")
            .field("sent", &self.sent())
            .field("lost", &self.lost())
            .field("regular_acks", &regular)
            .field("cumulative_acks", &cumulative)
            .finish()
    }
}
