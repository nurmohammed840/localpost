use lipi::{BitSet, BitSetMut};

#[derive(Debug)]
pub struct FileBitSet {
    bitmap: Vec<u8>,
    len: u32,
    completed_count: u32,

    filled_upto: u32,
}

impl FileBitSet {
    pub fn new(len: u32) -> Self {
        Self {
            bitmap: vec![0; len.div_ceil(8) as usize],
            len,
            completed_count: 0,
            filled_upto: 0,
        }
    }

    pub fn has(&self, no: u32) -> bool {
        assert!(no < self.len);
        BitSet::has(&self.bitmap, no as usize)
    }

    pub fn set(&mut self, no: u32) -> bool {
        assert!(no < self.len);
        let is_already_set = BitSetMut::set(&mut self.bitmap, no as usize).unwrap();
        if !is_already_set {
            self.completed_count += 1;

            if no == self.filled_upto {
                self.advance_filled_upto();
            }
        }
        is_already_set
    }

    pub fn is_full(&self) -> bool {
        self.completed_count == self.len
    }

    /// TODO: Make It Faster
    fn advance_filled_upto(&mut self) {
        while self.filled_upto < self.len && self.has(self.filled_upto) {
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
}
