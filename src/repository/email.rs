use std::error::Error;

use diesel::prelude::*;

use crate::models::{
    email::{Email, EmailRecipient, NewEmail, NewEmailRecipient},
    recipient::Recipient,
};

pub fn get_user_all_emails_with_recipients(
    conn: &mut SqliteConnection,
    user_id: i32,
) -> QueryResult<Vec<(Email, Vec<EmailRecipient>)>> {
    use crate::schema::emails;

    // Read all emails for a user sorted by timestamp
    let all_emails: Vec<Email> = emails::table
        .filter(emails::user_id.eq(user_id))
        .order(emails::created_at.desc())
        .select(Email::as_select()) // Ensure Diesel knows we're selecting the full Email struct
        .load(conn)?;

    // Load all recipients belonging to the fetched emails
    let email_recipients: Vec<EmailRecipient> = EmailRecipient::belonging_to(&all_emails)
        .select(EmailRecipient::as_select()) // Ensure Diesel knows we're selecting the full EmailRecipient struct
        .load(conn)?;

    // Group recipients by email and return
    Ok(email_recipients
        .grouped_by(&all_emails)
        .into_iter()
        .zip(all_emails)
        .map(|(recipients, email)| (email, recipients))
        .collect())
}

fn create_email_recipient(
    conn: &mut SqliteConnection,
    email_id: i32,
    address: &str,
    updated_at: &chrono::NaiveDateTime,
) -> QueryResult<EmailRecipient> {
    use crate::schema::email_recipients;

    let new_email_recipient = NewEmailRecipient {
        email_id,
        address,
        opened: false,
        updated_at,
        is_sent: false,
        replied: false,
    };

    diesel::insert_into(email_recipients::table)
        .values(&new_email_recipient)
        .execute(conn)?;

    email_recipients::table
        .filter(email_recipients::email_id.eq(email_id))
        .filter(email_recipients::address.eq(address))
        .first(conn)
}

pub fn create_email(
    conn: &mut SqliteConnection,
    subject: Option<&str>,
    message: &str,
    recipients: &Vec<String>,
    attachment: Option<&[u8]>,
    attachment_name: Option<&str>,
    attachment_mime: Option<&str>,
    user_id: i32,
) -> Result<Email, Box<dyn Error>> {
    use crate::schema::emails;
    use crate::schema::groups_recipients;
    use crate::schema::recipients;

    let created_at = chrono::Utc::now().naive_utc();

    let new_email = NewEmail {
        user_id,
        message: message,
        created_at: &created_at,
        is_sent: false,
        subject: subject,
        attachment: attachment,
        attachment_name: attachment_name,
        attachment_mime: attachment_mime,
    };

    diesel::insert_into(emails::table)
        .values(&new_email)
        .execute(conn)?;

    let email: Email = emails::table
        .filter(emails::user_id.eq(user_id))
        .filter(emails::created_at.eq(created_at))
        .filter(emails::message.eq(&new_email.message))
        .order(emails::created_at.desc())
        .first(conn)?;

    for recipient in recipients {
        // if recipient is an email and exists in the database create a new EmailRecipient
        // if recipient is not an email but a group id then fetch the group and create a new EmailRecipient for each member
        if recipient.contains('@') {
            let recipient = recipient.trim();
            let recipient: Recipient = recipients::table
                .filter(recipients::email.eq(recipient))
                .filter(recipients::unsubscribed_at.is_null())
                .select(Recipient::as_select())
                .first(conn)?;

            create_email_recipient(conn, email.id, &recipient.email, &created_at)?;
        } else {
            let group_id = recipient.parse::<i32>()?;

            let group_members: Vec<Recipient> = groups_recipients::table
                .filter(groups_recipients::group_id.eq(group_id))
                .inner_join(
                    recipients::table.on(groups_recipients::recipient_id.eq(recipients::id)),
                )
                .select(Recipient::as_select())
                .load(conn)?;

            for member in group_members {
                create_email_recipient(conn, email.id, &member.email, &created_at)?;
            }
        }
    }

    Ok(email)
}

pub fn remove_email(conn: &mut SqliteConnection, email_id: i32) -> QueryResult<usize> {
    use crate::schema::{email_recipients, emails};

    diesel::delete(emails::table.filter(emails::id.eq(email_id))).execute(conn)?;
    diesel::delete(email_recipients::table.filter(email_recipients::email_id.eq(email_id)))
        .execute(conn)
}

pub fn get_email(conn: &mut SqliteConnection, email_id: i32) -> QueryResult<Email> {
    use crate::schema::emails;

    emails::table.filter(emails::id.eq(email_id)).first(conn)
}

pub fn get_email_recipients(
    conn: &mut SqliteConnection,
    email_id: i32,
) -> QueryResult<Vec<EmailRecipient>> {
    use crate::schema::email_recipients;

    email_recipients::table
        .filter(email_recipients::email_id.eq(email_id))
        .load(conn)
}

