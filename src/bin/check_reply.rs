use std::env;

use dotenvy::dotenv;
use log::{error, info};

use pushkind_emailer::db::{DbConnection, establish_connection_pool, get_db_connection};
use pushkind_emailer::models::hub::Hub;
use pushkind_emailer::repository::email::set_email_recipient_replied_status;
use pushkind_emailer::repository::email::{
    get_hub_email_recipients_not_replied, update_email_num_replied,
};
use pushkind_emailer::repository::hub::list_hubs;

pub fn check_hub_email_replied(db_conn: &mut DbConnection, hub: &Hub, domain: &str) {
    let recipients = match get_hub_email_recipients_not_replied(db_conn, hub.id) {
        Ok(recipients) => recipients,
        Err(e) => {
            error!("Cannot get recipients: {}", e);
            return;
        }
    };

    let (imap_server, imap_port, username, password) =
        match (&hub.imap_server, hub.imap_port, &hub.login, &hub.password) {
            (Some(server), Some(port), Some(username), Some(password)) => {
                (server, port, username, password)
            }
            _ => {
                error!("Cannot get imap server and port for the hub");
                return;
            }
        };

    let imap_port = imap_port as u16;

    let tls = match native_tls::TlsConnector::builder().build() {
        Ok(tls) => tls,
        Err(e) => {
            error!("Cannot build tls connector: {}", e);
            return;
        }
    };
    let client = match imap::connect((imap_server.as_str(), imap_port), imap_server, &tls) {
        Ok(client) => client,
        Err(e) => {
            error!("Cannot connect to imap server: {}", e);
            return;
        }
    };

    let mut session: imap::Session<_> = match client.login(username, password).map_err(|e| e.0) {
        Ok(session) => session,
        Err(e) => {
            error!("Cannot login to imap server: {}", e);
            return;
        }
    };

    match session.select("INBOX") {
        Ok(_) => info!("Selected INBOX"),
        Err(e) => {
            error!("Cannot select INBOX: {}", e);
            return;
        }
    }

    for recipient in recipients {
        // Define the In-Reply-To Message-ID you are looking for
        let in_reply_to_id = format!("<{}@{}>", recipient.id, domain);

        // Search for emails with a matching In-Reply-To header
        let query = format!("HEADER In-Reply-To {}", in_reply_to_id);
        let search_result = match session.search(&query) {
            Ok(search_result) => search_result,
            Err(e) => {
                error!("Cannot search for emails: {}", e);
                continue;
            }
        };

        if search_result.is_empty() {
            info!(
                "No matching emails found for email_id: {}, recipient: {}.",
                recipient.email_id, recipient.address
            );
        } else {
            info!(
                "Found emails with In-Reply-To {}: {:?}",
                in_reply_to_id, search_result
            );
            match set_email_recipient_replied_status(db_conn, recipient.email_id, recipient.id) {
                Ok(_) => info!("Email recipient replied status set"),
                Err(e) => error!("Cannot set email recipient replied status: {}", e),
            }

            if let Err(e) = update_email_num_replied(db_conn, recipient.email_id) {
                error!("Failed to update email num_sent for: {}", e);
            }
        }
    }

    match session.logout() {
        Ok(_) => info!("Logged out"),
        Err(e) => error!("Cannot logout: {}", e),
    }
}

fn main() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    dotenv().ok(); // Load .env file

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "app.db".to_string());
    let domain = env::var("DOMAIN").unwrap_or_default();

    let db_pool = match establish_connection_pool(database_url) {
        Ok(pool) => pool,
        Err(e) => {
            error!("Cannot establish db connection: {}", e);
            return;
        }
    };

    let mut db_conn = match get_db_connection(&db_pool) {
        Some(conn) => conn,
        None => {
            error!("Cannot get db connection");
            return;
        }
    };

    let hubs = match list_hubs(&mut db_conn) {
        Ok(hub) => hub,
        Err(e) => {
            error!("Cannot get hub: {}", e);
            return;
        }
    };

    for hub in hubs {
        info!("Checking hub: {}", hub.name);
        check_hub_email_replied(&mut db_conn, &hub, &domain);
    }
}
