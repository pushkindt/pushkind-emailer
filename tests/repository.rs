//! Integration tests covering repository operations.
mod common;

use std::collections::BTreeMap;

use chrono::{Duration, NaiveDate, Utc};
use diesel::prelude::*;
use pushkind_emailer::domain::recipient::UpdateRecipient;
use pushkind_emailer::domain::types::{EmailId, HubId, RecipientEmail, RecipientId, RecipientName};
use pushkind_emailer::models::email::{
    Email as DbEmail, NewEmail as DbNewEmail, NewEmailRecipient as DbNewEmailRecipient,
};
use pushkind_emailer::models::group::{Group as DbGroup, GroupRecipient, NewGroup};
use pushkind_emailer::models::hub::NewHub as DbNewHub;
use pushkind_emailer::models::recipient::{NewRecipient, Recipient as DbRecipient, RecipientField};
use pushkind_emailer::repository::{DieselRepository, EmailReader, RecipientWriter};
use pushkind_emailer::schema::{
    email_recipients, emails, groups, groups_recipients, hubs, recipient_fields, recipients,
};

#[test]
fn update_recipient_is_atomic() {
    let test_db = common::TestDb::new();
    let pool = test_db.pool();
    let repo = DieselRepository::new(pool.clone());

    let (recipient_id, hub_id, group_id) = {
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

        (recipient.id, recipient.hub_id, group.id)
    };

    let mut new_fields = BTreeMap::new();
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
        repo.update_recipient(
            RecipientId::new(recipient_id).unwrap(),
            HubId::new(hub_id).unwrap(),
            &update
        )
        .is_err(),
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

fn insert_email(
    conn: &mut diesel::SqliteConnection,
    hub_id: i32,
    created_at: chrono::NaiveDateTime,
) -> i32 {
    let new_email = DbNewEmail {
        message: "Msg",
        created_at,
        is_sent: false,
        subject: None,
        attachment: None,
        attachment_name: None,
        attachment_mime: None,
        hub_id,
    };

    let email: DbEmail = diesel::insert_into(emails::table)
        .values(&new_email)
        .get_result(conn)
        .expect("failed to insert email");

    email.id
}

#[allow(clippy::too_many_arguments)]
fn insert_email_recipient(
    conn: &mut diesel::SqliteConnection,
    email_id: i32,
    address: &str,
    updated_at: chrono::NaiveDateTime,
    name: &str,
    fields: &str,
) {
    let new_recipient = DbNewEmailRecipient {
        email_id,
        address,
        opened: false,
        updated_at,
        is_sent: true,
        name,
        fields,
    };

    diesel::insert_into(email_recipients::table)
        .values(&new_recipient)
        .execute(conn)
        .expect("failed to insert email recipient");
}

#[test]
fn list_recent_email_recipients_returns_latest_snapshot() {
    let test_db = common::TestDb::new();
    let pool = test_db.pool();
    let repo = DieselRepository::new(pool.clone());

    let mut conn = pool.get().expect("failed to get db connection");

    diesel::insert_into(hubs::table)
        .values(vec![
            DbNewHub {
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
            },
            DbNewHub {
                id: 2,
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
            },
        ])
        .execute(&mut conn)
        .expect("failed to insert hubs");

    let created_1 = NaiveDate::from_ymd_opt(2024, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let created_2 = NaiveDate::from_ymd_opt(2024, 1, 2)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    let created_3 = NaiveDate::from_ymd_opt(2024, 1, 3)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();

    let email_1 = insert_email(&mut conn, 1, created_1);
    let email_2 = insert_email(&mut conn, 1, created_2);
    let email_3 = insert_email(&mut conn, 2, created_3);

    insert_email_recipient(
        &mut conn,
        email_1,
        "a@example.com",
        created_1,
        "Old A",
        r#"{"segment":"old"}"#,
    );
    insert_email_recipient(
        &mut conn,
        email_2,
        "a@example.com",
        created_2,
        "New A",
        r#"{"segment":"new"}"#,
    );
    insert_email_recipient(
        &mut conn,
        email_2,
        "b@example.com",
        created_2,
        "Bee",
        r#"{"segment":"bee"}"#,
    );
    insert_email_recipient(
        &mut conn,
        email_3,
        "a@example.com",
        created_3,
        "Hub2 A",
        r#"{"segment":"hub2"}"#,
    );

    let recipients = repo
        .list_recent_email_recipients(HubId::new(1).unwrap(), None)
        .expect("failed to list recipients");
    assert_eq!(recipients.len(), 2);

    let recipient_a = recipients
        .iter()
        .find(|recipient| recipient.address == RecipientEmail::new("a@example.com").unwrap())
        .expect("missing recipient a");
    assert_eq!(
        recipient_a.email_id,
        EmailId::new(email_2).expect("invalid email id")
    );

    assert_eq!(recipient_a.name, RecipientName::new("New A").unwrap());
    assert_eq!(
        recipient_a.fields.get("segment").map(String::as_str),
        Some("new")
    );

    let recipient_b = recipients
        .iter()
        .find(|recipient| recipient.address == RecipientEmail::new("b@example.com").unwrap())
        .expect("missing recipient b");
    assert_eq!(
        recipient_b.email_id,
        EmailId::new(email_2).expect("invalid email id")
    );
    assert_eq!(recipient_b.name, RecipientName::new("Bee").unwrap());
    assert_eq!(
        recipient_b.fields.get("segment").map(String::as_str),
        Some("bee")
    );

    let recipients_hub2 = repo
        .list_recent_email_recipients(HubId::new(2).unwrap(), None)
        .expect("failed to list hub2 recipients");
    assert_eq!(recipients_hub2.len(), 1);
    assert_eq!(
        recipients_hub2[0].address,
        RecipientEmail::new("a@example.com").unwrap()
    );
    assert_eq!(
        recipients_hub2[0].name,
        RecipientName::new("Hub2 A").unwrap()
    );
}

#[test]
fn list_recent_email_recipients_applies_number_of_days_filter() {
    let test_db = common::TestDb::new();
    let pool = test_db.pool();
    let repo = DieselRepository::new(pool.clone());

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

    let now = Utc::now().naive_utc();
    let recent = now - Duration::hours(12);
    let old = now - Duration::days(5);

    let email_old = insert_email(&mut conn, 1, old);
    let email_recent = insert_email(&mut conn, 1, recent);

    insert_email_recipient(&mut conn, email_old, "slow@example.com", old, "Slow", "{}");
    insert_email_recipient(
        &mut conn,
        email_recent,
        "fast@example.com",
        recent,
        "Fast",
        "{}",
    );

    let all = repo
        .list_recent_email_recipients(HubId::new(1).unwrap(), None)
        .expect("failed to list recipients");
    assert_eq!(all.len(), 2);

    let filtered = repo
        .list_recent_email_recipients(HubId::new(1).unwrap(), Some(3))
        .expect("failed to list recipients with filter");
    assert_eq!(filtered.len(), 1);
    assert_eq!(
        filtered[0].address,
        RecipientEmail::new("fast@example.com").unwrap()
    );
}
