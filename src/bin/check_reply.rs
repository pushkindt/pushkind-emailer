use std::env;
use std::str;
use std::sync::Arc;

use dotenvy::dotenv;
use pushkind_common::db::establish_connection_pool;
use pushkind_common::domain::email::UpdateEmailRecipient;

use pushkind_emailer::domain::hub::Hub;
use pushkind_emailer::repository::{DieselRepository, EmailReader, EmailWriter, HubReader};
use tokio::task;

fn extract_recipient_id(header: &str, domain: &str) -> Option<i32> {
    header
        .lines()
        .find(|line| line.starts_with("In-Reply-To:"))
        .and_then(|line| line.split('<').nth(1))
        .and_then(|part| part.split('>').next())
        .and_then(|msg_id| {
            let mut parts = msg_id.split('@');
            match (parts.next(), parts.next()) {
                (Some(id), Some(d)) if d == domain => id.parse().ok(),
                _ => None,
            }
        })
}

fn process_new_message(
    repo: &DieselRepository,
    session: &mut imap::Session<impl std::io::Read + std::io::Write>,
    uid: u32,
    domain: &str,
) {
    let fetches = match session.uid_fetch(uid.to_string(), "RFC822.HEADER") {
        Ok(f) => f,
        Err(e) => {
            log::error!("Cannot fetch header for UID {uid}: {e}");
            return;
        }
    };

    let fetch = match fetches.iter().next() {
        Some(f) => f,
        None => return,
    };

    let header = match fetch.header() {
        Some(h) => h,
        None => return,
    };

    let header_str = match str::from_utf8(header) {
        Ok(s) => s,
        Err(e) => {
            log::error!("Cannot parse header utf8: {e}");
            return;
        }
    };

    if let Some(recipient_id) = extract_recipient_id(header_str, domain) {
        if let Err(e) = repo.update_recipient(
            recipient_id,
            &UpdateEmailRecipient {
                is_sent: Some(true),
                replied: Some(true),
                opened: Some(true),
            },
        ) {
            log::error!("Cannot set email recipient replied status: {e}");
        } else {
            log::info!("Email recipient replied status set for {recipient_id}");
        }
    }
}

fn monitor_hub(repo: DieselRepository, hub: Hub, domain: String) {
    let (imap_server, imap_port, username, password) =
        match (&hub.imap_server, hub.imap_port, &hub.login, &hub.password) {
            (Some(server), Some(port), Some(username), Some(password)) => {
                (server, port as u16, username, password)
            }
            _ => {
                log::error!("Cannot get imap server and port for the hub#{}", hub.id);
                return;
            }
        };

    let tls = match native_tls::TlsConnector::builder().build() {
        Ok(tls) => tls,
        Err(e) => {
            log::error!("Cannot build tls connector for hub#{}: {e}", hub.id);
            return;
        }
    };
    let client = match imap::connect((imap_server.as_str(), imap_port), imap_server, &tls) {
        Ok(client) => client,
        Err(e) => {
            log::error!("Cannot connect to imap server in hub#{}: {e}", hub.id);
            return;
        }
    };

    let mut session = match client.login(username, password).map_err(|e| e.0) {
        Ok(session) => session,
        Err(e) => {
            log::error!("Cannot login to imap server in hub#{}: {e}", hub.id);
            return;
        }
    };

    if let Err(e) = session.select("INBOX") {
        log::error!("Cannot select INBOX in hub#{}: {e}", hub.id);
        return;
    }

    let recipients = match repo.list_not_replied_email_recipients(hub.id) {
        Ok(r) => r,
        Err(e) => {
            log::error!("Cannot get recipients in hub#{}: {e}", hub.id);
            Vec::new()
        }
    };

    log::info!(
        "Found {} recipients for the startup check in hub#{}",
        recipients.len(),
        hub.id,
    );
    for recipient in recipients {
        let in_reply_to_id = format!("<{}@{}>", recipient.id, domain);
        let query = format!("HEADER In-Reply-To {in_reply_to_id}");
        let search_result = match session.search(&query) {
            Ok(res) => res,
            Err(e) => {
                log::error!("Cannot search for emails in hub#{}: {e}", hub.id);
                continue;
            }
        };

        if !search_result.is_empty() {
            if let Err(e) = repo.update_recipient(
                recipient.id,
                &UpdateEmailRecipient {
                    is_sent: Some(true),
                    replied: Some(true),
                    opened: Some(true),
                },
            ) {
                log::error!("Cannot set email recipient replied status: {e}");
            } else {
                log::info!(
                    "Email recipient replied status set for {}, email id: {}",
                    &recipient.address,
                    recipient.email_id
                );
            }
        }
    }

    let mut last_uid = session
        .uid_search("ALL")
        .ok()
        .and_then(|uids| uids.into_iter().max())
        .unwrap_or(0);

    log::info!("Starting a monitoring loop for hub#{}", hub.id);
    loop {
        if let Err(e) = session.idle().and_then(|idle| idle.wait_keepalive()) {
            log::error!("Idle error in hub#{}: {e}", hub.id);
            break;
        }

        let search_query = format!("UID {}:*", last_uid + 1);
        let new_uids = match session.uid_search(&search_query) {
            Ok(uids) => uids,
            Err(e) => {
                log::error!("Cannot search new emails in hub#{}: {e}", hub.id);
                continue;
            }
        };

        for uid in &new_uids {
            process_new_message(&repo, &mut session, *uid, &domain);
        }

        if let Some(max_uid) = new_uids.iter().max() {
            last_uid = *max_uid;
        }
    }

    if let Err(e) = session.logout() {
        log::error!("Cannot logout from hub#{}: {e}", hub.id);
    }
}

#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").unwrap_or_else(|_| "app.db".to_string());
    let domain = Arc::new(env::var("DOMAIN").unwrap_or_default());

    let db_pool = match establish_connection_pool(&database_url) {
        Ok(pool) => pool,
        Err(e) => {
            log::error!("Cannot establish db connection: {e}");
            return;
        }
    };

    let repo = DieselRepository::new(db_pool);

    let hubs = match repo.list_hubs() {
        Ok(h) => h,
        Err(e) => {
            log::error!("Cannot get hubs: {e}");
            return;
        }
    };

    let mut handles = vec![];
    for hub in hubs {
        let repo = repo.clone();
        let domain = Arc::clone(&domain);
        handles.push(task::spawn_blocking(move || {
            monitor_hub(repo, hub, domain.to_string())
        }));
    }

    for handle in handles {
        match handle.await {
            Ok(_) => (), // task finished fine
            Err(e) => log::error!("Task panicked: {e:?}"),
        }
    }
}
