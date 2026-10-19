pub mod congestion;
pub mod rtt_estimator;

use std::{
    io::{ErrorKind, Read, Seek},
    net::UdpSocket,
    time::Duration,
};

use crate::file::{FileReader, FrameReader};
use crate::{Result, protocol::AckFrame};
use congestion::{Control, LostList};
use rtt_estimator::RttEstimator;

pub struct Sender<T> {
    reader: FrameReader<T>,
    socket: UdpSocket,

    lost_list: LostList,

    rtt_estimator: RttEstimator,
    congestion: Control,

    recv_buf: Vec<u8>,
}

impl<T> Sender<T>
where
    T: Seek + Read,
{
    pub fn new(reader: FileReader<T>, socket: UdpSocket) -> Self {
        Self {
            socket,
            reader: FrameReader::new(reader),
            lost_list: LostList::new(),
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
        let frames = (0..self.congestion.remaining_capacity())
            .map_while(|_| self.reader.next_data_frame().transpose());

        for frame in frames {
            let data = frame?;
            self.socket.send(data.to_bytes())?;
            self.congestion.send(data);
        }

        loop {
            if let Some(ack) = self.recv_ack()?
                && let Some(sended_at) = self.congestion.mark_as_complete(ack.no())
            {
                let rtt = sended_at.elapsed();
                self.rtt_estimator.update(rtt);
            }

            let rto = self.rtt_estimator.rto();
            self.congestion.process_lost(rto);

            let _a = self
                .congestion
                .take_lost_frames_if_in_flight_is_empty(&mut self.lost_list);

            break;
        }

        Ok(())
    }
}
