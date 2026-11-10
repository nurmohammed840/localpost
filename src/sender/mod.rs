pub mod congestion;
pub mod rtt_estimator;
mod utils;

use crate::{
    file::{AsyncFrameReader, FileReader},
    protocol::{AckFrame, DataFrame, DataFrameLike, ErrorFrame},
    socket::AsyncDatagramSocket as DatagramSocket,
};
use anyhow::{Result, bail};
use congestion::LostList;
use rtt_estimator::RttEstimator;
use std::{
    io::{ErrorKind, Read, Seek},
    time::{Duration, Instant},
};

pub struct FileSender<S> {
    socket: S,
    reader: AsyncFrameReader,

    lost_list: LostList,

    rtt_estimator: RttEstimator,
    outgoing: congestion::FlowControl,

    last_seen: Instant,
    pub conn_timeout: Duration,

    recv_buf: Vec<u8>,
}

impl<S> FileSender<S>
where
    S: DatagramSocket,
{
    pub fn new<T>(reader: FileReader<T>, socket: S) -> Self
    where
        T: Seek + Read + Send + 'static,
    {
        Self {
            reader: AsyncFrameReader::new(reader, 8 * 1024),
            socket,
            lost_list: LostList::default(),
            rtt_estimator: RttEstimator::new(Duration::from_secs(3)),
            outgoing: congestion::FlowControl::new(10),
            last_seen: Instant::now(),
            conn_timeout: Duration::from_secs(7),
            recv_buf: vec![0; 1024],
        }
    }

    async fn recv_ack(&mut self) -> Result<Option<AckFrame>> {
        let rto = self.rtt_estimator.rto();
        self.socket.set_read_timeout(Some(rto))?;

        let len = match self.socket.recv(&mut self.recv_buf).await {
            Err(err) if matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                return Ok(None);
            }
            len => len,
        }?;

        let frame = &self.recv_buf[..len];
        ErrorFrame::check(frame)?;
        AckFrame::try_from(frame).map(Some)
    }

    async fn next_frame_to_send(&mut self) -> Result<Option<DataFrame>> {
        // Retransmit lost frames first
        match self.lost_list.pop() {
            Some(frame) => Ok(Some(frame)),
            None => self.reader.next_frame().await,
        }
    }

    pub async fn init_send(&mut self) -> Result<()> {
        loop {
            for _ in 0..self.outgoing.remaining_capacity() {
                let Some(data) = self.next_frame_to_send().await? else {
                    break;
                };

                self.socket.send(data.as_bytes()).await?;
                self.outgoing.send(data);
            }

            if self.outgoing.is_empty() {
                return Ok(());
            }

            let time_since_last_seen = self.last_seen.elapsed();
            if time_since_last_seen > self.conn_timeout {
                bail!("connection timed out after {time_since_last_seen:.3?}");
            }

            let ack = self.recv_ack().await;

            let is_fin_ack = ack
                .as_ref()
                .is_err_and(|err| err.downcast_ref().is_some_and(ErrorFrame::is_fin_ack));

            if is_fin_ack {
                return Ok(());
            }

            let rtts = ack?.map(|ack| {
                self.last_seen = Instant::now();
                self.outgoing
                    .mark_as_complete(ack)
                    .chain(self.lost_list.mark_as_complete(ack))
            });

            for rtt in rtts.into_iter().flatten() {
                self.rtt_estimator.update(rtt.elapsed());
            }

            self.outgoing.process_lost(self.rtt_estimator.rto());
            self.outgoing
                .take_lost_frames_if_flight_is_empty(&mut self.lost_list);
        }
    }
}
