use std::convert::TryInto;
use std::env;
use std::error::Error;
use std::sync::Arc;

use dotenvy::dotenv;
use log::{error, info};
use mail_send::SmtpClientBuilder;
use mail_send::mail_builder::{
    MessageBuilder,
    headers::{HeaderType, url::URL},
};
use tokio::sync::Mutex;
use zmq;

use pushkind_emailer::db::{DbPool, establish_connection_pool, get_db_connection};
use pushkind_emailer::repository::email::{
    get_email, get_email_recipients, set_email_recipient_sent_status, set_email_sent_status,
};
use pushkind_emailer::repository::hub::get_hub;
use pushkind_emailer::repository::user::get_user;

async fn send_smtp_message(
    smtp_host: &str,
    smtp_port: u16,
    smtp_username: &str,
    smtp_password: &str,
    from: &str,
    to: &str,
    subject: &str,
    body: &str,
    message_id: &str,
    mail_unsubscribe_url: &str,
    attachment: Option<&[u8]>,
    attachment_name: Option<&str>,
    attachment_mime: Option<&str>,
) -> Result<(), mail_send::Error> {
    let mut message = MessageBuilder::new()
        .from((from, smtp_username))
        .to(vec![("", to)])
        .subject(subject)
        .html_body(body)
        .text_body(body)
        .message_id(message_id)
        .header(
            "List-Unsubscribe",
            HeaderType::from(URL::new(mail_unsubscribe_url)),
        );

    if attachment.is_some() && attachment_name.is_some() && attachment_mime.is_some() {
        message = message.attachment(
            attachment_mime.unwrap(),
            attachment_name.unwrap(),
            attachment.unwrap(),
        );
    }
    SmtpClientBuilder::new(smtp_host, smtp_port)
        .implicit_tls(true)
        .credentials((smtp_username, smtp_password))
        .connect()
        .await?
        .send(message)
        .await
}

async fn send_email(
    email_id: i32,
    db_pool: Arc<Mutex<DbPool>>,
    mail_tracking_url: &str,
    mail_message_id: &str,
    mail_unsubscribe_url: &str,
) -> Result<(), Box<dyn Error>> {
    let pool = db_pool.lock().await;
    let mut conn = get_db_connection(&pool).ok_or("Cannot get connection from the pool")?;

    let email = get_email(&mut conn, email_id)?;
    let user = get_user(&mut conn, email.user_id)?;
    let recipients = get_email_recipients(&mut conn, email_id)?;

    let hub = get_hub(&mut conn, user.hub_id.ok_or("Hub ID is missing")?)?;

    let smtp_host = hub.smtp_server.ok_or("SMTP server is missing")?;
    let smtp_port: u16 = hub
        .smtp_port
        .ok_or("SMTP port is missing")?
        .try_into()
        .map_err(|_| "Invalid SMTP port")?;
    let smtp_username = hub.login.ok_or("SMTP login is missing")?;
    let smtp_password = hub.password.ok_or("SMTP password is missing")?;
    let from = hub.sender.unwrap_or_default();

    info!("Sending email for email_id {} via hub {}", email_id, hub.id);

    let email_subject = email.subject.unwrap_or_default();

    for recipient in recipients {
        let body = format!(
            "{}<img border=\"0\" width=\"1\" height=\"1\" style=\"border:none\" src=\"{}{}\">",
            &email.message, mail_tracking_url, recipient.id
        );

        if let Err(e) = send_smtp_message(
            &smtp_host,
            smtp_port,
            &smtp_username,
            &smtp_password,
            &from,
            &recipient.address,
            &email_subject,
            &body,
            &format!("{}{}", recipient.id, mail_message_id),
            &mail_unsubscribe_url,
            email.attachment.as_deref(),
            email.attachment_name.as_deref(),
            email.attachment_mime.as_deref(),
        )
        .await
        {
            error!("Failed to send email to {}: {}", recipient.address, e);
            continue;
        }

        info!("Email sent successfully to {}", recipient.address);

        if let Err(e) = set_email_recipient_sent_status(&mut conn, recipient.id, true) {
            error!(
                "Failed to update sent status for recipient {}: {}",
                recipient.id, e
            );
        }
    }

    if let Err(e) = set_email_sent_status(&mut conn, email_id, true) {
        error!(
            "Failed to update email sent status for email {}: {}",
            email_id, e
        );
    } else {
        info!(
            "Email sent status updated successfully for email {}",
            email_id
        );
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    dotenv().ok(); // Load .env file

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "app.db".to_string());
    let zmq_address =
        env::var("ZMQ_ADDRESS").unwrap_or_else(|_| "tcp://127.0.0.1:5555".to_string());
    let mail_tracking_url = Arc::from(env::var("MAIL_TRACKING_URL").unwrap_or_default());
    let mail_message_id = Arc::from(env::var("MAIL_MESSAGE_ID").unwrap_or_default());
    let mail_unsubscribe_url = Arc::from(env::var("MAIL_UNSUBSCRIBE_URL").unwrap_or_default());

    let context = zmq::Context::new();
    let responder = context.socket(zmq::PULL).expect("Cannot create zmq socket");
    responder
        .bind(&zmq_address)
        .expect("Cannot bind to zmq port");

    let pool = match establish_connection_pool(database_url) {
        Ok(pool) => pool,
        Err(e) => {
            error!("Failed to establish database connection: {}", e);
            std::process::exit(1);
        }
    };

    let pool = Arc::new(Mutex::new(pool));

    info!("Starting email worker");

    loop {
        let mut buffer = [0; 4];
        match responder.recv_into(&mut buffer, 0) {
            Ok(_) => {
                let email_id = i32::from_be_bytes(buffer);
                let pool_clone = Arc::clone(&pool);
                let mail_tracking_url = Arc::clone(&mail_tracking_url);
                let mail_message_id = Arc::clone(&mail_message_id);
                let mail_unsubscribe_url = Arc::clone(&mail_unsubscribe_url);

                tokio::spawn(async move {
                    if let Err(e) = send_email(
                        email_id,
                        pool_clone,
                        &mail_tracking_url,
                        &mail_message_id,
                        &mail_unsubscribe_url,
                    )
                    .await
                    {
                        error!("Error sending email message: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Error receiving message: {}", e);
                continue;
            }
        }
    }
}
