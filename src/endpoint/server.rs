use std::{fs::File, sync::Arc};

use super::*;
use crate::{Result, receiver::RecvStatus};

pub struct Endpoint {
    pub conn: Conn,
}

impl Endpoint {
    fn receive_file(&mut self, file_info: FileInfo, acceptor: FileAcceptor) -> Result<()> {
        let FileAcceptor {
            file,
            status,
            callback,
        } = acceptor;

        let writer = FileWriter::new(file, file_info.frame_size - 4);

        let sender_socket_addr = (self.conn.peer_addr.ip(), file_info.tid.udp_port);
        let receiver_socket = self.conn.create_udp_socket()?;

        receiver_socket.connect(sender_socket_addr)?;

        self.conn.send(TransferID {
            udp_port: receiver_socket.local_addr()?.port(),
            file_id: file_info.tid.file_id,
        })?;

        let mut receiver = FileReceiver::new(writer, receiver_socket, &file_info)?;

        thread::spawn(move || {
            let result = receiver.init_receive(status);

            if let Err(err) = &result
                && !err.is::<ErrorFrame>()
            {
                let _ = receiver.send_error(err.to_string());
            }

            callback(result);
        });
        Ok(())
    }
}

pub fn run(port: u16, interface: Arc<dyn Connection>) -> Result<()> {
    let mut listener = Listener::new(port)?;
    let mut id = 0;

    let mut connection_handler = || -> Result<()> {
        id += 1;

        let (info, mut endpoint) = listener.accept()?;
        let interface = interface.clone();

        let mut accept_conn = move |interface: &dyn Connection, info: &SystemInfo| -> Result<()> {
            endpoint.conn.send(Hello::message())?;

            loop {
                let file_info = endpoint.conn.next_req()?;

                let Some(acceptor) = interface.file_request(id, info, &file_info)? else {
                    continue;
                };

                endpoint.receive_file(file_info, acceptor)?;
            }
        };

        thread::spawn(move || {
            let should_accept = interface.new_connection(id, &info);
            if !should_accept {
                return;
            }
            let error = accept_conn(&*interface, &info);
            interface.closed(id, error, info);
        });
        Ok(())
    };

    loop {
        if let Err(err) = connection_handler() {
            eprintln!("{err}")
        }
    }
}

pub trait WriteSeek: Write + Seek + Send + Sync {}
impl<T: Write + Seek + ?Sized + Send + Sync> WriteSeek for T {}
pub type FileObj = Box<dyn WriteSeek>;

pub type OnCompleteCallback = Box<dyn FnOnce(Result<()>) + Send + Sync>;

pub struct FileAcceptor {
    file: FileObj,
    status: Arc<RecvStatus>,
    callback: OnCompleteCallback,
}

impl FileAcceptor {
    fn new<W, CB>(file: W, cb: CB) -> FileAcceptor
    where
        W: Write + Seek + Send + Sync + 'static,
        CB: FnOnce(Result<()>) + Send + Sync + 'static,
    {
        Self {
            file: Box::new(file),
            callback: Box::new(cb),
            status: RecvStatus::new(),
        }
    }

    pub fn status(&self) -> Arc<RecvStatus> {
        self.status.clone()
    }
}

pub trait Connection: Send + Sync {
    // return `true` to accept connection
    fn new_connection(&self, _conn_id: u32, info: &SystemInfo) -> bool {
        println!("New Connection: {info:#?}");
        true
    }

    // return `true` to accept File
    fn file_request(
        &self,
        _conn_id: u32,
        _info: &SystemInfo,
        file_info: &FileInfo,
    ) -> Result<Option<FileAcceptor>> {
        println!("write req: {file_info:#?}");

        let file = File::options()
            .truncate(false)
            .create(true)
            .write(true)
            .open("file.txt")?;

        let acceptor = FileAcceptor::new(file, |res| {
            if let Err(err) = res {
                eprintln!("{err}")
            }
        });

        Ok(Some(acceptor))
    }

    fn closed(&self, _conn_id: u32, _: Result<()>, _: SystemInfo);
}
