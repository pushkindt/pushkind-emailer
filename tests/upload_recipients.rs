use actix_multipart::form::tempfile::TempFile;
use pushkind_emailer::forms::recipients::UploadRecipientsForm;
use std::collections::HashMap;
use std::io::{Seek, SeekFrom, Write};
use tempfile::NamedTempFile;

#[test]
fn upload_recipients_parse() {
    let csv_content = "name,email,groups,foo\nJohn,john@example.com,group1,bar\nJane,jane@example.com,group1,bar2";
    let mut file = NamedTempFile::new().unwrap();
    write!(file, "{}", csv_content).unwrap();
    file.as_file_mut().seek(SeekFrom::Start(0)).unwrap();
    let temp = TempFile {
        file,
        content_type: None,
        file_name: Some("r.csv".into()),
        size: csv_content.len(),
    };

    let mut form = UploadRecipientsForm { csv: temp };
    let res = form.parse(42).unwrap();
    assert_eq!(res.len(), 2);
    assert_eq!(res[0].name, "John");
    assert_eq!(res[0].email, "john@example.com");
    assert_eq!(res[0].hub_id, 42);
    assert_eq!(res[0].groups.as_ref().unwrap(), &vec!["group1".to_string()]);
    let mut fields = HashMap::new();
    fields.insert("foo".to_string(), "bar".to_string());
    assert_eq!(res[0].fields.as_ref().unwrap(), &fields);

    assert_eq!(res[1].name, "Jane");
    assert_eq!(res[1].email, "jane@example.com");
    assert_eq!(res[1].groups.as_ref().unwrap(), &vec!["group1".to_string()]);
    let mut fields2 = HashMap::new();
    fields2.insert("foo".to_string(), "bar2".to_string());
    assert_eq!(res[1].fields.as_ref().unwrap(), &fields2);
}
