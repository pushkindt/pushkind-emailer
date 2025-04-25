use std::{error::Error, io::Read};

use actix_multipart::form::tempfile::TempFile;
use log::info;

use crate::models::config::ServerConfig;

pub fn send_zmq_email_id(id: i32, zmq_config: &ServerConfig) -> Result<(), Box<dyn Error>> {
    let context = zmq::Context::new();
    let requester = context.socket(zmq::PUSH)?;
    requester.connect(&zmq_config.zmq_address)?;

    let buffer = id.to_be_bytes().to_vec();

    requester.send(buffer, 0)?;

    info!("Sent email id: {}", id);

    Ok(())
}

pub fn read_attachment_file(
    attachment: &mut TempFile,
) -> std::io::Result<(Option<String>, Option<String>, Option<Vec<u8>>)> {
    let mut buf = Vec::new();
    attachment.file.read_to_end(&mut buf)?; // propagate error properly

    let file_name = attachment.file_name.clone();
    let file_mime = attachment
        .content_type
        .clone()
        .map(|ct| ct.essence_str().to_string());

    Ok((file_name, file_mime, Some(buf)))
}
