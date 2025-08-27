use actix_multipart::form::{json::Json as MpJson, tempfile::TempFile, text::Text};
use pushkind_common::domain::email::NewEmail;
use pushkind_emailer::forms::main::SendEmailForm;
use pushkind_emailer::repository::test::TestRepository;
use std::io::{Seek, SeekFrom, Write};
use tempfile::NamedTempFile;

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

    let email: NewEmail = form.to_new_email(1, &TestRepository {}).unwrap();

    assert_eq!(email.message, "Hi");
    assert_eq!(email.subject.as_deref(), Some("Sub"));
    assert_eq!(email.attachment_name.as_deref(), Some("hello.txt"));
    assert_eq!(email.attachment_mime.as_deref(), Some("text/plain"));
    assert_eq!(email.attachment.as_deref().unwrap(), b"hello");
}
