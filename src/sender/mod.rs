pub mod congestion;
pub mod rtt_estimator;
mod utils;

use std::{
    io::{self, ErrorKind, Read, Seek},
    iter,
    time::{Duration, Instant},
};

use crate::{
    Result,
    protocol::{AckFrame, DataFrameLike, ErrorFrame},
};
use crate::{
    file::{FileReader, FrameReader},
    socket::DatagramSocket,
};
use congestion::LostList;
use rtt_estimator::RttEstimator;

pub struct FileSender<T, S> {
    reader: FrameReader<T>,
    socket: S,

    lost_list: LostList,

    rtt_estimator: RttEstimator,
    outgoing: congestion::FlowControl,

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
            outgoing: congestion::FlowControl::new(10),
            last_seen: Instant::now(),
            conn_timeout: Duration::from_secs(7),
            recv_buf: vec![0; 1024],
        }
    }

    fn recv_ack(&mut self) -> Result<Option<AckFrame>> {
        let rto = self.rtt_estimator.rto();
        self.socket.set_read_timeout(Some(rto))?;

        let len = match self.socket.recv(&mut self.recv_buf) {
            Ok(len) if len < 8 => return Err("small frame".into()),
            Err(err) if matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                return Ok(None);
            }
            len => len,
        }?;

        let frame = &self.recv_buf[..len];

        if let Some(error) = ErrorFrame::parse_from(frame)? {
            return Err(error.into());
        }

        Ok(Some(AckFrame::from(frame)))
    }

    pub fn init_send(&mut self) -> Result<()> {
        loop {
            let frames = iter::from_fn(|| {
                let lost_frame = self.lost_list.pop().map(Ok);
                lost_frame.or_else(|| self.reader.next_frame().transpose())
            });

            for frame in frames.take(self.outgoing.remaining_capacity()) {
                let data = frame?;
                self.socket.send(data.as_bytes())?;
                self.outgoing.send(data);
            }

            if self.outgoing.is_empty() {
                return Ok(());
            }

            let time_since_last_seen = self.last_seen.elapsed();
            if time_since_last_seen > self.conn_timeout {
                let msg = format!("connection timed out after {time_since_last_seen:.3?}");
                return Err(io::Error::new(ErrorKind::TimedOut, msg).into());
            }

            let ack = self.recv_ack()?.and_then(|ack| {
                self.outgoing
                    .mark_as_complete(ack)
                    .or_else(|| self.lost_list.mark_as_complete(ack))
            });

            if let Some(sended_at) = ack {
                self.rtt_estimator.update(sended_at.elapsed());
                self.last_seen = Instant::now();
            }

            self.outgoing.process_lost(self.rtt_estimator.rto());
            self.outgoing
                .take_lost_frames_if_flight_is_empty(&mut self.lost_list);
        }
    }
}
