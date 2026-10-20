use std::{
    cmp::Reverse,
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
    lost_in_flight: Vec<Payload>,
}

impl Default for Control {
    fn default() -> Self {
        Self::new(10)
    }
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

    pub fn mark_as_complete(&mut self, no: u32) -> Option<Instant> {
        let found = |p: &Payload| p.data.no() == no;

        let removed = self
            .in_flight
            .remove_first_if(found)
            .or_else(|| self.queued_in_flight.remove_first_if(found))
            .or_else(|| self.lost_in_flight.remove_first_if(found));

        if removed.is_some() && self.remaining_capacity() > self.reuse_free_space {
            self.reuse_free_space += 1;
        }

        removed.map(|p| p.send_at)
    }

    pub fn process_lost(&mut self, dur: Duration) {
        let now = Instant::now();

        let is_expired = |p: &mut Payload| now.duration_since(p.send_at) >= dur;

        self.in_flight
            .extract_if(.., is_expired)
            .extend_into(&mut self.lost_in_flight);

        self.queued_in_flight
            .extract_if(.., is_expired)
            .extend_into(&mut self.lost_in_flight);
    }

    pub fn all_space_are_used(&self) -> bool {
        let len = self.len() + self.reuse_free_space;
        len >= self.capacity
    }

    pub fn take_lost_frames_if_in_flight_is_empty(
        &mut self,
        lost_list: &mut LostList,
    ) -> Option<usize> {
        if !self.in_flight.is_empty() {
            return None;
        }

        if !self.all_space_are_used() {
            return None;
        }

        self.in_flight.append(&mut self.queued_in_flight);

        let lost_count = self.lost_in_flight.len();
        if lost_count == 0 {
            self.capacity += self.step;
            self.step += 1;
        } else {
            self.step = 1;
            self.capacity = self.capacity.saturating_sub(lost_count * 2).max(1);
        }

        lost_list.add_lost_frames(self.lost_in_flight.drain(..));
        self.reuse_free_space = 0;
        Some(lost_count)
    }
}

#[derive(Debug, Default)]
pub struct LostList {
    set: Vec<Payload>,
}

impl LostList {
    fn add_lost_frames(&mut self, other: impl IntoIterator<Item = Payload>) {
        self.set.extend(other);
        self.set
            .sort_unstable_by_key(|payload| Reverse(payload.data.no()));
    }

    pub fn pop(&mut self) -> Option<DataFrame> {
        self.set.pop().map(|payload| payload.data)
    }

    pub fn len(&self) -> usize {
        self.set.len()
    }

    pub fn is_empty(&self) -> bool {
        self.set.is_empty()
    }

