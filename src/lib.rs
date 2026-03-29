//! Core library for the pushkind emailer application.
//!
//! This crate exposes the domain types, data models and utilities that power
//! the pushkind emailer service.  The binary in [`main`](../main.rs) builds on
//! top of these modules to provide an HTTP server and background workers.

#[cfg(feature = "server")]
use std::{net::TcpListener, sync::Arc};

#[cfg(feature = "server")]
use crate::models::config::{AppConfig, Settings};
#[cfg(feature = "server")]
use crate::repository::DieselRepository;
#[cfg(feature = "server")]
use crate::routes::api::{
    api_v1_email_history, api_v1_emails, api_v1_group, api_v1_groups, api_v1_hub_settings,
    api_v1_iam, api_v1_no_access, api_v1_recipient, api_v1_recipients,
    api_v1_unsubscribed_recipients,
};
#[cfg(feature = "server")]
use crate::routes::aux::not_assigned;
#[cfg(feature = "server")]
use crate::routes::groups::{group_add, group_assign, group_delete, groups_show};
#[cfg(feature = "server")]
use crate::routes::main::{
    delete_email, export_email_recipients, index, resend_email, send_email, track_email,
};
#[cfg(feature = "server")]
use crate::routes::recipients::{
    recipient_add, recipient_save, recipients_clean, recipients_delete, recipients_show,
    recipients_source, recipients_upload,
};
#[cfg(feature = "server")]
use crate::routes::settings::{
    history_download, history_show, settings_save, settings_show, unsubscribed_show,
};
#[cfg(feature = "server")]
use actix_files::Files;
#[cfg(feature = "server")]
use actix_identity::IdentityMiddleware;
#[cfg(feature = "server")]
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
#[cfg(feature = "server")]
use actix_web::cookie::Key;
#[cfg(feature = "server")]
use actix_web::{App, HttpServer, dev::Server, middleware, web};
#[cfg(feature = "server")]
use pushkind_common::db::establish_connection_pool;
#[cfg(feature = "server")]
use pushkind_common::middleware::RedirectUnauthorized;
#[cfg(feature = "server")]
use pushkind_common::models::config::CommonServerConfig;
#[cfg(feature = "server")]
use pushkind_common::routes::logout;
#[cfg(feature = "server")]
use pushkind_common::zmq::{ZmqSender, ZmqSenderOptions};

#[cfg(feature = "data")]
pub mod domain;
#[cfg(feature = "server")]
pub mod dto;
#[cfg(feature = "server")]
pub mod frontend;

mod error_conversions;
#[cfg(feature = "server")]
pub mod forms;
#[cfg(feature = "data")]
pub mod models;
#[cfg(feature = "server")]
pub mod repository;
#[cfg(feature = "server")]
pub mod routes;
#[cfg(feature = "data")]
pub mod schema;
#[cfg(feature = "server")]
pub mod services;
#[cfg(feature = "server")]
pub mod utils;

pub const SERVICE_ACCESS_ROLE: &str = "emailer";
pub const SERVICE_ADMIN_ROLE: &str = "admin";

/// Builds and runs the Actix-Web HTTP server using the provided configuration.
#[cfg(feature = "server")]
pub async fn run(settings: Settings) -> std::io::Result<()> {
    let bind_address = (settings.server.address.clone(), settings.server.port);
    let listener = TcpListener::bind(bind_address)?;

    build_server(listener, settings.app)?.await
}

/// Builds an Actix-Web HTTP server on a pre-bound listener.
#[cfg(feature = "server")]
pub fn build_server(listener: TcpListener, app_config: AppConfig) -> std::io::Result<Server> {
    let common_config = CommonServerConfig {
        auth_service_url: app_config.auth_service_url.to_string(),
        secret: app_config.secret.clone(),
    };

    // Start a background ZeroMQ publisher used for outbound email notifications.
    let zmq_sender =
        ZmqSender::start(ZmqSenderOptions::pub_default(&app_config.zmq_emailer_pub))
            .map_err(|e| std::io::Error::other(format!("Failed to start ZMQ sender: {e}")))?;

    let zmq_sender = Arc::new(zmq_sender);

    // Establish Diesel connection pool for the SQLite database.
    let pool = establish_connection_pool(&app_config.database_url).map_err(|e| {
        std::io::Error::other(format!("Failed to establish database connection: {e}"))
    })?;

    let repo = DieselRepository::new(pool);

    // Keys and stores for identity and sessions.
    let secret_key = Key::from(app_config.secret.as_bytes());

    let server = HttpServer::new(move || {
        App::new()
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(false) // set to true in prod
                    .cookie_domain(Some(format!(".{}", app_config.domain)))
                    .build(),
            )
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .service(Files::new("/assets", "./assets"))
            .service(
                web::scope("")
                    .wrap(RedirectUnauthorized)
                    .service(
                        web::scope("/api")
                            .service(api_v1_iam)
                            .service(api_v1_emails)
                            .service(api_v1_recipients)
                            .service(api_v1_recipient)
                            .service(api_v1_groups)
                            .service(api_v1_group)
                            .service(api_v1_hub_settings)
                            .service(api_v1_unsubscribed_recipients)
                            .service(api_v1_email_history)
                            .service(api_v1_no_access),
                    )
                    .service(not_assigned)
                    .service(logout)
                    .service(index)
                    .service(send_email)
                    .service(resend_email)
                    .service(delete_email)
                    .service(track_email)
                    .service(export_email_recipients)
                    .service(settings_show)
                    .service(settings_save)
                    .service(recipients_show)
                    .service(recipient_add)
                    .service(recipients_delete)
                    .service(recipients_clean)
                    .service(recipients_upload)
                    .service(recipient_save)
                    .service(recipients_source)
                    .service(groups_show)
                    .service(group_add)
                    .service(group_delete)
                    .service(group_assign)
                    .service(unsubscribed_show)
                    .service(history_show)
                    .service(history_download),
            )
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(app_config.clone()))
            .app_data(web::Data::new(common_config.clone()))
            .app_data(web::Data::new(zmq_sender.clone()))
    })
    .listen(listener)?
    .run();

    Ok(server)
}
