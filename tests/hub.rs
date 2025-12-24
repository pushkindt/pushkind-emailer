//! Integration tests covering hub-related repository and service behavior.
use pushkind_emailer::domain::hub::Hub;

#[test]
fn unsubscribe_url_with_login() {
    let hub = Hub::try_new(
        1,
        Some("user@example.com".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        0,
    )
    .unwrap();
    assert_eq!(
        hub.unsubscribe_url(),
        "mailto:user@example.com?subject=unsubscribe"
    );
}

#[test]
fn unsubscribe_url_no_login() {
    let hub = Hub::try_new(
        1, None, None, None, None, None, None, None, None, None, None, 0,
    )
    .unwrap();
    assert_eq!(hub.unsubscribe_url(), "");
}
