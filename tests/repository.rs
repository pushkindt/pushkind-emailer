//! Integration tests covering repository operations.
mod common;

use std::collections::HashMap;

use diesel::prelude::*;
use pushkind_emailer::domain::recipient::UpdateRecipient;
use pushkind_emailer::models::group::{Group as DbGroup, GroupRecipient, NewGroup};
use pushkind_emailer::models::hub::NewHub as DbNewHub;
use pushkind_emailer::models::recipient::{NewRecipient, Recipient as DbRecipient, RecipientField};
use pushkind_emailer::repository::{DieselRepository, RecipientWriter};
use pushkind_emailer::schema::{groups, groups_recipients, hubs, recipient_fields, recipients};

#[test]
fn update_recipient_is_atomic() {
    let test_db = common::TestDb::new();
    let pool = test_db.pool();
    let repo = DieselRepository::new(pool.clone());

    let (recipient_id, group_id) = {
        let mut conn = pool.get().expect("failed to get db connection");

        diesel::insert_into(hubs::table)
            .values(DbNewHub {
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
            })
            .execute(&mut conn)
            .expect("failed to insert hub");

        let group: DbGroup = diesel::insert_into(groups::table)
            .values(NewGroup {
                name: "group-1",
                hub_id: 1,
            })
            .get_result(&mut conn)
            .expect("failed to insert group");

        let recipient: DbRecipient = diesel::insert_into(recipients::table)
            .values(NewRecipient {
                name: "Old Name",
                email: "old@example.com",
                hub_id: 1,
            })
            .get_result(&mut conn)
            .expect("failed to insert recipient");

        diesel::insert_into(recipient_fields::table)
            .values(RecipientField {
                recipient_id: recipient.id,
                field: "city".to_string(),
                value: "Paris".to_string(),
            })
            .execute(&mut conn)
            .expect("failed to insert recipient field");

        diesel::insert_into(groups_recipients::table)
            .values(GroupRecipient {
                group_id: group.id,
                recipient_id: recipient.id,
            })
            .execute(&mut conn)
            .expect("failed to insert group/recipient link");

        (recipient.id, group.id)
    };

    let mut new_fields = HashMap::new();
    new_fields.insert("city".to_string(), "London".to_string());
    let invalid_group_id = 999_999;
    let update = UpdateRecipient::try_new(
        "New Name",
        "new@example.com",
        new_fields,
        vec![invalid_group_id],
    )
    .expect("failed to build UpdateRecipient");

    assert!(
        repo.update_recipient(recipient_id, &update).is_err(),
        "expected update_recipient to fail due to invalid group_id foreign key"
    );

    let mut conn = pool.get().expect("failed to get db connection");

    let db_recipient: DbRecipient = recipients::table
        .find(recipient_id)
        .select(DbRecipient::as_select())
        .first(&mut conn)
        .expect("failed to load recipient");
    assert_eq!(db_recipient.name, "Old Name");
    assert_eq!(db_recipient.email, "old@example.com");

    let db_fields: Vec<RecipientField> = recipient_fields::table
        .filter(recipient_fields::recipient_id.eq(recipient_id))
        .select(RecipientField::as_select())
        .load(&mut conn)
        .expect("failed to load recipient fields");
    assert_eq!(db_fields.len(), 1);
    assert_eq!(db_fields[0].field, "city");
    assert_eq!(db_fields[0].value, "Paris");

    let db_links: Vec<GroupRecipient> = groups_recipients::table
        .filter(groups_recipients::recipient_id.eq(recipient_id))
        .select(GroupRecipient::as_select())
        .load(&mut conn)
        .expect("failed to load group/recipient links");
    assert_eq!(db_links.len(), 1);
    assert_eq!(db_links[0].group_id, group_id);
}
