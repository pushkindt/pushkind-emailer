use std::env;
use std::error::Error;
use std::sync::Arc;

use dotenvy::dotenv;
use log::{error, info};
use mail_send::SmtpClientBuilder;
use mail_send::mail_builder::MessageBuilder;
use pushkind_emailer::models::email::{Email, EmailRecipient};
use pushkind_emailer::models::hub::Hub;
use tokio::sync::Mutex;
use zmq;

use pushkind_emailer::db::{DbPool, establish_connection_pool, get_db_connection};
use pushkind_emailer::repository::email::{
    get_email, get_email_recipients, set_email_recipient_sent_status, set_email_sent_status,
};
use pushkind_emailer::repository::hub::get_hub;
use pushkind_emailer::repository::user::get_user;

async fn send_smtp_message(
    hub: &Hub,
    email: &Email,
    recipient: &EmailRecipient,
    mail_tracking_url: &str,
    mail_message_id: &str,
) -> Result<(), mail_send::Error> {
    let template = hub.email_template.as_deref().unwrap_or_default();
    let mut body: String;
    if template.contains("{message}") {
        body = template.replace("{message}", &email.message);
    } else {
        body = format!("{}{}", &email.message, template);
    }

    body.push_str(&format!(
        r#"<img height="1" width="1" border="0" src="{mail_tracking_url}{}">"#,
        recipient.id
    ));

    let message_id = format!("{}{}", recipient.id, mail_message_id);

    let recipient_address = vec![("", recipient.address.as_str())];
    let sender_email = hub.sender.as_deref().unwrap_or_default();
    let sender_login = hub.login.as_deref().unwrap_or_default();
    let subject = email.subject.as_deref().unwrap_or_default();

    let mut message = MessageBuilder::new()
        .from((sender_email, sender_login))
        .to(recipient_address)
        .subject(subject)
        .html_body(&body)
        .text_body(&body)
        .message_id(message_id);

    if let (Some(mime), Some(name), Some(content)) = (
        email.attachment_mime.as_deref(),
        email.attachment_name.as_deref(),
        email.attachment.as_deref(),
    ) {
        message = message.attachment(mime, name, content);
    }

    let smtp_server = hub.smtp_server.as_deref().unwrap_or_default();
    let smtp_port = hub.smtp_port.unwrap_or(25) as u16; // assume smtp_port is Option<u16>?

    let credentials = (
        hub.login.as_deref().unwrap_or_default(),
        hub.password.as_deref().unwrap_or_default(),
    );

    SmtpClientBuilder::new(smtp_server, smtp_port)
        .implicit_tls(true)
        .credentials(credentials)
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
) -> Result<(), Box<dyn Error>> {
    let pool = db_pool.lock().await;
    let mut conn = get_db_connection(&pool).ok_or("Cannot get connection from the pool")?;

    let email = get_email(&mut conn, email_id)?;
    let user = get_user(&mut conn, email.user_id)?;
    let recipients = get_email_recipients(&mut conn, email_id)?;
    let hub = get_hub(&mut conn, user.hub_id.ok_or("Hub ID is missing")?)?;

    info!("Sending email for email_id {} via hub {}", email_id, hub.id);

    for recipient in recipients {
        if let Err(e) =
            send_smtp_message(&hub, &email, &recipient, mail_tracking_url, mail_message_id).await
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

    let bind_address = env::var("ADDRESS").unwrap_or_default();
    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "app.db".to_string());
    let zmq_address =
        env::var("ZMQ_ADDRESS").unwrap_or_else(|_| "tcp://127.0.0.1:5555".to_string());
    let mail_tracking_url = Arc::from(format!("{bind_address}/track/"));
    let mail_message_id = Arc::from(env::var("MAIL_MESSAGE_ID").unwrap_or_default());

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

                tokio::spawn(async move {
                    if let Err(e) =
                        send_email(email_id, pool_clone, &mail_tracking_url, &mail_message_id).await
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
