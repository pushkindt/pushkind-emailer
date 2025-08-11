use pushkind_emailer::domain::hub::Hub;

#[test]
fn unsubscribe_url_with_login() {
    let hub = Hub {
        id: 1,
        login: Some("user@example.com".to_string()),
        password: None,
        sender: None,
        smtp_server: None,
        smtp_port: None,
        created_at: None,
        updated_at: None,
        imap_server: None,
        imap_port: None,
        email_template: None,
    };
    assert_eq!(
        hub.unsubscribe_url(),
        "mailto:user@example.com?subject=unsubscribe"
    );
}

#[test]
fn unsubscribe_url_no_login() {
    let hub = Hub {
        id: 1,
        login: None,
        password: None,
        sender: None,
        smtp_server: None,
        smtp_port: None,
        created_at: None,
        updated_at: None,
        imap_server: None,
        imap_port: None,
        email_template: None,
    };
    assert_eq!(hub.unsubscribe_url(), "");
}
