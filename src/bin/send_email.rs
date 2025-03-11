use std::env;
use std::error::Error;
use std::{thread, time};

use dotenvy::dotenv;
use log::{error, info};
use mail_send::mail_builder::MessageBuilder;

use mail_send::SmtpClientBuilder;
use pushkind_emailer::db::{DbPool, establish_connection_pool, get_db_connection};
use pushkind_emailer::repository::email::{get_email, get_email_recipients};
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
) -> Result<(), mail_send::Error> {
    let message = MessageBuilder::new()
        .from((from, smtp_username))
        .to(vec![("", to)])
        .subject(subject)
        .html_body(body)
        .text_body(body);

    // Connect to the SMTP submissions port, upgrade to TLS and
    // authenticate using the provided credentials.
    SmtpClientBuilder::new(smtp_host, smtp_port)
        .implicit_tls(true)
        .credentials((smtp_username, smtp_password))
        .connect()
        .await?
        .send(message)
        .await
}

async fn send_email(email_id: i32, db_pool: &DbPool) -> Result<(), Box<dyn Error>> {
    let mut conn = get_db_connection(db_pool)?;

    let email = get_email(&mut conn, email_id)?;
    let user = get_user(&mut conn, email.user_id)?;
    let recipients = get_email_recipients(&mut conn, email_id)?;

    let hub_id: i32 = user
        .hub_id
        .ok_or_else(|| Box::<dyn Error>::from("Expected a number, found None"))?;

    let hub = get_hub(&mut conn, hub_id)?;

    let body = email.message;

    let smtp_host = hub
        .server
        .ok_or_else(|| Box::<dyn Error>::from("Expected smtp server, found None"))?;
    let smtp_port: i32 = hub
        .port
        .ok_or_else(|| Box::<dyn Error>::from("Expected smtp port, found None"))?;
    let smtp_username = hub
        .login
        .ok_or_else(|| Box::<dyn Error>::from("Expected smtp login, found None"))?;
    let smtp_password = hub
        .password
        .ok_or_else(|| Box::<dyn Error>::from("Expected smtp password, found None"))?;
    let from = hub.sender.unwrap_or_default();

    println!("Sending email for email_id {} for hub {}", email_id, hub.id);
    for recipient in recipients {
        let result = send_smtp_message(
            &smtp_host,
            smtp_port.try_into().unwrap(),
            &smtp_username,
            &smtp_password,
            &from,
            &recipient.address,
            "Pushkind-Emailer",
            &body,
        )
        .await;

        match result {
            Ok(_) => {
                println!("Email sent successfully to {}", recipient.address);
            }
            Err(e) => {
                println!("Failed to send email to {}: {}", recipient.address, e);
            }
        }
    }
    println!("Finished processing email_id: {}", email_id);
    Ok(())
}

#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    dotenv().ok(); // Load .env file

    let database_url = env::var("DATABASE_URL").unwrap_or("app.db".to_string());
    let zmq_address = env::var("ZMQ_ADDRESS").unwrap_or("tcp://127.0.0.1:5555".to_string());

    let context = zmq::Context::new();
    let responder = context.socket(zmq::PULL).expect("Cannot create zmq socket");
    responder
        .bind(&zmq_address)
        .expect("Cannot bind to zmq port");

    let pool = establish_connection_pool(database_url);
    info!("Starting email worker");
    loop {
        let mut buffer = [0; 4];
        match responder.recv_into(&mut buffer, 0) {
            Ok(_) => {
                let email_id: i32 = i32::from_be_bytes(buffer);
                if let Err(e) = send_email(email_id, &pool).await {
                    error!("Error sending email message: {}", e);
                }
            }
            Err(e) => {
                error!("Error receiving message: {}", e);
                continue;
            }
        }
    }
}
