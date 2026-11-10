use crate::{file::FrameReader, protocol::DataFrame};
use anyhow::Result;
use std::io::{Read, Seek};

pub struct AsyncFrameReader {
    rx: flume::Receiver<DataFrame>,
    task: Option<nio::JoinHandle<Result<()>>>,
}

impl AsyncFrameReader {
    pub fn new<T>(mut frames: FrameReader<T>, prefetch: usize) -> Self
    where
        T: Read + Seek + Send + 'static,
    {
        assert!(prefetch > 0); // bounded(0) is a rendezvous channel: no read-ahead

        let (tx, rx) = flume::bounded(prefetch);

        let task = move || {
            while let Some(frame) = frames.next_frame()? {
                if tx.send(frame).is_err() {
                    break;
                }
            }
            Ok(())
        };

        Self {
            rx,
            task: Some(nio::spawn_blocking(task)),
        }
    }

    pub async fn next_frame(&mut self) -> Result<Option<DataFrame>> {
        if let Ok(frame) = self.rx.recv_async().await {
            return Ok(Some(frame));
        }

        // Disconnected: Read Error
        if let Some(task) = self.task.take() {
            task.await??;
        }

        Ok(None)
    }
}
