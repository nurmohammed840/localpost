use localpost::deps::{futures, nio};
use std::{
    future::Future,
    sync::{Arc, LazyLock},
};

static RT: LazyLock<Arc<nio::RuntimeContext>> = LazyLock::new(|| {
    nio::RuntimeBuilder::new()
        .worker_threads(1)
        .rt()
        .unwrap()
        .context()
});

pub fn block_on<F, Fut>(future: F) -> Fut::Output
where
    F: FnOnce() -> Fut + Send,
    Fut: Future + 'static,
    Fut::Output: Send + 'static,
{
    futures::executor::block_on(spawn_pinned(future)).unwrap()
}

pub fn spawn_pinned<F, Fut>(future: F) -> nio::JoinHandle<Fut::Output>
where
    F: FnOnce() -> Fut + Send,
    Fut: Future + 'static,
    Fut::Output: Send + 'static,
{
    RT.spawn_pinned_at(0, future)
}

pub fn init_async_runtime() {
    LazyLock::force(&RT);
}