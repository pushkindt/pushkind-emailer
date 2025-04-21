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
    domain: &str,
) -> Result<(), mail_send::Error> {
    let template = hub.email_template.as_deref().unwrap_or_default();

    let unsubscribe_url = hub.get_usubscribe_url();
    let mut body: String;

    let template = template.replace("{unsubscribe_url}", &unsubscribe_url);

    if template.contains("{message}") {
        body = template.replace("{message}", &email.message);
    } else {
        body = format!("{}{}", &email.message, template);
    }

    body.push_str(&format!(
        r#"<img height="1" width="1" border="0" src="https://{domain}/track/{}">"#,
        recipient.id
    ));

    let message_id = format!("{}@{}", recipient.id, domain);

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
        .message_id(message_id)
        .header(
            "List-Unsubscribe",
            HeaderType::from(URL::new(&unsubscribe_url)),
        );

    println!(
        "attachment_mime: {:?}, attachment_name: {:?}, attachment: {:?}",
        email.attachment_mime, email.attachment_name, email.attachment
    );

    if let (Some(mime), Some(name), Some(content)) = (
        email.attachment_mime.as_deref(),
        email.attachment_name.as_deref(),
        email.attachment.as_deref(),
    ) {
        if !name.is_empty() && !content.is_empty() {
            message = message.attachment(mime, name, content);
        }
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
    domain: &str,
) -> Result<(), Box<dyn Error>> {
    let pool = db_pool.lock().await;
    let mut conn = get_db_connection(&pool).ok_or("Cannot get connection from the pool")?;

    let email = get_email(&mut conn, email_id)?;
    let user = get_user(&mut conn, email.user_id)?;
    let recipients = get_email_recipients(&mut conn, email_id)?;
    let hub = get_hub(&mut conn, user.hub_id.ok_or("Hub ID is missing")?)?;

    info!("Sending email for email_id {} via hub {}", email_id, hub.id);

    for recipient in recipients {
        if let Err(e) = send_smtp_message(&hub, &email, &recipient, &domain).await {
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
    let domain = Arc::from(env::var("DOMAIN").unwrap_or_default());
    //

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
                let domain = Arc::clone(&domain);

                tokio::spawn(async move {
                    if let Err(e) = send_email(email_id, pool_clone, &domain).await {
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
