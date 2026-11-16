use std::{fs::File, path::PathBuf, sync::Arc, time::Duration};

use crate::{
    frb_generated::StreamSink,
    resource::Res,
    runtime::{block_on, init_async_runtime, spawn_blocking},
};
use deps::anyhow::Result;
use flutter_rust_bridge::frb;
use localpost::{
    deps,
    endpoint::{Connection, Endpoint, FileRequest, Listener},
    file::FileReader,
    receiver::RecvStatus,
};

pub use localpost::protocol::{FileInfo, SystemInfo, TransferID};
use qrcode::{render::svg, QrCode};

#[frb(mirror(SystemInfo))]
pub struct _SystemInfo {
    pub os_name: String,
    pub os_version: String,
    pub os_type: String,
    pub architecture: String,
    pub hostname: String,
    pub port: u16,
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

#[frb(opaque)]
pub struct Server {
    listener: Res<Listener>,
}

impl Server {
    pub fn bind(port: u16) -> Result<(Server, String, String)> {
        block_on(async move || {
            let ip = local_ip_address::local_ip()?;

            let listener = match Listener::listen((ip, port)).await {
                Err(_) if port != 0 => Listener::listen((ip, 0)).await,
                res => res,
            }?;

            let addr = listener.tcp.local_addr()?.to_string();

            let svg = QrCode::new(addr.as_bytes())?
                .render::<svg::Color>()
                .quiet_zone(true)
                .build();

            Ok((listener.into(), addr, svg))
        })
    }

    pub fn run(self, sink: StreamSink<(ResConnection, SystemInfo)>) {
        block_on(async move || loop {
            let sent = match self.listener.accept().await {
                Ok((conn, info)) => sink.add((conn.into(), info)),
                Err(error) => sink.add_error(error),
            };

            if sent.is_err() {
                break;
            }
        })
    }
}

#[frb(opaque)]
pub struct ResConnection {
    conn: Res<Connection>,
}

impl ResConnection {
    pub fn handshake(self) -> Result<(ResEndpoint, String)> {
        block_on(async move || {
            let endpoint = self.conn.take().handshake().await?;
            let peer_addr = endpoint.peer_addr();
            Ok((endpoint.into(), peer_addr.to_string()))
        })
    }
}

#[frb(opaque)]
pub struct ResEndpoint {
    endpoint: Arc<Res<Endpoint>>,
}

impl ResEndpoint {
    pub fn open(
        addr: String,
        timeout_ms: u64,
        server_port: u16,
    ) -> Result<(ResEndpoint, SystemInfo)> {
        block_on(async move || {
            let (endpoint, info) =
                Endpoint::open_with_timeout(addr, Duration::from_millis(timeout_ms), server_port)
                    .await?;

            Ok((endpoint.into(), info))
        })
    }

    pub fn send_file_from_path(&self, path: String, frame_size: u16) -> Result<()> {
        let endpoint = self.endpoint.clone();
        let (reader, file_info) = FileReader::open(path, frame_size)?;

        block_on(async move || endpoint.send(reader, file_info).await)
    }

    pub fn send_file_from_fd(&self, fd: i32, name: String, frame_size: u16) -> Result<()> {
        use std::io::{Seek, SeekFrom};
        use std::os::fd::FromRawFd;

        // SAFETY: `fd` was detached on the Android side, so we are its sole owner,
        // and it is closed exactly once when `file` drops.
        let mut file = unsafe { File::from_raw_fd(fd) };
        file.seek(SeekFrom::Start(0))?;

        let (reader, file_info) =
            FileReader::from(file, name.into(), frame_size, FileReader::BUFFER_SIZE)?;

        let endpoint = self.endpoint.clone();
        block_on(async move || endpoint.send(reader, file_info).await)
    }

    pub fn accept_requests(&self, sink: StreamSink<ResFile>) {
        let endpoint = self.endpoint.clone();

        block_on(async move || {
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

    pub fn disconnect(&self) -> Result<()> {
        let endpoint = self.endpoint.clone();
        block_on(async move || endpoint.disconnect().await)
    }
}

#[frb(opaque)]
pub struct ResFile {
    req: Arc<Res<FileRequest>>,
}

impl ResFile {
    pub fn file_info(&self) -> Result<FileInfo> {
        let req = self.req.clone();
        block_on(async move || req.file_info().await)
    }

    pub fn receive(self, path: String, status: &ResRecvStatus) -> Result<()> {
        let status = status.inner.clone();

        block_on(async move || {
            let file = spawn_blocking(|| {
                File::options()
                    .truncate(false)
                    .create(true)
                    .write(true)
                    .open(path)
            })
            .await??;

            self.req.receive(file, status).await
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

    #[frb(sync)]
    pub fn total_and_recv_count(&self) -> (u32, u32) {
        self.inner.total_and_recv_count()
    }
}

// ---

pub fn pick_file(from_dir: Option<String>) -> Option<String> {
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        None // Android uses the Kotlin picker instead
    }
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        let mut dialog = rfd::FileDialog::new()
            .set_title("Select file to send")
            .set_show_hidden_files(true);

        let dir = from_dir
            .map(PathBuf::from)
            .filter(|p| p.is_dir())
            .or_else(dirs::download_dir)
            .or_else(dirs::home_dir);

        if let Some(path) = dir {
            dialog = dialog.set_directory(path);
        }

        dialog.pick_file().map(|p| p.to_string_lossy().into_owned())
    }
}

// ---

impl From<Listener> for Server {
    fn from(value: Listener) -> Self {
        Self {
            listener: value.into(),
        }
    }
}

impl From<Connection> for ResConnection {
    fn from(value: Connection) -> Self {
        Self { conn: value.into() }
    }
}

impl From<Endpoint> for ResEndpoint {
    fn from(value: Endpoint) -> Self {
        Self {
            endpoint: Arc::new(value.into()),
        }
    }
}

impl From<FileRequest> for ResFile {
    fn from(req: FileRequest) -> Self {
        Self {
            req: Arc::new(Res::from(req)),
        }
    }
}
