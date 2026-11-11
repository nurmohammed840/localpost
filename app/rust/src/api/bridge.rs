use std::{cell::RefCell, sync::Arc};

use crate::{
    frb_generated::StreamSink,
    resource::Res,
    runtime::{block_on, init_async_runtime},
};
use deps::anyhow::Result;
use derive_more::From;
use flutter_rust_bridge::frb;
use localpost::{
    deps,
    endpoint::{Endpoint, FileRequest},
};

pub use localpost::protocol::SystemInfo;

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

#[frb(mirror(SystemInfo))]
pub struct _SystemInfo {
    pub os_name: String,
    pub os_version: String,
    pub os_type: String,
    pub architecture: String,
    pub hostname: String,
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
    pub fn file_info(&self) -> Result<()> {
        let req = self.req.clone();
        block_on(async move || {
            // ...
            Ok(())
        })
    }
}

// pub fn run_server(port: u16, sink: StreamSink<(EndpointID, SystemInfo)>) {
//     block_on(async move || {
//         if let Err(err) = _run_server(port, &sink).await {
//             let _ = sink.add_error(err);
//         }
//     });
// }

// async fn _run_server(port: u16, sink: &StreamSink<(EndpointID, SystemInfo)>) -> Result<()> {
//     let mut listener = Listener::new(port).await?;
//     loop {
//         let (conn, _info) = match listener.accept().await {
//             Ok(conn) => conn,
//             Err(error) => {
//                 if sink.add_error(error).is_err() {
//                     break Ok(());
//                 }
//                 continue;
//             }
//         };

//         nio::spawn_local(async move {
//             let mut _endpoint = conn.handshake().await.unwrap();
//             // how to get `sink` here?
//         });
//     }
// }

// async fn handle_endpoint(mut endpoint: Endpoint, sink: StreamSink<FileEvent>) -> Result<()> {
//     while let Some(mut request) = endpoint.accept_request().await? {
//         nio::spawn_local(async {
//             let info = request.file_info().await;
//             handle_file(request).await;
//         });
//     }
//     Ok(())
// }

// async fn handle_file(mut request: FileRequest) -> Result<()> {
//     Ok(())
// }
