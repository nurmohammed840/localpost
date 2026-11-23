use std::collections::VecDeque;

use anyhow::{Result, bail};

#[derive(Debug)]
pub struct AckBitSet {
    window: VecDeque<u8>,
    /// Number of bytes already dropped from the front. Every bit in them was set.
    ///
    /// same as `filled_upto / 8`
    base: usize,
    cap: u32,
    /// Total number of distinct frames set, including the dropped ones.
    len: u32,
    /// Every frame `< filled_upto` has been set.
    filled_upto: u32,
}

impl AckBitSet {
    pub fn new(cap: u32) -> Self {
        Self::new_with(cap, 0).unwrap()
    }

    pub fn new_with(cap: u32, filled_upto: u32) -> Result<Self> {
        if filled_upto > cap {
            bail!("filled_upto {filled_upto} exceeds capacity {cap}");
        }

        Ok(Self {
            window: VecDeque::new(),
            base: (filled_upto / 8) as usize,
            cap,
            len: filled_upto,
            filled_upto,
        })
    }

    /// Equivalent to: `(no - base * 8) / 8`
    ///
    /// where `base == (filled_upto / 8)`.
    ///
    /// Only valid when `no >= filled_upto`.
    fn slot_idx(&self, no: u32) -> usize {
        (no / 8) as usize - self.base
    }

    /// Returns `Ok(true)` if the frame was already set.
    pub fn set(&mut self, no: u32) -> Result<bool> {
        if no >= self.cap {
            bail!("frame number {no} out of bounds (capacity: {})", self.cap);
        }

        if no < self.filled_upto {
            return Ok(true);
        }

        let slot_idx = self.slot_idx(no);
        let mask = 1u8 << (no % 8);

        if slot_idx >= self.window.len() {
            self.window.resize(slot_idx + 1, 0);
        }

        let slot = &mut self.window[slot_idx];

        let already_set = *slot & mask != 0;

        if already_set {
            return Ok(true);
        }

        *slot |= mask;
        self.len += 1;

        if no == self.filled_upto {
            self.advance_filled_upto();
            self.remove_filled_bytes();
        }

        Ok(false)
    }

    pub fn is_full(&self) -> bool {
        self.len == self.cap
    }

    pub fn filled_upto(&self) -> u32 {
        self.filled_upto
    }

    fn advance_filled_upto(&mut self) {
        while self.filled_upto < self.cap {
            let idx = self.slot_idx(self.filled_upto);
            let Some(&slot) = self.window.get(idx) else {
                break;
            };

            let bit = self.filled_upto % 8;
            let count = (slot >> bit).trailing_ones();

            self.filled_upto += count;

            if count < 8 - bit {
                break; // hit an unset bit
            }
        }
    }

    fn remove_filled_bytes(&mut self) {
        let full_bytes = (self.filled_upto / 8) as usize - self.base;
        self.window.drain(..full_bytes);
        self.base += full_bytes;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_set_filled_upto() {
        let size = 9999;
        let mut s = AckBitSet::new(size);

        let mut nums: Vec<u32> = (0..size).collect();
        for curr in nums.chunks_mut(200) {
            fastrand::shuffle(curr);
            for no in curr {
                s.set(*no).unwrap();
            }
        }

        assert_eq!(s.filled_upto(), size);
    }

    #[test]
    fn test_filled_len() {
        let mut s = AckBitSet::new(100);

        assert!(s.set(100).is_err());

        s.set(10).unwrap();
        assert_eq!(s.filled_upto(), 0);
        assert_eq!(s.len, 1);

        assert_eq!(s.set(0).unwrap(), false);
        assert_eq!(s.filled_upto(), 1);
        assert_eq!(s.len, 2);

        assert_eq!(s.set(0).unwrap(), true); // duplicate doesn't inc len
        assert_eq!(s.filled_upto(), 1);
        assert_eq!(s.len, 2);

        for no in 1..=9 {
            s.set(no).unwrap();
        }
        assert_eq!(s.filled_upto(), 11);
        assert_eq!(s.len, 11);
    }
}
