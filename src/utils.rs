use std::{error::Error, io::Read};

use actix_multipart::form::tempfile::TempFile;
use log::info;

use crate::models::config::ServerConfig;

/// Pushes a new email ID to the background delivery service over ZeroMQ.
///
/// The function establishes a [`zmq::PUSH`] socket using the address from
/// `zmq_config` and sends the big-endian byte representation of `id`.
/// If any ZeroMQ operation fails the underlying error is returned.
pub fn send_zmq_email_id(id: i32, zmq_config: &ServerConfig) -> Result<(), Box<dyn Error>> {
    let context = zmq::Context::new();
    let requester = context.socket(zmq::PUSH)?;
    requester.connect(&zmq_config.zmq_address)?;

    let buffer = id.to_be_bytes().to_vec();

    requester.send(buffer, 0)?;

    info!("Sent email id: {id}");

    Ok(())
}

/// Reads an uploaded attachment into memory returning metadata and contents.
///
/// # Parameters
/// * `attachment` - Temporary file created by `actix-multipart` to read from.
///
/// # Returns
/// A tuple `(file_name, mime_type, data)` where:
/// - `file_name` is the original file name if provided.
/// - `mime_type` is the MIME type sent with the upload if present.
/// - `data` contains the file bytes.
///
/// Any I/O error while reading the file is propagated to the caller.
#[allow(clippy::type_complexity)]
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
