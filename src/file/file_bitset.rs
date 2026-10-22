use lipi::{BitSet, BitSetMut};

#[derive(Debug)]
pub struct FileBitSet {
    bitmap: Vec<u8>,
    cap: u32,
    len: u32,

    filled_upto: u32,
}

impl FileBitSet {
    pub fn new(cap: u32) -> Self {
        Self {
            bitmap: vec![0; cap.div_ceil(8) as usize],
            cap,
            len: 0,
            filled_upto: 0,
        }
    }

    pub fn has(&self, no: u32) -> bool {
        assert!(no < self.cap);
        BitSet::has(&self.bitmap, no as usize)
    }

    /// # TODO: Don't Panic!
    pub fn set(&mut self, no: u32) -> bool {
        assert!(no < self.cap);
        let is_already_set = BitSetMut::set(&mut self.bitmap, no as usize).unwrap();
        if !is_already_set {
            self.len += 1;

            if no == self.filled_upto {
                self.advance_filled_upto();
            }
        }
        is_already_set
    }

    pub fn is_full(&self) -> bool {
        self.len == self.cap
    }

    /// # TODO: Make It Faster
    fn advance_filled_upto(&mut self) {
        while self.filled_upto < self.cap && self.has(self.filled_upto) {
            self.filled_upto += 1;
        }
    }

    pub fn filled_upto(&self) -> u32 {
        self.filled_upto
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_set_filled_upto() {
        let size = 9999;
        let mut s = FileBitSet::new(size);

        let mut nums: Vec<u32> = (0..size).collect();
        for curr in nums.chunks_mut(200) {
            fastrand::shuffle(curr);
            for no in curr {
                s.set(*no);
            }
        }

        assert_eq!(s.filled_upto(), size);
    }

    #[test]
    fn test_filled_len() {
        let mut s = FileBitSet::new(100);

        s.set(10);
        assert_eq!(s.filled_upto(), 0);
        assert_eq!(s.len, 1);

        assert_eq!(s.set(0), false);
        assert_eq!(s.filled_upto(), 1);
        assert_eq!(s.len, 2);

        assert_eq!(s.set(0), true); // duplicate doesn't inc len
        assert_eq!(s.filled_upto(), 1);
        assert_eq!(s.len, 2);

        for no in 1..=9 {
            s.set(no);
        }
        assert_eq!(s.filled_upto(), 11);
        assert_eq!(s.len, 11);
    }
}
