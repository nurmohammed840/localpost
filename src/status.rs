use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

use crate::protocol::AckFrame;
pub use crate::receiver::RecvStatus;
pub use crate::sender::SendStatus;

#[derive(Default)]
pub struct AckCounter(
    /// high 32 bits = selective acks
    /// low  32 bits = just cumulative acks
    AtomicU64,
);

const SELECTIVE_ACK_SHIFT: u32 = 32;
const CUMULATIVE_ACK_MASK: u64 = u32::MAX as u64;

impl AckCounter {
    /// `(selective, cumulative)` from one consistent snapshot.
    pub fn count(&self) -> (u32, u32) {
        let acks = self.0.load(Relaxed);
        (
            (acks >> SELECTIVE_ACK_SHIFT) as u32,
            (acks & CUMULATIVE_ACK_MASK) as u32,
        )
    }

    pub fn selective(&self) -> u32 {
        self.count().0
    }

    pub fn cumulative(&self) -> u32 {
        self.count().1
    }

    pub fn increase(&self, ack: &AckFrame) {
        let is_selective_ack = !ack.bitset.is_empty();

        let delta = if is_selective_ack {
            1 << SELECTIVE_ACK_SHIFT
        } else {
            1
        };

        self.0.fetch_add(delta, Relaxed);
    }
}

impl std::fmt::Debug for AckCounter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (selective, cumulative) = self.count();
        f.debug_struct("AckCounter")
            .field("cumulative", &cumulative)
            .field("selective", &selective)
            .finish()
    }
}
