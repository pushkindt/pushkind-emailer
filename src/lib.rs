//! Core library for the pushkind emailer application.
//!
//! This crate exposes the domain types, data models and utilities that power
//! the pushkind emailer service.  The binary in [`main`](../main.rs) builds on
//! top of these modules to provide an HTTP server and background workers.

use std::sync::Arc;

use actix_files::Files;
use actix_identity::IdentityMiddleware;
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use actix_web::{App, HttpServer, middleware, web};
use actix_web_flash_messages::{FlashMessagesFramework, storage::CookieMessageStore};
use pushkind_common::db::establish_connection_pool;
use pushkind_common::middleware::RedirectUnauthorized;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{logout, not_assigned};
use pushkind_common::zmq::{ZmqSender, ZmqSenderOptions};
use tera::Tera;

use crate::models::config::ServerConfig;
use crate::repository::DieselRepository;
use crate::routes::groups::{groups_add, groups_assign, groups_delete, groups_modal, groups_show};
use crate::routes::main::{
    delete_email, export_email_recipients, index, resend_email, send_email, track_email,
};
use crate::routes::recipients::{
    recipients_add, recipients_clean, recipients_delete, recipients_modal, recipients_save,
    recipients_show, recipients_source, recipients_upload,
};
use crate::routes::settings::{
    history_download, history_show, settings_save, settings_show, unsubscribed_show,
};

pub mod domain;
pub mod dto;
pub mod forms;
pub mod models;
pub mod repository;
pub mod routes;
pub mod schema;
pub mod services;
pub mod utils;

pub const SERVICE_ACCESS_ROLE: &str = "emailer";
pub const SERVICE_ADMIN_ROLE: &str = "admin";

/// Builds and runs the Actix-Web HTTP server using the provided configuration.
pub async fn run(server_config: ServerConfig) -> std::io::Result<()> {
    let common_config = CommonServerConfig {
        auth_service_url: server_config.auth_service_url.to_string(),
        secret: server_config.secret.clone(),
    };

    // Start a background ZeroMQ publisher used for outbound email notifications.
    let zmq_sender = ZmqSender::start(ZmqSenderOptions::pub_default(
        &server_config.zmq_emailer_pub,
    ))
    .map_err(|e| std::io::Error::other(format!("Failed to start ZMQ sender: {e}")))?;

    let zmq_sender = Arc::new(zmq_sender);

    // Establish Diesel connection pool for the SQLite database.
    let pool = establish_connection_pool(&server_config.database_url).map_err(|e| {
        std::io::Error::other(format!("Failed to establish database connection: {e}"))
    })?;

    let repo = DieselRepository::new(pool);

    // Keys and stores for identity, sessions, and flash messages.
    let secret_key = Key::from(server_config.secret.as_bytes());

    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();

    let tera = Tera::new(&server_config.templates_dir)
        .map_err(|e| std::io::Error::other(format!("Template parsing error(s): {e}")))?;

    let bind_address = (server_config.address.clone(), server_config.port);

    HttpServer::new(move || {
        App::new()
            .wrap(message_framework.clone())
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(false) // set to true in prod
                    .cookie_domain(Some(format!(".{}", server_config.domain)))
                    .build(),
            )
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .service(Files::new("/assets", "./assets"))
            .service(
                web::scope("")
                    .wrap(RedirectUnauthorized)
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
                    .service(recipients_add)
                    .service(recipients_delete)
                    .service(recipients_clean)
                    .service(recipients_upload)
                    .service(recipients_modal)
                    .service(recipients_save)
                    .service(recipients_source)
                    .service(groups_show)
                    .service(groups_add)
                    .service(groups_delete)
                    .service(groups_assign)
                    .service(groups_modal)
                    .service(unsubscribed_show)
                    .service(history_show)
                    .service(history_download),
            )
            .app_data(web::Data::new(tera.clone()))
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(server_config.clone()))
            .app_data(web::Data::new(common_config.clone()))
            .app_data(web::Data::new(zmq_sender.clone()))
    })
    .bind(bind_address)?
    .run()
    .await
}