pub fn set_email_sent_status(
    conn: &mut SqliteConnection,
    email_id: i32,
    status: bool,
) -> QueryResult<usize> {
    use crate::schema::emails;

    diesel::update(emails::table.filter(emails::id.eq(email_id)))
        .set(emails::is_sent.eq(status))
        .execute(conn)
}

pub fn set_email_recipient_sent_status(
    conn: &mut SqliteConnection,
    recipient_id: i32,
    status: bool,
) -> QueryResult<usize> {
    use crate::schema::email_recipients;

    diesel::update(email_recipients::table.filter(email_recipients::id.eq(recipient_id)))
        .set(email_recipients::is_sent.eq(status))
        .execute(conn)
}

pub fn set_email_recipient_opened_status(
    conn: &mut SqliteConnection,
    recipient_id: i32,
    status: bool,
) -> QueryResult<usize> {
    use crate::schema::email_recipients;

    diesel::update(email_recipients::table.filter(email_recipients::id.eq(recipient_id)))
        .set(email_recipients::opened.eq(status))
        .execute(conn)
}

pub fn reset_email_sent_and_opened_status(
    conn: &mut SqliteConnection,
    email_id: i32,
) -> QueryResult<usize> {
    use crate::schema::email_recipients;

    set_email_sent_status(conn, email_id, false)?;

    diesel::update(email_recipients::table.filter(email_recipients::email_id.eq(email_id)))
        .set((
            email_recipients::opened.eq(false),
            email_recipients::is_sent.eq(false),
        ))
        .execute(conn)
}

pub fn get_hub_email_recipients_not_replied(
    conn: &mut SqliteConnection,
    hub_id: i32,
) -> QueryResult<Vec<EmailRecipient>> {
    use crate::schema::email_recipients;
    use crate::schema::emails;
    use crate::schema::users;

    email_recipients::table
        .inner_join(emails::table.on(email_recipients::email_id.eq(emails::id)))
        .inner_join(users::table.on(emails::user_id.eq(users::id)))
        .filter(users::hub_id.eq(hub_id))
        .filter(email_recipients::replied.eq(false))
        .select(EmailRecipient::as_select())
        .load(conn)
}

pub fn set_email_recipient_replied_status(
    conn: &mut SqliteConnection,
    email_id: i32,
    recipient_id: i32,
) -> QueryResult<usize> {
    use crate::schema::email_recipients;
    use crate::schema::emails;

    diesel::update(email_recipients::table.filter(email_recipients::id.eq(recipient_id)))
        .set((
            email_recipients::replied.eq(true),
            email_recipients::is_sent.eq(true),
            email_recipients::opened.eq(true),
        ))
        .execute(conn)?;

    diesel::update(emails::table.filter(emails::id.eq(email_id)))
        .set(emails::is_sent.eq(true))
        .execute(conn)
}

pub fn update_email_num_sent(conn: &mut SqliteConnection, email_id: i32) -> QueryResult<usize> {
    use crate::schema::email_recipients;
    use crate::schema::emails;

    let num_value: i64 = email_recipients::table
        .filter(email_recipients::email_id.eq(email_id))
        .filter(email_recipients::is_sent.eq(true))
        .count()
        .get_result(conn)?;

    //Set email num_sent to the number of recipients that have is_sent = true
    diesel::update(emails::table.filter(emails::id.eq(email_id)))
        .set(emails::num_sent.eq(num_value as i32))
        .execute(conn)
}

pub fn update_email_num_opened(conn: &mut SqliteConnection, email_id: i32) -> QueryResult<usize> {
    use crate::schema::email_recipients;
    use crate::schema::emails;

    let num_value: i64 = email_recipients::table
        .filter(email_recipients::email_id.eq(email_id))
        .filter(email_recipients::opened.eq(true))
        .count()
        .get_result(conn)?;

    //Set email num_sent to the number of recipients that have is_sent = true
    diesel::update(emails::table.filter(emails::id.eq(email_id)))
        .set(emails::num_sent.eq(num_value as i32))
        .execute(conn)
}

pub fn update_email_num_replied(conn: &mut SqliteConnection, email_id: i32) -> QueryResult<usize> {
    use crate::schema::email_recipients;
    use crate::schema::emails;

    let num_value: i64 = email_recipients::table
        .filter(email_recipients::email_id.eq(email_id))
        .filter(email_recipients::replied.eq(true))
        .count()
        .get_result(conn)?;

    //Set email num_sent to the number of recipients that have is_sent = true
    diesel::update(emails::table.filter(emails::id.eq(email_id)))
        .set(emails::num_sent.eq(num_value as i32))
        .execute(conn)
}

pub fn get_email_recipient(
    conn: &mut SqliteConnection,
    recipient_id: i32,
) -> QueryResult<EmailRecipient> {
    use crate::schema::email_recipients;

    email_recipients::table
        .filter(email_recipients::id.eq(recipient_id))
        .first(conn)
}
