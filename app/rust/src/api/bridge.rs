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

fn block_on<F, Fut>(future: F) -> Fut::Output
where
    F: FnOnce() -> Fut + Send,
    Fut: Future + 'static,
    Fut::Output: Send + 'static,
{
    futures::executor::block_on(RT.spawn_pinned_at(0, future)).unwrap()
}

#[flutter_rust_bridge::frb(init)]
pub fn init_app() {
    LazyLock::force(&RT);
}

pub fn get_local_ip() -> String {
    localpost::local_ip_address::local_ip()
        .ok()
        .map(|ip| ip.to_string())
        .unwrap_or_default()
}
