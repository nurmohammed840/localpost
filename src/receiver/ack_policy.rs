#![allow(warnings)]
use std::mem;

use crate::protocol::AckFrame;

#[derive(Default)]
pub struct AckPolicy {
    is_ack_sent: bool,
    is_frame_recieved: bool,
}

impl AckPolicy {
    pub fn on_frame_received(&mut self, cumulative: u32, frame_no: u32) -> Option<AckFrame> {
        self.is_frame_recieved = true;

        if frame_no < cumulative {
            return None;
        }

        self.is_ack_sent = true;
        Some(AckFrame::new(cumulative, Some(frame_no)))
    }

    /// This fn should be call at the end of each batch
    pub fn send_ack_if_not_sent(&mut self, cumulative: u32) -> Option<AckFrame> {
        let policy = mem::take(self);
        if !policy.is_frame_recieved || policy.is_ack_sent {
            return None;
        }
        Some(AckFrame::new(cumulative, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_order() {
        let mut policy = AckPolicy::default();

        let mut acks = (0..4).filter_map(|frame_no| policy.on_frame_received(2, frame_no));

        assert_eq!(acks.next(), Some(AckFrame::new(2, Some(2))));
        assert_eq!(acks.next(), Some(AckFrame::new(2, Some(3))));
        assert_eq!(acks.next(), None);

        assert_eq!(policy.is_ack_sent, true);
    }

    #[test]
    fn rev_order() {
        let mut policy = AckPolicy::default();

        let mut acks = (0..4)
            .rev()
            .filter_map(|frame_no| policy.on_frame_received(2, frame_no));

        assert_eq!(acks.next(), Some(AckFrame::new(2, Some(3))));
        assert_eq!(acks.next(), Some(AckFrame::new(2, Some(2))));
        assert_eq!(acks.next(), None);

        assert_eq!(policy.is_ack_sent, true);
    }

    #[test]
    fn yield_single_cumulative_ack_on_deferred_acks() {
        let mut policy = AckPolicy::default();
        let mut acks = (0..4).filter_map(|frame_no| policy.on_frame_received(4, frame_no));

        assert!(acks.next().is_none());
        assert_eq!(policy.send_ack_if_not_sent(5), Some(AckFrame::new(5, None)));
        assert_eq!(policy.send_ack_if_not_sent(5), None);
        assert_eq!(policy.is_ack_sent, false);
    }

    #[test]
    fn test_name() {
        let mut policy = AckPolicy::default();

        let mut acks = [2, 0, 3, 1]
            .into_iter()
            .filter_map(|frame_no| policy.on_frame_received(2, frame_no));

        assert_eq!(acks.next(), Some(AckFrame::new(2, Some(2))));
        assert_eq!(acks.next(), Some(AckFrame::new(2, Some(3))));
        assert_eq!(policy.send_ack_if_not_sent(2), None);
    }
}
