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
    }

    pub fn mark_as_complete(&mut self, no: u32) {
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

    pub fn are_all_space_used(&self) -> bool {
        let len = self.len() + self.reuse_free_space;
        len >= self.capacity
    }

    pub fn collect_lost_frames(&mut self) -> Option<LostFrames> {
        if !self.in_flight.is_empty() {
            return None;
        }

        self.in_flight.append(&mut self.queued_in_flight);

        let lost_frames = mem::take(&mut self.lost_in_flight);
        if lost_frames.is_empty() {
            if self.are_all_space_used() {
                self.step += 1;
                self.capacity += self.step;
            }
        } else {
            self.step = 1;
            self.capacity = self.capacity.saturating_sub(lost_frames.len() * 2).max(1);
        }

        self.reuse_free_space = 0;
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

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn make_as_complete(&mut self, no: u32) -> bool {
        self.data.remove_first_if(|data| data.no() == no).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Control {
        fn send_frame(&mut self, no: u32) {
            self.send(DataFrame::new(no, &[]));
        }

        fn mark_as_lost(&mut self, no: u32) {
            let found = |p: &Payload| p.data.no() == no;
            let data_frame = |p: Payload| p.data;

            self.in_flight
                .remove_first_if(found)
                .map(data_frame)
                .into_iter()
                .extend_into(&mut self.lost_in_flight);

            self.queued_in_flight
                .remove_first_if(found)
                .map(data_frame)
                .into_iter()
                .extend_into(&mut self.lost_in_flight);
        }
    }

    #[test]
    fn test_send_blocks() {
        let mut c = Control::new(3);
        c.send_frame(2);
        c.send_frame(0);
        c.send_frame(1);

        c.mark_as_complete(0);
        c.mark_as_complete(2);
        c.mark_as_complete(1);

        c.collect_lost_frames().unwrap();

        assert!(c.is_empty());
        assert_eq!(c.step, 4);
        assert_eq!(c.remaining_capacity(), 7);
    }

    #[test]
    fn test_mark_and_recover_lost_frames() {
        let mut c = Control::new(4);

        for no in 0..=3 {
            c.send_frame(no);
        }

        c.mark_as_lost(0);
        c.mark_as_lost(1);
        c.mark_as_complete(2);
        c.mark_as_complete(3);

        c.send_frame(4);
        c.send_frame(5);

        let mut lost = c.collect_lost_frames().unwrap();
        assert_eq!(lost.pop().unwrap().no(), 0);
        assert_eq!(lost.pop().unwrap().no(), 1);

        assert_eq!(c.step, 1);
        assert_eq!(c.capacity, 1);
        assert_eq!(c.len(), 2);

        c.mark_as_complete(4);
        c.mark_as_complete(5);

        assert_eq!(c.remaining_capacity(), 1);
        c.send_frame(0);

        c.collect_lost_frames().unwrap();
        c.send_frame(1);

        assert_eq!(c.step, 2);
        assert_eq!(c.capacity, 3);
        assert_eq!(c.len(), 2);
    }

    #[test]
    fn test_capacity_remains_stable_when_sender_empty() {
        let mut c = Control::new(2);

        c.collect_lost_frames().unwrap();
        assert_eq!(c.step, 2);
        assert_eq!(c.capacity, 2);

        c.send_frame(0);
        c.mark_as_complete(0);

        c.send_frame(1);
        c.mark_as_complete(1);

        c.collect_lost_frames().unwrap();
        assert_eq!(c.step, 2);
        assert_eq!(c.capacity, 2);

        c.send_frame(0);
        c.send_frame(1);

        c.mark_as_complete(0);
        c.mark_as_complete(1);

        c.collect_lost_frames().unwrap();
        assert_eq!(c.step, 3);
        assert_eq!(c.capacity, 5);
    }

    #[test]
    fn test_capacity_increasing_linearly_on_each_step() {
        let mut c = Control::new(0);

        fn next(c: &mut Control) -> (usize, usize) {
            let cap = c.remaining_capacity();
            (0..cap).for_each(|no| c.send_frame(no as u32));
            (0..cap).for_each(|no| c.mark_as_complete(no as u32));
            c.collect_lost_frames().unwrap();

            let res = (c.step, c.capacity);
            assert_eq!(formula(c.step), res);
            res
        }

        fn formula(step: usize) -> (usize, usize) {
            (step, step * (step + 1) / 2)
        }

        assert_eq!((c.step, c.capacity), (1, 1));
        assert_eq!(next(&mut c), (2, 3));
        assert_eq!(next(&mut c), (3, 6));
        assert_eq!(next(&mut c), (4, 10));
        assert_eq!(next(&mut c), (5, 15));
        assert_eq!(next(&mut c), (6, 21));
        assert_eq!(next(&mut c), (7, 28));
        assert_eq!(next(&mut c), (8, 36));
        assert_eq!(next(&mut c), (9, 45));
        assert_eq!(next(&mut c), (10, 55));
    }
}
