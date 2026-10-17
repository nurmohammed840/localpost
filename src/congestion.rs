use crate::protocol::DataFrame;
use std::{
    mem,
    time::{Duration, Instant},
};

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

pub enum Plane {
    New,
    InFlight(Payload),
    Landed,
    NextInFlight(Payload),
    Lost(Payload),
}

impl Plane {
    pub fn is_new(&self) -> bool {
        matches!(self, Self::New)
    }

    pub fn is_landed(&self) -> bool {
        matches!(self, Self::Landed)
    }

    pub fn is_in_flight(&self) -> bool {
        matches!(self, Plane::InFlight(_))
    }

    pub fn mark_as_lost(&mut self) {
        match mem::replace(self, Plane::New) {
            Plane::InFlight(payload) | Plane::NextInFlight(payload) => *self = Plane::Lost(payload),
            Plane::Lost(_) => panic!("already lost"),
            Plane::New | Plane::Landed => panic!("not in flight"),
        }
    }

    pub fn set_in_flight(&mut self, data: DataFrame) {
        let plane = mem::replace(self, Plane::InFlight(Payload::new(data)));
        assert!(plane.is_new());
    }

    pub fn set_next_in_flight(&mut self, data: DataFrame) {
        let plane = mem::replace(self, Plane::NextInFlight(Payload::new(data)));
        assert!(plane.is_landed());
    }

    pub fn active_in_flight(&mut self) -> Option<&mut Payload> {
        match self {
            Plane::InFlight(p) | Plane::NextInFlight(p) => Some(p),
            _ => None,
        }
    }

    pub fn payload(&self) -> Option<&Payload> {
        match self {
            Plane::InFlight(p) | Plane::NextInFlight(p) | Plane::Lost(p) => Some(p),
            _ => None,
        }
    }
}

pub struct Control {
    window_size: usize,
    window: Vec<Plane>,

    queued_in_flight: Vec<Payload>,
    lost_in_flight: Vec<DataFrame>,
}

impl Control {
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size,
            window: Vec::with_capacity(window_size),

            queued_in_flight: Vec::with_capacity(window_size),
            lost_in_flight: Vec::with_capacity(window_size),
        }
    }

    pub fn make_new(&mut self) -> Option<&mut Plane> {
        if self.window_size >= self.window.len() {
            return None;
        }
        self.window.push(Plane::New);
        self.window.last_mut()
    }

    pub fn find_available(&mut self) -> Option<&mut Plane> {
        self.iter().find(|s| s.is_landed())
    }

    pub fn iter(&mut self) -> impl Iterator<Item = &mut Plane> {
        self.window.iter_mut()
    }

    pub fn make_as_landed(&mut self, no: u32) -> Option<()> {
        remove_first_if(&mut self.queued_in_flight, |p| p.data.no() == no);
        remove_first_if(&mut self.lost_in_flight, |p| p.no() == no);

        let plane = self
            .iter()
            .find(|plane| plane.payload().is_some_and(|p| p.data.no() == no))?;

        let _ = mem::replace(plane, Plane::Landed);

        Some(())
    }

    pub fn process_lost(&mut self, dur: Duration) {
        let now = Instant::now();

        let lost = self
            .queued_in_flight
            .extract_if(.., |payload| now.duration_since(payload.send_at) >= dur)
            .map(|payload| payload.data);

        self.lost_in_flight.extend(lost);

        for plane in self.iter() {
            if let Some(payload) = plane.active_in_flight()
                && now.duration_since(payload.send_at) >= dur
            {
                plane.mark_as_lost();
            }
        }
    }

    pub fn try_make_new_batch(&mut self) -> bool {
        let batch_not_yet_complete = self.iter().any(|plane| plane.is_in_flight());
        if batch_not_yet_complete {
            return false;
        }

        for plane in self.window.drain(..) {
            match plane {
                Plane::New => panic!("unused plane"),
                Plane::InFlight(_) => {
                    // unreachable!()
                }
                Plane::Landed => {}
                Plane::NextInFlight(payload) => self.queued_in_flight.push(payload),
                Plane::Lost(Payload { data, .. }) => self.lost_in_flight.push(data),
            }
        }
        true
    }

    pub fn process_ack(&mut self, no: u32, max_dur: Duration) {
        self.make_as_landed(no);
        self.process_lost(max_dur);
        self.try_make_new_batch();
    }
}

fn remove_first_if<T>(v: &mut Vec<T>, f: impl FnMut(&T) -> bool) {
    if let Some(i) = v.iter().position(f) {
        v.swap_remove(i);
    }
}
