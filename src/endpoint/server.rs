use std::{fs::File, sync::Arc};

use super::*;
use crate::{DynError, Result, receiver::RecvStatus};

pub struct Endpoint {
    pub conn: Conn,
}

impl Endpoint {
    fn receive_file(
        &mut self,
        file_info: FileInfo,
        file: FileObj,
        status: Arc<RecvStatus>,
    ) -> Result<()> {
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
            if let Err(err) = receiver.init_receive(status) {
                if let Some(proto_error) = err.downcast_ref::<ErrorFrame>() {
                    println!("protocol error: {proto_error:#?}");
                } else {
                    let _ = receiver.send_error(err.to_string());
                }
            }
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
                let status = RecvStatus::new();
                let Some(file) = interface.file_request(id, &info, &file_info, status.clone())?
                else {
                    continue;
                };
                endpoint.receive_file(file_info, file, status)?;
            }
        };

        thread::spawn(move || {
            let should_accept = interface.new(id, &info);
            if !should_accept {
                return;
            }
            let error = accept_conn(&*interface, &info).err();
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

pub trait Connection: Send + Sync {
    // return `true` to accept connection
    fn new(&self, _id: u32, info: &SystemInfo) -> bool {
        println!("New Connection: {info:#?}");
        true
    }

    // return `true` to accept File
    fn file_request(
        &self,
        _id: u32,
        _info: &SystemInfo,
        file_info: &FileInfo,
        _status: Arc<RecvStatus>,
    ) -> Result<Option<FileObj>> {
        println!("write req: {file_info:#?}");

        let file = File::options()
            .truncate(false)
            .create(true)
            .write(true)
            .open("file.txt")?;

        Ok(Some(Box::new(file)))
    }

    fn closed(&self, _id: u32, _: Option<DynError>, _: SystemInfo);
}
