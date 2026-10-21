pub mod congestion;
pub mod rtt_estimator;
mod utils;

use std::{
    io::{ErrorKind, Read, Seek},
    iter,
    time::Duration,
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

    recv_buf: Vec<u8>,
}

impl<T, S> FileSender<T, S>
where
    T: Seek + Read,
    S: DatagramSocket,
{
    pub fn new(reader: FileReader<T>, socket: S) -> Self {
        Self {
            socket,
            reader: FrameReader::new(reader),
            lost_list: LostList::default(),
            rtt_estimator: RttEstimator::new(Duration::from_secs(3)),
            congestion: Control::new(10),
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

            let ack = self.recv_ack()?.and_then(|ack| {
                self.congestion
                    .mark_as_complete(ack.no())
                    .or_else(|| self.lost_list.mark_as_complete(ack.no()))
            });

            if let Some(sended_at) = ack {
                let rtt = sended_at.elapsed();
                self.rtt_estimator.update(rtt);
            }

            let rto = self.rtt_estimator.rto();
            self.congestion.process_lost(rto);
            self.congestion
                .take_lost_frames_if_in_flight_is_empty(&mut self.lost_list);
        }
    }
}
