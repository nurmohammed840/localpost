use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::{fmt, sync::Arc};

use crate::status::AckCounter;

const DUPLICATE_BITS: u32 = 31;
const TOTAL_SHIFT: u32 = DUPLICATE_BITS;
const DUPLICATE_MASK: u64 = (1 << DUPLICATE_BITS) - 1;

#[derive(Default)]
pub struct RecvStatus {
    pub ack_counter: AckCounter,
    // high 33 bits = total (every packet seen, duplicates included)
    // low  31 bits = duplicate
    // received = total - duplicate
    state: AtomicU64,
}

impl RecvStatus {
    pub fn new() -> Arc<Self> {
        Default::default()
    }

    pub fn inc_total_count(&self, num: u32) {
        self.state.fetch_add((num as u64) << TOTAL_SHIFT, Relaxed);
    }

    pub fn inc_duplicate_count(&self) {
        self.state.fetch_add(1, Relaxed);
    }

    pub fn total_count(&self) -> u64 {
        self.total_and_duplicate_count().0
    }

    pub fn duplicate_count(&self) -> u32 {
        self.total_and_duplicate_count().1
    }

    pub fn recv_count(&self) -> u64 {
        let (total, duplicate) = self.total_and_duplicate_count();
        total.saturating_sub(duplicate as u64)
    }

    pub fn total_and_duplicate_count(&self) -> (u64, u32) {
        let state = self.state.load(Relaxed);
        ((state >> TOTAL_SHIFT), (state & DUPLICATE_MASK) as u32)
    }
}

impl fmt::Debug for RecvStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (total, duplicate) = self.total_and_duplicate_count();
        f.debug_struct("RecvStatus")
            .field("total", &total)
            .field("duplicate", &duplicate)
            .field("received", &total.saturating_sub(duplicate as u64))
            .field("acks", &self.ack_counter)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recv_status() {
        let s = RecvStatus::new();
        s.inc_total_count(5);
        s.inc_duplicate_count();
        s.inc_duplicate_count();

        assert_eq!(s.total_and_duplicate_count(), (5, 2));
        assert_eq!(s.recv_count(), 3);
    }
}
