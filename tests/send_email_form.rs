use pushkind_emailer::{forms::main::SendEmailForm, domain::email::NewEmail};
use actix_multipart::form::{text::Text, json::Json as MpJson, tempfile::TempFile};
use tempfile::NamedTempFile;
use std::io::{Write, Seek, SeekFrom};

#[test]
fn send_email_form_into_new_email_with_attachment() {
    let mut named = NamedTempFile::new().unwrap();
    write!(named, "hello").unwrap();
    named.as_file_mut().seek(SeekFrom::Start(0)).unwrap();
    let attachment = TempFile {
        file: named,
        content_type: Some(mime::TEXT_PLAIN),
        file_name: Some("hello.txt".into()),
        size: 5,
    };

    let form = SendEmailForm {
        message: Text("Hi".to_string()),
        subject: Text(Some("Sub".to_string())),
        attachment: Some(attachment),
        recipients: MpJson(vec!["a@example.com".to_string()]),
    };

    let email: NewEmail = form.into();

    assert_eq!(email.message, "Hi");
    assert_eq!(email.subject.as_deref(), Some("Sub"));
    assert_eq!(email.recipients, vec!["a@example.com".to_string()]);
    assert_eq!(email.attachment_name.as_deref(), Some("hello.txt"));
    assert_eq!(email.attachment_mime.as_deref(), Some("text/plain"));
    assert_eq!(email.attachment.as_deref().unwrap(), b"hello");
}
