use std::env;

use dotenvy::dotenv;
use pushkind_common::db::establish_connection_pool;
use pushkind_common::domain::email::UpdateEmailRecipient;

use pushkind_emailer::domain::hub::Hub;
use pushkind_emailer::repository::{DieselRepository, EmailReader, EmailWriter, HubReader};

pub fn check_hub_email_replied(repo: DieselRepository, hub: &Hub, domain: &str) {
    let recipients = match repo.list_emails_not_replied_recipients(hub.id) {
        Ok(recipients) => recipients,
        Err(e) => {
            log::error!("Cannot get recipients: {e}");
            return;
        }
    };

    let (imap_server, imap_port, username, password) =
        match (&hub.imap_server, hub.imap_port, &hub.login, &hub.password) {
            (Some(server), Some(port), Some(username), Some(password)) => {
                (server, port, username, password)
            }
            _ => {
                log::error!("Cannot get imap server and port for the hub");
                return;
            }
        };

    let imap_port = imap_port as u16;

    let tls = match native_tls::TlsConnector::builder().build() {
        Ok(tls) => tls,
        Err(e) => {
            log::error!("Cannot build tls connector: {e}");
            return;
        }
    };
    let client = match imap::connect((imap_server.as_str(), imap_port), imap_server, &tls) {
        Ok(client) => client,
        Err(e) => {
            log::error!("Cannot connect to imap server: {e}");
            return;
        }
    };

    let mut session: imap::Session<_> = match client.login(username, password).map_err(|e| e.0) {
        Ok(session) => session,
        Err(e) => {
            log::error!("Cannot login to imap server: {e}");
            return;
        }
    };

    match session.select("INBOX") {
        Ok(_) => log::info!("Selected INBOX"),
        Err(e) => {
            log::error!("Cannot select INBOX: {e}");
            return;
        }
    }

    for recipient in recipients {
        // Define the In-Reply-To Message-ID you are looking for
        let in_reply_to_id = format!("<{}@{}>", recipient.id, domain);

        // Search for emails with a matching In-Reply-To header
        let query = format!("HEADER In-Reply-To {in_reply_to_id}");
        let search_result = match session.search(&query) {
            Ok(search_result) => search_result,
            Err(e) => {
                log::error!("Cannot search for emails: {e}");
                continue;
            }
        };

        if search_result.is_empty() {
            log::info!(
                "No matching emails found for email_id: {}, recipient: {}.",
                recipient.email_id,
                recipient.address
            );
        } else {
            log::info!("Found emails with In-Reply-To {in_reply_to_id}: {search_result:?}");
            match repo.update_recipient(
                recipient.id,
                &UpdateEmailRecipient {
                    is_sent: Some(true),
                    replied: Some(true),
                    opened: Some(true),
                },
            ) {
                Ok(_) => log::info!("Email recipient replied status set"),
                Err(e) => log::error!("Cannot set email recipient replied status: {e}"),
            }
        }
    }

    match session.logout() {
        Ok(_) => log::info!("Logged out"),
        Err(e) => log::error!("Cannot logout: {e}"),
    }
}

fn main() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    dotenv().ok(); // Load .env file

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "app.db".to_string());
    let domain = env::var("DOMAIN").unwrap_or_default();

    let db_pool = match establish_connection_pool(&database_url) {
        Ok(pool) => pool,
        Err(e) => {
            log::error!("Cannot establish db connection: {e}");
            return;
        }
    };

    let repo = DieselRepository::new(db_pool);

    let hubs = match repo.list_hubs() {
        Ok(hub) => hub,
        Err(e) => {
            log::error!("Cannot get hubs: {e}");
            return;
        }
    };

    for hub in hubs {
        log::info!("Checking hub: {}", hub.id);
        check_hub_email_replied(repo.clone(), &hub, &domain);
    }
}
