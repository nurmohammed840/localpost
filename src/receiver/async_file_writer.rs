use crate::{
    file::FileWriter,
    protocol::{DataFrame, DataFrameLike},
};
use futures::{FutureExt, task::AtomicWaker};
use std::{
    future::poll_fn,
    io::{self, Seek, Write},
    mem,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    task::Poll,
};

pub struct AsyncFileWriter {
    tx: mpsc::Sender<DataFrame>,
    task: nio::JoinHandle<io::Result<()>>,
    shared: Arc<Shared>,
    max_pending: usize,
}

#[derive(Default)]
struct Shared {
    pending: AtomicUsize,
    waker: AtomicWaker,
}

impl Shared {
    fn release(&self, n: usize) {
        self.pending.fetch_sub(n, Ordering::Release);
        self.waker.wake();
    }
}

impl AsyncFileWriter {
    pub fn new<T>(mut file: FileWriter<T>, flush_threshold: usize) -> Self
    where
        T: Write + Seek + Send + 'static,
    {
        assert!(flush_threshold > 0);

        let shared = Arc::new(Shared::default());
        let pending = Arc::clone(&shared);

        let (tx, rx) = mpsc::channel::<DataFrame>();

        let task = move || {
            let mut buffered = 0;

            for frame in rx.iter() {
                buffered += frame.data().len();
                file.write(frame);

                if buffered >= flush_threshold {
                    file.flush()?;
                    pending.release(mem::take(&mut buffered));
                }
            }

            file.flush()
        };

        Self {
            max_pending: flush_threshold * 2,
            task: nio::spawn_blocking(task),
            shared,
            tx,
        }
    }

    fn has_room(&self) -> bool {
        self.shared.pending.load(Ordering::Acquire) < self.max_pending
    }

    pub async fn write(&mut self, frame: DataFrame) -> io::Result<()> {
        let len = frame.data().len();

        // increase before sending, so the writer can never subtract first.
        self.shared.pending.fetch_add(len, Ordering::Release);
        let _ = self.tx.send(frame);

        poll_fn(|cx| {
            if let Poll::Ready(result) = self.task.poll_unpin(cx) {
                result??;
                return Poll::Ready(Err(closed()));
            }

            if self.has_room() {
                return Poll::Ready(Ok(()));
            }

            self.shared.waker.register(cx.waker());

            // Need to check condition **after** `register` to avoid a race
            // condition that would result in lost notifications.
            if self.has_room() {
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            }
        })
        .await
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
