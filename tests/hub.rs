use pushkind_emailer::domain::hub::Hub;
use pushkind_emailer::domain::types::{HubId, HubLogin, ImapUid};

#[test]
fn unsubscribe_url_with_login() {
    let hub = Hub {
        id: HubId::try_from(1).unwrap(),
        login: Some(HubLogin::try_from("user@example.com").unwrap()),
        password: None,
        sender: None,
        smtp_server: None,
        smtp_port: None,
        created_at: None,
        updated_at: None,
        imap_server: None,
        imap_port: None,
        email_template: None,
        imap_last_uid: ImapUid::try_from(0).unwrap(),
    };
    assert_eq!(
        hub.unsubscribe_url(),
        "mailto:user@example.com?subject=unsubscribe"
    );
}

#[test]
fn unsubscribe_url_no_login() {
    let hub = Hub {
        id: HubId::try_from(1).unwrap(),
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
        imap_last_uid: ImapUid::try_from(0).unwrap(),
    };
    assert_eq!(hub.unsubscribe_url(), "");
}
