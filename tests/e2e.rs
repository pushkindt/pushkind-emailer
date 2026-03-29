use chrono::{NaiveDate, NaiveDateTime};
use diesel::prelude::*;
use reqwest::{StatusCode, header, multipart};
use serde_json::Value;

mod common;

use pushkind_emailer::{
    models::{
        email::{
            Email as DbEmail, EmailRecipient as DbEmailRecipient, NewEmail as DbNewEmail,
            NewEmailRecipient as DbNewEmailRecipient,
        },
        group::Group as DbGroup,
        hub::{Hub as DbHub, NewHub as DbNewHub},
        recipient::{NewRecipient as DbNewRecipient, Recipient as DbRecipient},
    },
    schema::{email_recipients, emails, groups, hubs, recipients, unsubscribes},
};

struct SeededEmail {
    email_id: i32,
    unopened_recipient_id: i32,
}

async fn response_json(response: reqwest::Response) -> Value {
    let body = response
        .text()
        .await
        .expect("Response body should be readable.");
    serde_json::from_str(&body).expect("Response body should be valid JSON.")
}

async fn assert_html_page(client: &reqwest::Client, url: String, expected_title: &str) {
    let response = client
        .get(url)
        .send()
        .await
        .expect("Failed to request HTML page.");

    assert_eq!(response.status(), StatusCode::OK);
    let html = response
        .text()
        .await
        .expect("HTML response should be readable.");
    assert!(html.contains(expected_title));
}

fn form_body(fields: Vec<(impl Into<String>, impl Into<String>)>) -> String {
    let fields = fields
        .into_iter()
        .map(|(key, value)| (key.into(), value.into()))
        .collect::<Vec<(String, String)>>();
    serde_html_form::to_string(&fields).expect("Form body should serialize.")
}

fn timestamp(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
) -> NaiveDateTime {
    NaiveDate::from_ymd_opt(year, month, day)
        .expect("date should be valid")
        .and_hms_opt(hour, minute, second)
        .expect("time should be valid")
}

