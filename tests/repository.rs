mod common;

#[test]
fn test_recipient_repository_crud() {
    let test_db = common::TestDb::new();
    let _pool = test_db.pool();
}
