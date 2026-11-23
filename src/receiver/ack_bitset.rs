use std::{collections::VecDeque, fmt, iter, mem};

use anyhow::{Result, bail};

use crate::protocol::AckFrame;

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

    pub fn ack_frame(&self) -> AckFrame {
        let shift = self.filled_upto % 8; // 0..=7 bits to shift

        let has_ahead = match self.window.len() {
            0 => false,
            1 => self.window[0] >> shift != 0,
            _ => true,
        };

        if !has_ahead {
            return AckFrame {
                cumulative: self.filled_upto,
                bitset: Box::default(),
            };
        }

        let mut bitset = Vec::with_capacity(self.window.len());
        let mut window = self.window.iter().copied().peekable();

        while let Some(lo) = window.next() {
            let lo = lo as u16;
            let hi = window.peek().copied().unwrap_or(0) as u16;

            let byte = ((hi << 8) | lo) >> shift;
            bitset.push(byte as u8);
        }

        while bitset.last() == Some(&0) {
            bitset.pop();
        }

        AckFrame {
            cumulative: self.filled_upto,
            bitset: bitset.into_boxed_slice(),
        }
    }
}

impl AckFrame {
    /// every frame below `cumulative` is acked.
    pub fn is_cumulative_acked(&self, no: u32) -> bool {
        no < self.cumulative
    }

    pub fn is_selective_acked(&self, no: u32) -> bool {
        let Some(off) = no.checked_sub(self.cumulative) else {
            return false; // below `cumulative`: not in the bitset
        };
        let off = off as usize;

        self.bitset
            .get(off / 8)
            .is_some_and(|byte| byte & (1 << (off % 8)) != 0)
    }

    pub fn is_acked(&self, no: u32) -> bool {
        self.is_cumulative_acked(no) || self.is_selective_acked(no)
    }

    pub fn iter(&self) -> impl Iterator<Item = u32> {
        let start = self.cumulative;

        let mut bytes = self.bitset.iter().copied().enumerate();
        let mut cur = 0;
        let mut base = 0;

        iter::from_fn(move || {
            while cur == 0 {
                let (i, byte) = bytes.next()?;
                cur = byte;
                base = 8 * i as u32;
            }

            let bit = cur.trailing_zeros(); // position of lowest set bit
            cur &= cur - 1; // clear lowest set bit; where cur != 0
            Some(start + base + bit)
        })
    }
}

impl fmt::Debug for AckBitSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let window = fmt::from_fn(|f| {
            let mut first = true;

            for slot in &self.window {
                if !mem::take(&mut first) {
                    write!(f, " {slot:01b}")?;
                } else {
                    write!(f, "{slot:01b}")?;
                }
            }
            Ok(())
        });
        f.debug_struct("AckBitSet")
            .field("base", &format_args!("{} ({})", self.base, self.base * 8))
            .field("window", &window)
            .field("cap", &self.cap)
            .field("len", &self.len)
            .field("data", &self.ack_frame())
            .finish()
    }
}

impl fmt::Debug for AckFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let values = fmt::from_fn(|f| f.debug_list().entries(self.iter()).finish());
        f.debug_struct("AckFrame")
            .field("cumulative", &self.cumulative)
            .field("values", &format_args!("{values}"))
            .finish()
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
    fn test_filled_len() -> Result<()> {
        let mut s = AckBitSet::new(100);

        assert!(s.set(100).is_err());

        s.set(10)?;
        assert_eq!(s.filled_upto(), 0);
        assert_eq!(s.len, 1);

        assert_eq!(s.set(0)?, false);
        assert_eq!(s.filled_upto(), 1);
        assert_eq!(s.len, 2);

        assert_eq!(s.set(0)?, true); // duplicate doesn't inc len
        assert_eq!(s.filled_upto(), 1);
        assert_eq!(s.len, 2);

        for no in 1..=9 {
            s.set(no)?;
        }
        assert_eq!(s.filled_upto(), 11);
        assert_eq!(s.len, 11);
        Ok(())
    }

    #[test]
    fn test_ack_frame_iter() -> Result<()> {
        for filled in [0u32, 5, 7, 8, 11, 20, 63] {
            let mut s = AckBitSet::new_with(200, filled)?;
            let mut expected = std::collections::BTreeSet::new();

            for _ in 0..100 {
                let no = fastrand::u32(filled..200);
                s.set(no)?;
                expected.insert(no);

                // drop frames already covered by the cumulative ack
                let want: Vec<u32> = expected.range(s.filled_upto()..).copied().collect();
                let got: Vec<u32> = s.ack_frame().iter().collect();
                assert_eq!(got, want, "filled={filled}");
            }
        }
        Ok(())
    }
}
