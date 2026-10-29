use std::{fs::File, sync::Arc};

use super::*;
use crate::{DynError, Result};

pub struct Endpoint {
    pub conn: Conn,
}

impl Endpoint {
    fn receive_file(&mut self, file_info: FileInfo, file: FileObj) -> Result<()> {
        let _writer = FileWriter::new(file, file_info.frame_size - 4);
        Ok(())
    }
}

pub fn run(port: u16, interface: Arc<dyn ServerInterface>) -> Result<()> {
    let mut listener = Listener::new(port)?;
    let mut id = 0;

    let mut connection_handler = || -> Result<()> {
        id += 1;

        let (info, mut endpoint) = listener.accept()?;

        let interface = interface.clone();

        let mut accept_conn =
            move |interface: &dyn ServerInterface, info: &SystemInfo| -> Result<()> {
                endpoint.conn.send(Hello::message())?;

                loop {
                    let file_info = endpoint.conn.next_req()?;
                    let Some(file) = interface.file_request(id, &info, &file_info)? else {
                        continue;
                    };
                    endpoint.receive_file(file_info, file)?;
                }
            };

        thread::spawn(move || {
            let should_accept = interface.new_connection(id, &info);
            if !should_accept {
                return;
            }
            let error = accept_conn(&*interface, &info).err();
            interface.connection_closed(id, error, info);
        });
        Ok(())
    };

    loop {
        if let Err(err) = connection_handler() {
            eprintln!("{err}")
        }
    }
}

pub trait WriteSeek: Write + Seek {}
impl<T: Write + Seek + ?Sized> WriteSeek for T {}

pub type FileObj = Box<dyn WriteSeek>;

pub trait ServerInterface: Send + Sync {
    // return `true` to accept connection
    fn new_connection(&self, _id: u32, info: &SystemInfo) -> bool {
        println!("New Connection: {info:#?}");
        true
    }

    // return `true` to accept File
    fn file_request(
        &self,
        _id: u32,
        _info: &SystemInfo,
        file_info: &FileInfo,
    ) -> Result<Option<FileObj>> {
        println!("write req: {file_info:#?}");

        let file = File::options()
            .truncate(false)
            .create(true)
            .write(true)
            .open("file.txt")?;

        Ok(Some(Box::new(file)))
    }

    fn connection_closed(&self, _id: u32, _: Option<DynError>, _: SystemInfo);
}
