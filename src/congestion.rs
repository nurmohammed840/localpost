use std::{
    cmp, mem,
    time::{Duration, Instant},
};

use crate::{
    protocol::DataFrame,
    utils::{IteratorExt, VecExt},
};

#[derive(Debug)]
pub struct Payload {
    send_at: Instant,
    data: DataFrame,
}

impl Payload {
    pub fn new(data: DataFrame) -> Self {
        Self {
            send_at: Instant::now(),
            data,
        }
    }
}

#[derive(Debug)]
pub struct Control {
    step: usize,
    capacity: usize,
    reuse_free_space: usize,

    /// Highest `len()` reached this round. Used to tell whether the window
    /// was actually used, so an idle sender doesn't inflate `capacity`.
    peak_len: usize,

    in_flight: Vec<Payload>,
    queued_in_flight: Vec<Payload>,
    lost_in_flight: Vec<DataFrame>,
}

impl Control {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            step: capacity,
            capacity,
            reuse_free_space: 0,

            peak_len: 0,

            in_flight: Vec::new(),
            queued_in_flight: Vec::new(),
            lost_in_flight: Vec::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.in_flight.len() + self.queued_in_flight.len() + self.lost_in_flight.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn remaining_capacity(&self) -> usize {
        self.capacity.saturating_sub(self.len())
    }

    pub fn send(&mut self, data: DataFrame) {
        assert!(self.remaining_capacity() != 0);
        if self.reuse_free_space != 0 {
            self.reuse_free_space -= 1;
            self.queued_in_flight.push(Payload::new(data));
        } else {
            self.in_flight.push(Payload::new(data));
        }

        self.peak_len = self.peak_len.max(self.len());
    }

    pub fn make_as_complete(&mut self, no: u32) {
        let match_by_no = |data: &DataFrame| data.no() == no;
        let found = |p: &Payload| match_by_no(&p.data);

        let is_removed = self.in_flight.remove_first_if(found).is_some()
            || self.queued_in_flight.remove_first_if(found).is_some()
            || self.lost_in_flight.remove_first_if(match_by_no).is_some();

        if is_removed && self.capacity > self.reuse_free_space {
            self.reuse_free_space += 1;
        }
    }

    pub fn process_lost(&mut self, dur: Duration) {
        let now = Instant::now();

        let is_expired = |p: &mut Payload| now.duration_since(p.send_at) >= dur;
        let data_frame = |p: Payload| p.data;

        self.in_flight
            .extract_if(.., is_expired)
            .map(data_frame)
            .extend_into(&mut self.lost_in_flight);

        self.queued_in_flight
            .extract_if(.., is_expired)
            .map(data_frame)
            .extend_into(&mut self.lost_in_flight);
    }

    pub fn collect_lost_frames(&mut self) -> Option<LostFrames> {
        if !self.in_flight.is_empty() {
            return None;
        }

        self.in_flight.append(&mut self.queued_in_flight);

        let lost_frames = mem::take(&mut self.lost_in_flight);
        if lost_frames.is_empty() {
            if self.peak_len >= self.capacity {
                self.step += 1;
                self.capacity += self.step;
            }
        } else {
            self.step = 1;
            self.capacity = self.capacity.saturating_sub(lost_frames.len() * 2).max(1);
        }

        self.reuse_free_space = 0;
        self.peak_len = self.len();
        Some(LostFrames::new(lost_frames))
    }
}

#[derive(Debug)]
pub struct LostFrames {
    data: Vec<DataFrame>,
}

impl LostFrames {
    pub fn new(mut frames: Vec<DataFrame>) -> Self {
        frames.sort_unstable_by_key(|frame| cmp::Reverse(frame.no()));
        Self { data: frames }
    }

    pub fn pop(&mut self) -> Option<DataFrame> {
        self.data.pop()
    }

    pub fn make_as_complete(&mut self, no: u32) -> bool {
        self.data.remove_first_if(|data| data.no() == no).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(no: u32) -> DataFrame {
        DataFrame::new(no, &[])
    }

    #[test]
    fn test_send() {
        let mut c = Control::new(2);
        c.send(block(0));
        c.send(block(1));
        println!("c: {:#?}", c);
    }
}
