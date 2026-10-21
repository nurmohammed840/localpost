pub mod congestion;
pub mod rtt_estimator;
mod utils;

use std::{
    io::{self, ErrorKind, Read, Seek},
    iter,
    time::{Duration, Instant},
};

use crate::{Result, protocol::AckFrame};
use crate::{
    file::{FileReader, FrameReader},
    socket::DatagramSocket,
};
use congestion::{Control, LostList};
use rtt_estimator::RttEstimator;

pub struct FileSender<T, S> {
    reader: FrameReader<T>,
    socket: S,

    lost_list: LostList,

    rtt_estimator: RttEstimator,
    congestion: Control,

    last_seen: Instant,
    pub conn_timeout: Duration,

    recv_buf: Vec<u8>,
}

impl<T, S> FileSender<T, S>
where
    T: Seek + Read,
    S: DatagramSocket,
{
    pub fn new(reader: FileReader<T>, socket: S) -> Self {
        Self {
            reader: FrameReader::new(reader),
            socket,
            lost_list: LostList::default(),
            rtt_estimator: RttEstimator::new(Duration::from_secs(3)),
            congestion: Control::new(10),
            last_seen: Instant::now(),
            conn_timeout: Duration::from_secs(7),
            recv_buf: vec![0; 1024],
        }
    }

    fn recv_ack(&mut self) -> Result<Option<AckFrame>> {
        let rto = self.rtt_estimator.rto();
        self.socket.set_read_timeout(Some(rto))?;

        let len = match self.socket.recv(&mut self.recv_buf) {
            Err(err) if matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                return Ok(None);
            }
            len => len,
        }?;

        let data = &self.recv_buf[..len];
        Ok(data.try_into().ok().map(AckFrame::from))
    }

    pub fn init_send(&mut self) -> Result<()> {
        loop {
            let frames = iter::from_fn(|| {
                let lost_frame = self.lost_list.pop().map(Ok);
                lost_frame.or_else(|| self.reader.next_frame().transpose())
            });

            for frame in frames.take(self.congestion.remaining_capacity()) {
                let data = frame?;
                self.socket.send(data.to_bytes())?;
                self.congestion.send(data);
            }

            if self.congestion.is_empty() {
                return Ok(());
            }

            let time_since_last_seen = self.last_seen.elapsed();
            if time_since_last_seen > self.conn_timeout {
                return Err(Box::new(io::Error::new(
                    ErrorKind::TimedOut,
                    format!("connection timed out after {time_since_last_seen:.3?}"),
                )));
            }

            let ack = self.recv_ack()?.and_then(|ack| {
                self.congestion
                    .mark_as_complete(ack.no())
                    .or_else(|| self.lost_list.mark_as_complete(ack.no()))
            });

            if let Some(sended_at) = ack {
                let rtt = sended_at.elapsed();
                self.rtt_estimator.update(rtt);
                self.last_seen = Instant::now();
            }

            let rto = self.rtt_estimator.rto();
            self.congestion.process_lost(rto);
            self.congestion
                .take_lost_frames_if_in_flight_is_empty(&mut self.lost_list);
        }
    }
}
