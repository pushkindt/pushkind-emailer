//! Binary that launches the HTTP server for the pushkind emailer application.

use std::env;
use std::sync::Arc;

use actix_files::Files;
use actix_identity::IdentityMiddleware;
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use actix_web::{App, HttpServer, middleware, web};
use actix_web_flash_messages::{FlashMessagesFramework, storage::CookieMessageStore};
use dotenvy::dotenv;
use log::error;
use pushkind_common::db::establish_connection_pool;
use pushkind_common::middleware::RedirectUnauthorized;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::{logout, not_assigned};
use pushkind_common::zmq::{ZmqSender, ZmqSenderOptions};
use tera::Tera;

use pushkind_emailer::models::config::ServerConfig;
use pushkind_emailer::repository::DieselRepository;
use pushkind_emailer::routes::groups::{
    groups_add, groups_assign, groups_delete, groups_modal, groups_show, groups_unassign,
};
use pushkind_emailer::routes::main::{
    delete_email, export_email_recipients, index, resend_email, send_email, track_email,
};
use pushkind_emailer::routes::recipients::{
    recipients_add, recipients_clean, recipients_delete, recipients_modal, recipients_save,
    recipients_show, recipients_source, recipients_upload,
};
use pushkind_emailer::routes::settings::{settings_save, settings_show};

/// Application entry point launching the Actix Web server.
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    dotenv().ok(); // Load .env file
    let database_url = env::var("DATABASE_URL").unwrap_or("app.db".to_string());
    let port = env::var("PORT").unwrap_or("8080".to_string());
    let port = port.parse::<u16>().unwrap_or(8080);
    let address = env::var("ADDRESS").unwrap_or("127.0.0.1".to_string());
    let zmq_address = env::var("ZMQ_EMAILER_PUB").unwrap_or("tcp://127.0.0.1:5557".to_string());
    let crm_service_url = env::var("CRM_SERVICE_URL").unwrap_or_default();

    let pool = match establish_connection_pool(&database_url) {
        Ok(pool) => pool,
        Err(e) => {
            error!("Failed to establish database connection: {e}");
            std::process::exit(1);
        }
    };
    let repo = DieselRepository::new(pool);

    let secret = env::var("SECRET_KEY");
    let secret_key = match &secret {
        Ok(key) => Key::from(key.as_bytes()),
        Err(_) => Key::generate(),
    };

    let auth_service_url = env::var("AUTH_SERVICE_URL");
    let auth_service_url = match auth_service_url {
        Ok(auth_service_url) => auth_service_url,
        Err(_) => {
            error!("AUTH_SERVICE_URL environment variable not set");
            std::process::exit(1);
        }
    };

    let zmq_sender = Arc::new(ZmqSender::start(ZmqSenderOptions::pub_default(
        &zmq_address,
    )));

    let server_config = ServerConfig { crm_service_url };
    let common_config = CommonServerConfig {
        secret: secret.unwrap_or_default(),
        auth_service_url,
    };

    let domain = env::var("DOMAIN").unwrap_or("localhost".to_string());

    let message_store = CookieMessageStore::builder(secret_key.clone()).build();
    let message_framework = FlashMessagesFramework::builder(message_store).build();

    let tera = match Tera::new("templates/**/*") {
        Ok(t) => t,
        Err(e) => {
            log::error!("Parsing error(s): {e}");
            std::process::exit(1);
        }
    };

    HttpServer::new(move || {
        App::new()
            .wrap(message_framework.clone())
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(false) // set to true in prod
                    .cookie_domain(Some(format!(".{domain}")))
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
                    .service(groups_unassign),
            )
            .app_data(web::Data::new(tera.clone()))
            .app_data(web::Data::new(repo.clone()))
            .app_data(web::Data::new(server_config.clone()))
            .app_data(web::Data::new(common_config.clone()))
            .app_data(web::Data::new(zmq_sender.clone()))
    })
    .bind((address, port))?
    .run()
    .await
}
