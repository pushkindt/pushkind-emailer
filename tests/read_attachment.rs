use pushkind_emailer::utils::read_attachment_file;
use actix_multipart::form::tempfile::TempFile;
use tempfile::NamedTempFile;
use std::io::{Write, Seek, SeekFrom};

#[test]
fn read_attachment_file_returns_tuple() {
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "data").unwrap();
    file.as_file_mut().seek(SeekFrom::Start(0)).unwrap();
    let mut temp = TempFile {
        file,
        content_type: Some(mime::TEXT_PLAIN),
        file_name: Some("file.txt".into()),
        size: 4,
    };

    let (name, mime, data) = read_attachment_file(&mut temp).unwrap();
    assert_eq!(name.unwrap(), "file.txt");
    assert_eq!(mime.unwrap(), "text/plain");
    assert_eq!(data.unwrap(), b"data");
}
