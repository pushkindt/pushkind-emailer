//! Small utility helpers shared across the crate.
use std::io::Read;

use actix_multipart::form::tempfile::TempFile;

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

pub(crate) fn calculate_total_pages(total_items: usize, per_page: usize) -> usize {
    if per_page == 0 {
        return 0;
    }

    total_items.div_ceil(per_page)
}
