use std::{fs::File, sync::Arc};

use crate::{
    frb_generated::StreamSink,
    resource::Res,
    runtime::{block_on, init_async_runtime, spawn_blocking},
};
use deps::anyhow::Result;
use derive_more::From;
use flutter_rust_bridge::frb;
use localpost::{
    deps,
    endpoint::{Endpoint, FileRequest},
    protocol::{FileInfo, TransferID},
    receiver::RecvStatus,
};

pub use localpost::protocol::SystemInfo;

#[frb(mirror(SystemInfo))]
pub struct _SystemInfo {
    pub os_name: String,
    pub os_version: String,
    pub os_type: String,
    pub architecture: String,
    pub hostname: String,
}

#[frb(mirror(TransferID))]
pub struct _TransferID {
    pub udp_port: u16,
    pub id: Option<u32>,
}

#[frb(mirror(FileInfo))]
pub struct _FileInfo {
    pub tid: TransferID,
    pub name: Vec<u8>,
    pub size: u64,
    pub frame_size: u16,
    pub last_modified: u64,
}

#[frb(init)]
pub fn init_app() {
    init_async_runtime();
}

pub fn get_local_ip() -> String {
    localpost::local_ip_address::local_ip()
        .ok()
        .map(|ip| ip.to_string())
        .unwrap_or_default()
}

#[frb(opaque)]
#[derive(From)]
#[from(Endpoint)]
pub struct ResEndpoint {
    endpoint: Res<Endpoint>,
}

impl ResEndpoint {
    pub fn open(addr: String) -> Result<(ResEndpoint, SystemInfo)> {
        block_on(async || {
            let (endpoint, info) = Endpoint::open(addr).await?;
            Ok((endpoint.into(), info))
        })
    }

    pub fn accept_requests(self, sink: StreamSink<ResFile>) {
        block_on(async move || {
            let mut endpoint = self.endpoint.take();
            while let Some(result) = endpoint.accept_request().await.transpose() {
                let sent = match result {
                    Ok(request) => sink.add(request.into()),
                    Err(error) => sink.add_error(error),
                };
                if sent.is_err() {
                    break;
                }
            }
        })
    }
}

#[frb(opaque)]
pub struct ResRecvStatus {
    inner: Arc<RecvStatus>,
}

impl ResRecvStatus {
    #[frb(sync)]
    pub fn new() -> Self {
        Self {
            inner: RecvStatus::new(),
        }
    }
    
}

#[frb(opaque)]
pub struct ResFile {
    req: Arc<Res<FileRequest>>,
}

impl From<FileRequest> for ResFile {
    fn from(req: FileRequest) -> Self {
        Self {
            req: Arc::new(Res::from(req)),
        }
    }
}

impl ResFile {
    pub fn file_info(&self) -> Result<FileInfo> {
        let req = self.req.clone();
        block_on(async move || req.file_info().await)
    }

    pub fn receive(self, path: String, status: ResRecvStatus) -> Result<()> {
        block_on(async move || {
            let file = spawn_blocking(|| {
                File::options()
                    .truncate(false)
                    .create(true)
                    .write(true)
                    .open(path)
            })
            .await??;

            self.req.receive(file, status.inner.clone()).await.unwrap();
            Ok(())
        })
    }
}
