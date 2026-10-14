use lipi::{BitSet, BitSetMut};

#[derive(Debug)]
pub struct FileBitSet {
    bitmap: Vec<u8>,
    len: u32,
    completed_count: u32,
}

impl FileBitSet {
    pub fn new(len: u32) -> Self {
        Self {
            bitmap: vec![0; len.div_ceil(8) as usize],
            len,
            completed_count: 0,
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
        }
        is_already_set
    }

    pub fn is_full(&self) -> bool {
        self.completed_count == self.len
    }
}
