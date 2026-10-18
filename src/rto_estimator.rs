use std::time::Duration;

#[derive(Debug)]
pub struct RttEstimator {
    srtt: Duration,
    rttvar: Duration,

    has_sample: bool,
}

impl RttEstimator {
    pub const GRANULARITY: Duration = Duration::from_millis(1);

    pub fn new(rtt: Duration) -> Self {
        Self {
            srtt: rtt,
            rttvar: rtt / 2,
            has_sample: false,
        }
    }

    pub fn rttvar(&self) -> Duration {
        self.rttvar
    }

    pub fn srtt(&self) -> Duration {
        self.srtt
    }

    pub fn update(&mut self, rtt: Duration) {
        if !self.has_sample {
            *self = Self::new(rtt);
            self.has_sample = true;
            return;
        }

        let diff = self.srtt.abs_diff(rtt);
        self.rttvar = (self.rttvar * 3 + diff) / 4;
        self.srtt = (self.srtt * 7 + rtt) / 8;
    }

    pub fn rto(&self) -> Duration {
        self.srtt + (self.rttvar * 4).max(Self::GRANULARITY)
    }
}
