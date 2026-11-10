use crate::{file::FileWriter, protocol::DataFrame};
use std::io::{self, Seek, Write};

pub struct AsyncFileWriter {
    tx: flume::Sender<DataFrame>,
    task: nio::JoinHandle<io::Result<()>>,
}

impl AsyncFileWriter {
    pub fn new<T>(mut file: FileWriter<T>, flush_threshold: usize) -> Self
    where
        T: Write + Seek + Send + 'static,
    {
        assert!(flush_threshold > 0);

        let cap = flush_threshold.div_ceil(file.frame_size().into());
        let (tx, rx) = flume::bounded(cap);

        let task = move || {
            for frame in rx.iter() {
                file.write(frame);

                if file.total_bytes() >= flush_threshold {
                    file.flush()?;
                }
            }

            file.flush()
        };

        Self {
            task: nio::spawn_blocking(task),
            tx,
        }
    }

    pub async fn write(&mut self, frame: DataFrame) -> io::Result<()> {
        if self.tx.send_async(frame).await.is_ok() {
            return Ok(());
        }

        // Writer thread is gone: Read Error
        (&mut self.task).await??;

        Err(closed())
    }

    pub async fn finish(self) -> io::Result<()> {
        let Self { tx, task, .. } = self;
        drop(tx); // closes the channel
        task.await?
    }
}

fn closed() -> io::Error {
    io::Error::new(io::ErrorKind::BrokenPipe, "file writer closed")
}