    pub fn mark_as_complete(&mut self, no: u32) -> Option<Instant> {
        self.set
            .remove_first_if(|payload| payload.data.no() == no)
            .map(|p| p.send_at)
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use super::*;

    fn frame(no: u32) -> DataFrame {
        DataFrame::new(no, &[])
    }

    impl Control {
        fn send_frame(&mut self, no: u32) {
            self.send(frame(no));
        }

        fn mark_as_lost(&mut self, no: u32) {
            let found = |p: &Payload| p.data.no() == no;

            self.in_flight
                .remove_first_if(found)
                .into_iter()
                .extend_into(&mut self.lost_in_flight);

            self.queued_in_flight
                .remove_first_if(found)
                .into_iter()
                .extend_into(&mut self.lost_in_flight);
        }

        fn collect_lost_frames(&mut self) -> Option<LostList> {
            let mut lost_frames = LostList::default();
            self.take_lost_frames_if_in_flight_is_empty(&mut lost_frames)
                .map(|_| lost_frames)
        }
    }

    #[test]
    fn test_send_blocks() {
        let mut c = Control::new(3);
        c.send_frame(2);
        c.send_frame(0);
        c.send_frame(1);

        assert_eq!(c.remaining_capacity(), 0);

        c.mark_as_complete(0);
        c.mark_as_complete(2);
        c.mark_as_complete(1);

        assert_eq!(c.remaining_capacity(), 3);
        c.collect_lost_frames().unwrap();

        assert!(c.is_empty());
        assert_eq!(c.step, 4);
        assert_eq!(c.remaining_capacity(), 6);
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
        assert_eq!(c.remaining_capacity(), 0);

        c.mark_as_complete(5);
        assert_eq!(c.remaining_capacity(), 1);

        assert_eq!(c.reuse_free_space, 1);

        c.send_frame(0);

        c.collect_lost_frames().unwrap();
        c.send_frame(1);

        assert_eq!(c.step, 2);
        assert_eq!(c.capacity, 2);
        assert_eq!(c.len(), 2);
    }

    #[test]
    fn test_partial_send_does_not_change_capacity() {
        let mut c = Control::new(2);

        assert!(c.collect_lost_frames().is_none());
        assert_eq!(c.step, 2);
        assert_eq!(c.capacity, 2);

        c.send_frame(0);
        c.mark_as_complete(0);

        c.send_frame(1);
        c.mark_as_complete(1);

        assert!(c.collect_lost_frames().is_none());

        c.send_frame(0);
        c.send_frame(1);

        c.mark_as_complete(0);
        c.mark_as_complete(1);

        c.collect_lost_frames().unwrap();
        assert_eq!(c.step, 3);
        assert_eq!(c.capacity, 4);
    }

    #[test]
    fn test_capacity_increasing_linearly_on_each_step() {
        let mut c = Control::new(0);

        fn next(c: &mut Control) -> (usize, usize) {
            let cap = c.remaining_capacity();
            (0..cap).for_each(|no| c.send_frame(no as u32));
            (0..cap).for_each(|no| {
                c.mark_as_complete(no as u32);
            });
            c.collect_lost_frames().unwrap();
            (c.step, c.capacity)
        }

        assert_eq!((c.step, c.capacity), (1, 1));
        assert_eq!(next(&mut c), (2, 2));
        assert_eq!(next(&mut c), (3, 4));
        assert_eq!(next(&mut c), (4, 7));
        assert_eq!(next(&mut c), (5, 11));
        assert_eq!(next(&mut c), (6, 16));
        assert_eq!(next(&mut c), (7, 22));
        assert_eq!(next(&mut c), (8, 29));
        assert_eq!(next(&mut c), (9, 37));
        assert_eq!(next(&mut c), (10, 46));
    }

    #[test]
    fn test_process_lost() {
        let mut c = Control::new(2);

        c.send_frame(0);
        thread::sleep(Duration::from_millis(100));
        c.process_lost(Duration::from_millis(150));
        assert!(c.lost_in_flight.is_empty());

        c.send_frame(1);
        thread::sleep(Duration::from_millis(100));
        c.process_lost(Duration::from_millis(50));
        assert_eq!(c.lost_in_flight.len(), 2);
    }

    fn payload(no: u32) -> Payload {
        Payload::new(frame(no))
    }

    #[test]
    fn test_complete_frames_increase_reuse_space() {
        let mut c = Control {
            step: 1,
            capacity: 1,
            in_flight: vec![payload(0), payload(1), payload(2)],
            ..Default::default()
        };

        c.mark_as_complete(0);
        assert_eq!(c.reuse_free_space, 0);
        assert_eq!(c.remaining_capacity(), 0);

        c.mark_as_complete(1);
        assert_eq!(c.reuse_free_space, 0);
        assert_eq!(c.remaining_capacity(), 0);

        c.mark_as_complete(2);
        assert_eq!(c.reuse_free_space, 1);
        assert_eq!(c.remaining_capacity(), 1);
    }

    #[test]
    fn test_lost_frame_recovery_order() {
        let mut lost_list = LostList::default();
        lost_list.add_lost_frames([payload(2), payload(4), payload(1)]);
        lost_list.add_lost_frames([payload(6), payload(3)]);
        lost_list.add_lost_frames([payload(5)]);

        let mut list = vec![];
        while let Some(pop) = lost_list.pop() {
            list.push(pop.no());
        }

        assert_eq!(list, [1, 2, 3, 4, 5, 6]);
    }
}