fn seed_hub(app: &common::TestApp) {
    let mut conn = app
        .db_pool()
        .get()
        .expect("Failed to get SQLite connection from pool.");

    diesel::insert_into(hubs::table)
        .values(DbNewHub {
            id: common::HUB_ID,
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
        .expect("Failed to insert hub.");
}

fn seed_email_history_and_unsubscribe(app: &common::TestApp) -> SeededEmail {
    let mut conn = app
        .db_pool()
        .get()
        .expect("Failed to get SQLite connection from pool.");

    let optout_recipient: DbRecipient = diesel::insert_into(recipients::table)
        .values(DbNewRecipient {
            name: "Opt Out",
            email: "optout@example.com",
            hub_id: common::HUB_ID,
        })
        .get_result(&mut conn)
        .expect("Failed to insert unsubscribed recipient.");

    diesel::insert_into(unsubscribes::table)
        .values((
            unsubscribes::email.eq("optout@example.com"),
            unsubscribes::hub_id.eq(common::HUB_ID),
            unsubscribes::reason.eq(Some("manual opt-out")),
            unsubscribes::created_at.eq(timestamp(2026, 3, 5, 8, 0, 0)),
            unsubscribes::updated_at.eq(timestamp(2026, 3, 5, 8, 0, 0)),
        ))
        .execute(&mut conn)
        .expect("Failed to insert unsubscribe record.");

    let email: DbEmail = diesel::insert_into(emails::table)
        .values(DbNewEmail {
            message: "<p>Seeded email</p>",
            created_at: timestamp(2026, 3, 6, 9, 30, 0),
            is_sent: true,
            subject: Some("Seeded Subject"),
            attachment: None,
            attachment_name: None,
            attachment_mime: None,
            hub_id: common::HUB_ID,
        })
        .get_result(&mut conn)
        .expect("Failed to insert seeded email.");

    let unopened_recipient: DbEmailRecipient = diesel::insert_into(email_recipients::table)
        .values(DbNewEmailRecipient {
            email_id: email.id,
            address: "fresh@example.com",
            opened: false,
            updated_at: timestamp(2026, 3, 6, 9, 31, 0),
            is_sent: true,
            name: "Fresh User",
            fields: r#"{"segment":"new"}"#,
        })
        .get_result(&mut conn)
        .expect("Failed to insert unopened email recipient.");

    let _opened_recipient: DbEmailRecipient = diesel::insert_into(email_recipients::table)
        .values(DbNewEmailRecipient {
            email_id: email.id,
            address: optout_recipient.email.as_str(),
            opened: true,
            updated_at: timestamp(2026, 3, 6, 9, 32, 0),
            is_sent: true,
            name: optout_recipient.name.as_str(),
            fields: "{}",
        })
        .get_result(&mut conn)
        .expect("Failed to insert opened email recipient.");

    DbEmail::recalc_email_stats(&mut conn, email.id).expect("Email stats should recalculate.");

    SeededEmail {
        email_id: email.id,
        unopened_recipient_id: unopened_recipient.id,
    }
}

fn recipient_id_by_email(app: &common::TestApp, email: &str) -> i32 {
    let mut conn = app
        .db_pool()
        .get()
        .expect("Failed to get SQLite connection from pool.");
    recipients::table
        .filter(recipients::hub_id.eq(common::HUB_ID))
        .filter(recipients::email.eq(email))
        .select(DbRecipient::as_select())
        .first::<DbRecipient>(&mut conn)
        .expect("Recipient should exist.")
        .id
}

fn group_id_by_name(app: &common::TestApp, name: &str) -> i32 {
    let mut conn = app
        .db_pool()
        .get()
        .expect("Failed to get SQLite connection from pool.");
    groups::table
        .filter(groups::hub_id.eq(common::HUB_ID))
        .filter(groups::name.eq(name))
        .select(DbGroup::as_select())
        .first::<DbGroup>(&mut conn)
        .expect("Group should exist.")
        .id
}

#[ignore = "local-only end-to-end test"]
#[actix_web::test]
async fn test_emailer_logged_out_user_is_redirected_to_auth() {
    let app = common::spawn_app().await;
    let client = common::build_no_redirect_client();

    let response = client
        .get(format!("{}/", app.address()))
        .send()
        .await
        .expect("Failed to request Emailer index.");

    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .expect("Redirect location should be present.");
    assert!(location.starts_with("https://users.pushkind.test/auth/signin?next="));
}

#[ignore = "local-only end-to-end test"]
#[actix_web::test]
async fn test_emailer_admin_full_management_story() {
    let app = common::spawn_app().await;
    seed_hub(&app);
    let seeded = seed_email_history_and_unsubscribe(&app);
    let client = common::build_reqwest_client();

    common::login_as(
        &client,
        app.address(),
        "admin@example.com",
        "Emailer Admin",
        common::HUB_ID,
        &["emailer", "admin"],
    )
    .await;

    for (path, title) in [
        ("/", "<title>Emailer</title>"),
        ("/recipients", "<title>Emailer Recipients</title>"),
        ("/groups", "<title>Emailer Groups</title>"),
        ("/unsubscribed", "<title>Emailer Unsubscribed</title>"),
        ("/history", "<title>Emailer History</title>"),
        ("/settings", "<title>Emailer Settings</title>"),
    ] {
        assert_html_page(&client, format!("{}{}", app.address(), path), title).await;
    }

    let iam_response = client
        .get(format!("{}/api/v1/iam", app.address()))
        .send()
        .await
        .expect("Failed to request IAM payload.");

    assert_eq!(iam_response.status(), StatusCode::OK);
    let iam_payload = response_json(iam_response).await;
    assert_eq!(iam_payload["current_user"]["email"], "admin@example.com");
    assert!(
        iam_payload["local_menu_items"]
            .as_array()
            .expect("Local menu items should be present.")
            .iter()
            .any(|item| item["url"] == "/settings")
    );

    let add_recipient_response = client
        .post(format!("{}/recipient/add", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("name", "Manual User"),
            ("email", "manual@example.com"),
        ]))
        .send()
        .await
        .expect("Failed to add recipient.");

    assert_eq!(add_recipient_response.status(), StatusCode::OK);

    let upload_response = client
        .post(format!("{}/recipients/upload", app.address()))
        .multipart(
            multipart::Form::new().part(
                "csv",
                multipart::Part::bytes(
                    b"name,email,groups,plan\nUpload User,upload@example.com,CSV Group,pro\nSkipped,,,\n"
                        .to_vec(),
                )
                .file_name("recipients.csv"),
            ),
        )
        .send()
        .await
        .expect("Failed to upload recipients CSV.");

    assert_eq!(upload_response.status(), StatusCode::OK);

    let source_response = client
        .post(format!("{}/recipients/source", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![(
            "source",
            format!("{}/test/recipient-source", app.address()),
        )]))
        .send()
        .await
        .expect("Failed to import recipients from source.");

    assert_eq!(source_response.status(), StatusCode::OK);

    let manual_recipient_id = recipient_id_by_email(&app, "manual@example.com");
    let upload_recipient_id = recipient_id_by_email(&app, "upload@example.com");
    let source_recipient_id = recipient_id_by_email(&app, "source@example.com");
    let source_group_id = group_id_by_name(&app, "Source Group");

    let recipients_response = client
        .get(format!("{}/api/v1/recipients", app.address()))
        .send()
        .await
        .expect("Failed to request recipients API.");

    assert_eq!(recipients_response.status(), StatusCode::OK);
    let recipients_payload = response_json(recipients_response).await;
    assert_eq!(
        recipients_payload["crm_service_url"],
        "https://crm.pushkind.test"
    );
    let recipient_items = recipients_payload["recipients"]["items"]
        .as_array()
        .expect("Recipient list should be present.");
    assert!(
        recipient_items
            .iter()
            .any(|item| item["email"] == "manual@example.com")
    );
    assert!(
        recipient_items
            .iter()
            .any(|item| item["email"] == "upload@example.com")
    );
    assert!(
        recipient_items
            .iter()
            .any(|item| item["email"] == "source@example.com")
    );

    let add_group_response = client
        .post(format!("{}/group/add", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![("name", "Broadcast")]))
        .send()
        .await
        .expect("Failed to add group.");

    assert_eq!(add_group_response.status(), StatusCode::OK);
    let broadcast_group_id = group_id_by_name(&app, "Broadcast");

    let assign_group_response = client
        .post(format!(
            "{}/group/{}/assign",
            app.address(),
            broadcast_group_id
        ))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("recipient_id", manual_recipient_id.to_string()),
            ("recipient_id", upload_recipient_id.to_string()),
        ]))
        .send()
        .await
        .expect("Failed to assign recipients to group.");

    assert_eq!(assign_group_response.status(), StatusCode::OK);

    let groups_response = client
        .get(format!("{}/api/v1/groups", app.address()))
        .send()
        .await
        .expect("Failed to request groups API.");

    assert_eq!(groups_response.status(), StatusCode::OK);
    let groups_payload = response_json(groups_response).await;
    let group_items = groups_payload["groups"]
        .as_array()
        .expect("Group list should be present.");
    assert!(group_items.iter().any(|item| item["name"] == "Broadcast"));
    assert!(
        group_items
            .iter()
            .any(|item| item["name"] == "Source Group")
    );
    assert!(
        groups_payload["custom_fields"]
            .as_array()
            .expect("Custom fields list should be present.")
            .iter()
            .any(|field| field == "plan")
    );
    assert!(
        groups_payload["custom_fields"]
            .as_array()
            .expect("Custom fields list should be present.")
            .iter()
            .any(|field| field == "region")
    );

    let group_modal_response = client
        .get(format!(
            "{}/api/v1/groups/{}",
            app.address(),
            broadcast_group_id
        ))
        .send()
        .await
        .expect("Failed to request group modal.");

    assert_eq!(group_modal_response.status(), StatusCode::OK);
    let group_modal_payload = response_json(group_modal_response).await;
    assert_eq!(group_modal_payload["group"]["name"], "Broadcast");
    let assigned_recipients = group_modal_payload["recipients"]
        .as_array()
        .expect("Assigned recipients should be present.");
    assert_eq!(assigned_recipients.len(), 2);
    assert!(
        assigned_recipients
            .iter()
            .any(|recipient| recipient["id"] == manual_recipient_id)
    );
    assert!(
        assigned_recipients
            .iter()
            .any(|recipient| recipient["id"] == upload_recipient_id)
    );

    let save_recipient_response = client
        .post(format!(
            "{}/recipient/{}/save",
            app.address(),
            manual_recipient_id
        ))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("name", "Manual User Updated".to_string()),
            ("email", "manual@example.com".to_string()),
            ("groups", broadcast_group_id.to_string()),
            ("field", "city".to_string()),
            ("value", "Istanbul".to_string()),
            ("field", "tier".to_string()),
            ("value", "gold".to_string()),
        ]))
        .send()
        .await
        .expect("Failed to save recipient.");

    assert_eq!(save_recipient_response.status(), StatusCode::OK);

    let recipient_modal_response = client
        .get(format!(
            "{}/api/v1/recipients/{}",
            app.address(),
            manual_recipient_id
        ))
        .send()
        .await
        .expect("Failed to request recipient modal.");

    assert_eq!(recipient_modal_response.status(), StatusCode::OK);
    let recipient_modal_payload = response_json(recipient_modal_response).await;
    assert_eq!(
        recipient_modal_payload["recipient"]["name"],
        "Manual User Updated"
    );
    assert!(
        recipient_modal_payload["recipient"]["group_ids"]
            .as_array()
            .expect("Recipient group ids should be present.")
            .iter()
            .any(|group_id| group_id == broadcast_group_id)
    );
    assert!(
        recipient_modal_payload["recipient"]["fields"]
            .as_array()
            .expect("Recipient fields should be present.")
            .iter()
            .any(|field| field["name"] == "city" && field["value"] == "Istanbul")
    );

    let emails_response = client
        .get(format!(
            "{}/api/v1/emails?retry={}",
            app.address(),
            seeded.email_id
        ))
        .send()
        .await
        .expect("Failed to request emails API.");

    assert_eq!(emails_response.status(), StatusCode::OK);
    let emails_payload = response_json(emails_response).await;
    assert_eq!(emails_payload["retry_email"]["id"], seeded.email_id);
    assert!(
        emails_payload["groups"]
            .as_array()
            .expect("Group options should be present.")
            .iter()
            .any(|group| group["id"] == broadcast_group_id)
    );
    assert!(
        emails_payload["recipients"]
            .as_array()
            .expect("Recipient options should be present.")
            .iter()
            .any(|recipient| recipient["id"] == "manual@example.com")
    );
    assert!(
        emails_payload["recipients"]
            .as_array()
            .expect("Recipient options should be present.")
            .iter()
            .all(|recipient| recipient["id"] != "optout@example.com")
    );
    assert!(
        emails_payload["emails"]["items"]
            .as_array()
            .expect("Email preview items should be present.")
            .iter()
            .any(|item| item["id"] == seeded.email_id)
    );

    let send_recipients = serde_json::to_string(&vec![
        "manual@example.com".to_string(),
        broadcast_group_id.to_string(),
        "optout@example.com".to_string(),
    ])
    .expect("Recipients list should serialize.");
    let send_response = client
        .post(format!("{}/email/send", app.address()))
        .multipart(
            multipart::Form::new()
                .text("message", "<p>Hello <script>alert(1)</script></p>")
                .text("subject", "April update")
                .text("cooldown_days", "30")
                .part(
                    "recipients",
                    multipart::Part::text(send_recipients)
                        .mime_str("application/json")
                        .expect("Recipients MIME should be valid."),
                )
                .part(
                    "attachment",
                    multipart::Part::bytes(b"guide".to_vec())
                        .file_name("guide.txt")
                        .mime_str("text/plain")
                        .expect("Attachment MIME should be valid."),
                ),
        )
        .send()
        .await
        .expect("Failed to queue email.");

    assert_eq!(send_response.status(), StatusCode::OK);
    let send_payload = response_json(send_response).await;
    assert_eq!(send_payload["message"], "Сообщение добавлено в очередь.");

    let resend_response = client
        .post(format!(
            "{}/email/{}/resend",
            app.address(),
            seeded.email_id
        ))
        .send()
        .await
        .expect("Failed to queue retry.");

    assert_eq!(resend_response.status(), StatusCode::OK);
    let resend_payload = response_json(resend_response).await;
    assert_eq!(resend_payload["message"], "Сообщение добавлено в очередь.");

    let export_email_response = client
        .get(format!(
            "{}/email/{}/recipients/export",
            app.address(),
            seeded.email_id
        ))
        .send()
        .await
        .expect("Failed to export email recipients.");

    assert_eq!(export_email_response.status(), StatusCode::OK);
    let exported_csv = export_email_response
        .text()
        .await
        .expect("Exported email recipients CSV should be readable.");
    assert!(exported_csv.contains("fresh@example.com"));
    assert!(exported_csv.contains("optout@example.com"));

    let anonymous_client = common::build_no_redirect_client();
    let track_response = anonymous_client
        .get(format!(
            "{}/track/{}",
            app.address(),
            seeded.unopened_recipient_id
        ))
        .send()
        .await
        .expect("Failed to request tracking pixel.");

    assert!(track_response.status().is_redirection());
    assert_eq!(
        track_response
            .headers()
            .get(header::LOCATION)
            .and_then(|value| value.to_str().ok()),
        Some("/assets/placeholder.png")
    );

    let mut conn = app
        .db_pool()
        .get()
        .expect("Failed to get SQLite connection from pool.");
    let tracked_recipient = email_recipients::table
        .find(seeded.unopened_recipient_id)
        .select(DbEmailRecipient::as_select())
        .first::<DbEmailRecipient>(&mut conn)
        .expect("Tracked recipient should exist.");
    assert!(tracked_recipient.opened);
    let tracked_email = emails::table
        .find(seeded.email_id)
        .select(DbEmail::as_select())
        .first::<DbEmail>(&mut conn)
        .expect("Tracked email should exist.");
    assert_eq!(tracked_email.num_opened, 2);
    drop(conn);

    let unsubscribed_response = client
        .get(format!("{}/api/v1/unsubscribed-recipients", app.address()))
        .send()
        .await
        .expect("Failed to request unsubscribed recipients API.");

    assert_eq!(unsubscribed_response.status(), StatusCode::OK);
    let unsubscribed_payload = response_json(unsubscribed_response).await;
    assert!(
        unsubscribed_payload["items"]
            .as_array()
            .expect("Unsubscribed recipients should be present.")
            .iter()
            .any(|item| {
                item["email"] == "optout@example.com" && item["reason"] == "manual opt-out"
            })
    );

    let history_response = client
        .get(format!("{}/api/v1/email-history", app.address()))
        .send()
        .await
        .expect("Failed to request email history API.");

    assert_eq!(history_response.status(), StatusCode::OK);
    let history_payload = response_json(history_response).await;
    assert_eq!(
        history_payload["crm_service_url"],
        "https://crm.pushkind.test"
    );
    let history_items = history_payload["items"]
        .as_array()
        .expect("History items should be present.");
    assert!(
        history_items
            .iter()
            .any(|item| item["address"] == "fresh@example.com" && item["opened"] == true)
    );

    let history_download_response = client
        .get(format!("{}/history/download", app.address()))
        .send()
        .await
        .expect("Failed to download history CSV.");

    assert_eq!(history_download_response.status(), StatusCode::OK);
    let history_csv = history_download_response
        .text()
        .await
        .expect("History CSV should be readable.");
    assert!(history_csv.contains("fresh@example.com"));
    assert!(history_csv.contains("optout@example.com"));

    let save_settings_response = client
        .post(format!("{}/settings/save", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("login", "mailer@example.com"),
            ("password", "secret-123"),
            ("sender", "Pushkind Mailer"),
            ("smtp_server", "smtp.example.com"),
            ("smtp_port", "2525"),
            ("imap_server", "imap.example.com"),
            ("imap_port", "993"),
            ("message", "<p>Footer</p>"),
        ]))
        .send()
        .await
        .expect("Failed to save hub settings.");

    assert_eq!(save_settings_response.status(), StatusCode::OK);

    let mut conn = app
        .db_pool()
        .get()
        .expect("Failed to get SQLite connection from pool.");
    let saved_hub = hubs::table
        .find(common::HUB_ID)
        .select(DbHub::as_select())
        .first::<DbHub>(&mut conn)
        .expect("Saved hub should exist.");
    assert_eq!(saved_hub.login.as_deref(), Some("mailer@example.com"));
    assert_eq!(saved_hub.sender.as_deref(), Some("Pushkind Mailer"));
    assert_eq!(saved_hub.smtp_server.as_deref(), Some("smtp.example.com"));
    assert_eq!(saved_hub.smtp_port, Some(2525));
    assert_eq!(saved_hub.imap_server.as_deref(), Some("imap.example.com"));
    assert_eq!(saved_hub.imap_port, Some(993));
    assert_eq!(saved_hub.email_template.as_deref(), Some("<p>Footer</p>"));
    drop(conn);

    let hub_settings_response = client
        .get(format!("{}/api/v1/hub-settings", app.address()))
        .send()
        .await
        .expect("Failed to request hub settings API.");

    assert_eq!(hub_settings_response.status(), StatusCode::OK);
    let hub_settings_payload = response_json(hub_settings_response).await;
    assert_eq!(hub_settings_payload["login"], "mailer@example.com");
    assert_eq!(hub_settings_payload["sender"], "Pushkind Mailer");
    assert_eq!(hub_settings_payload["smtp_port"], 2525);
    assert_eq!(hub_settings_payload["imap_port"], 993);

    let delete_recipient_response = client
        .post(format!(
            "{}/recipient/{}/delete",
            app.address(),
            source_recipient_id
        ))
        .send()
        .await
        .expect("Failed to delete source recipient.");

    assert_eq!(delete_recipient_response.status(), StatusCode::OK);
    let mut conn = app
        .db_pool()
        .get()
        .expect("Failed to get SQLite connection from pool.");
    let deleted_recipient = recipients::table
        .find(source_recipient_id)
        .select(DbRecipient::as_select())
        .first::<DbRecipient>(&mut conn)
        .optional()
        .expect("Recipient lookup should succeed.");
    assert!(deleted_recipient.is_none());
    drop(conn);

    let delete_group_response = client
        .post(format!(
            "{}/group/{}/delete",
            app.address(),
            source_group_id
        ))
        .send()
        .await
        .expect("Failed to delete source group.");

    assert_eq!(delete_group_response.status(), StatusCode::OK);
    let mut conn = app
        .db_pool()
        .get()
        .expect("Failed to get SQLite connection from pool.");
    let deleted_group = groups::table
        .find(source_group_id)
        .select(DbGroup::as_select())
        .first::<DbGroup>(&mut conn)
        .optional()
        .expect("Group lookup should succeed.");
    assert!(deleted_group.is_none());
    drop(conn);

    let delete_email_response = client
        .post(format!(
            "{}/email/{}/delete",
            app.address(),
            seeded.email_id
        ))
        .send()
        .await
        .expect("Failed to delete seeded email.");

    assert_eq!(delete_email_response.status(), StatusCode::OK);
    let mut conn = app
        .db_pool()
        .get()
        .expect("Failed to get SQLite connection from pool.");
    let deleted_email = emails::table
        .find(seeded.email_id)
        .select(DbEmail::as_select())
        .first::<DbEmail>(&mut conn)
        .optional()
        .expect("Email lookup should succeed.");
    assert!(deleted_email.is_none());
    drop(conn);

    let clean_recipients_response = client
        .post(format!("{}/recipients/clean", app.address()))
        .send()
        .await
        .expect("Failed to clean recipients.");

    assert_eq!(clean_recipients_response.status(), StatusCode::OK);
    let mut conn = app
        .db_pool()
        .get()
        .expect("Failed to get SQLite connection from pool.");
    let remaining_recipients = recipients::table
        .filter(recipients::hub_id.eq(common::HUB_ID))
        .select(DbRecipient::as_select())
        .load::<DbRecipient>(&mut conn)
        .expect("Recipient list should load.");
    let remaining_groups = groups::table
        .filter(groups::hub_id.eq(common::HUB_ID))
        .select(DbGroup::as_select())
        .load::<DbGroup>(&mut conn)
        .expect("Group list should load.");
    assert!(remaining_recipients.is_empty());
    assert!(remaining_groups.is_empty());
}

#[ignore = "local-only end-to-end test"]
#[actix_web::test]
async fn test_emailer_non_admin_settings_access_story() {
    let app = common::spawn_app().await;
    seed_hub(&app);

    let emailer_client = common::build_reqwest_client();
    common::login_as(
        &emailer_client,
        app.address(),
        "emailer@example.com",
        "Emailer User",
        common::HUB_ID,
        &["emailer"],
    )
    .await;

    let emailer_index_response = emailer_client
        .get(format!("{}/", app.address()))
        .send()
        .await
        .expect("Failed to request Emailer index for non-admin user.");

    assert_eq!(emailer_index_response.status(), StatusCode::OK);

    let emailer_settings_client = common::build_no_redirect_client();
    common::login_as(
        &emailer_settings_client,
        app.address(),
        "emailer@example.com",
        "Emailer User",
        common::HUB_ID,
        &["emailer"],
    )
    .await;

    let settings_response = emailer_settings_client
        .get(format!("{}/settings", app.address()))
        .send()
        .await
        .expect("Failed to request settings page for non-admin user.");

    assert_eq!(settings_response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        settings_response
            .headers()
            .get(header::LOCATION)
            .and_then(|value| value.to_str().ok()),
        Some("/")
    );

    let emailer_iam_response = emailer_client
        .get(format!("{}/api/v1/iam", app.address()))
        .send()
        .await
        .expect("Failed to request IAM payload for non-admin user.");

    assert_eq!(emailer_iam_response.status(), StatusCode::OK);
    let emailer_iam_payload = response_json(emailer_iam_response).await;
    assert!(
        emailer_iam_payload["local_menu_items"]
            .as_array()
            .expect("Local menu items should be present.")
            .iter()
            .all(|item| item["url"] != "/settings")
    );

    let emailer_admin_api_client = common::build_no_redirect_client();
    common::login_as(
        &emailer_admin_api_client,
        app.address(),
        "emailer@example.com",
        "Emailer User",
        common::HUB_ID,
        &["emailer"],
    )
    .await;

    let hub_settings_response = emailer_admin_api_client
        .get(format!("{}/api/v1/hub-settings", app.address()))
        .send()
        .await
        .expect("Failed to request hub settings API for non-admin user.");

    assert_eq!(hub_settings_response.status(), StatusCode::SEE_OTHER);
    assert!(
        hub_settings_response
            .headers()
            .get(header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .expect("Hub settings redirect should be present.")
            .starts_with("https://users.pushkind.test/auth/signin?next=")
    );

    let save_settings_response = emailer_admin_api_client
        .post(format!("{}/settings/save", app.address()))
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(form_body(vec![
            ("login", "mailer@example.com"),
            ("password", "secret-123"),
        ]))
        .send()
        .await
        .expect("Failed to post settings as non-admin user.");

    assert_eq!(save_settings_response.status(), StatusCode::SEE_OTHER);
    assert!(
        save_settings_response
            .headers()
            .get(header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .expect("Settings save redirect should be present.")
            .starts_with("https://users.pushkind.test/auth/signin?next=")
    );
}

#[ignore = "local-only end-to-end test"]
#[actix_web::test]
async fn test_emailer_no_role_access_story() {
    let app = common::spawn_app().await;
    seed_hub(&app);

    let no_role_index_client = common::build_no_redirect_client();
    common::login_as(
        &no_role_index_client,
        app.address(),
        "blocked@example.com",
        "Blocked User",
        common::HUB_ID,
        &[],
    )
    .await;

    let denied_index_response = no_role_index_client
        .get(format!("{}/", app.address()))
        .send()
        .await
        .expect("Failed to request Emailer index without role.");

    assert_eq!(denied_index_response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        denied_index_response
            .headers()
            .get(header::LOCATION)
            .and_then(|value| value.to_str().ok()),
        Some("/na")
    );

    let no_role_page_client = common::build_reqwest_client();
    common::login_as(
        &no_role_page_client,
        app.address(),
        "blocked@example.com",
        "Blocked User",
        common::HUB_ID,
        &[],
    )
    .await;

    let no_access_page_response = no_role_page_client
        .get(format!("{}/na", app.address()))
        .send()
        .await
        .expect("Failed to request no-access page.");

    assert_eq!(no_access_page_response.status(), StatusCode::OK);
    let no_access_html = no_access_page_response
        .text()
        .await
        .expect("No-access page should be readable.");
    assert!(no_access_html.contains("<title>Emailer No Access</title>"));
}

#[ignore = "local-only end-to-end test"]
#[actix_web::test]
async fn test_emailer_no_role_api_story() {
    let app = common::spawn_app().await;
    seed_hub(&app);

    let no_role_iam_client = common::build_no_redirect_client();
    common::login_as(
        &no_role_iam_client,
        app.address(),
        "blocked@example.com",
        "Blocked User",
        common::HUB_ID,
        &[],
    )
    .await;

    let denied_iam_response = no_role_iam_client
        .get(format!("{}/api/v1/iam", app.address()))
        .send()
        .await
        .expect("Failed to request IAM payload without role.");

    assert_eq!(denied_iam_response.status(), StatusCode::SEE_OTHER);
    assert!(
        denied_iam_response
            .headers()
            .get(header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .expect("IAM redirect should be present.")
            .starts_with("https://users.pushkind.test/auth/signin?next=")
    );

    let no_role_no_access_client = common::build_reqwest_client();
    common::login_as(
        &no_role_no_access_client,
        app.address(),
        "blocked@example.com",
        "Blocked User",
        common::HUB_ID,
        &[],
    )
    .await;

    let no_access_response = no_role_no_access_client
        .get(format!("{}/api/v1/no-access", app.address()))
        .send()
        .await
        .expect("Failed to request no-access payload.");

    assert_eq!(no_access_response.status(), StatusCode::OK);
    let no_access_payload = response_json(no_access_response).await;
    assert_eq!(
        no_access_payload["current_user"]["email"],
        "blocked@example.com"
    );
}
