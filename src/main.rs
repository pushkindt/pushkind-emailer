use std::env;

use actix_files::Files;
use actix_identity::IdentityMiddleware;
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use actix_web::{App, HttpServer, middleware, web};
use dotenvy::dotenv;

use log::error;
use pushkind_emailer::db::establish_connection_pool;
use pushkind_emailer::middleware::RedirectUnauthorized;
use pushkind_emailer::models::config::ServerConfig;
use pushkind_emailer::routes::auth::{login, logout, register, signin, signup};
use pushkind_emailer::routes::files::{file_manager, upload_image};
use pushkind_emailer::routes::groups::{
    groups, groups_add, groups_assign, groups_delete, groups_unassign,
};
use pushkind_emailer::routes::main::{delete_email, index, retry_email, send_email, track_email};
use pushkind_emailer::routes::recipients::{
    recipients, recipients_add, recipients_clean, recipients_delete, recipients_modal,
    recipients_save, recipients_upload,
};
use pushkind_emailer::routes::settings::{
    settings, settings_activate, settings_add, settings_delete, settings_save,
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    dotenv().ok(); // Load .env file
    let database_url = env::var("DATABASE_URL").unwrap_or("app.db".to_string());
    let port = env::var("PORT").unwrap_or("8080".to_string());
    let port = port.parse::<u16>().unwrap_or(8080);
    let address = env::var("ADDRESS").unwrap_or("127.0.0.1".to_string());
    let zmq_address = env::var("ZMQ_ADDRESS").unwrap_or("tcp://127.0.0.1:5555".to_string());

    let pool = match establish_connection_pool(database_url) {
        Ok(pool) => pool,
        Err(e) => {
            error!("Failed to establish database connection: {}", e);
            std::process::exit(1);
        }
    };

    let secret = env::var("SECRET_KEY");
    let secret_key = match &secret {
        Ok(key) => Key::from(key.as_bytes()),
        Err(_) => Key::generate(),
    };

    let zmq_config = ServerConfig {
        zmq_address,
        secret: secret.unwrap_or_default(),
    };

    HttpServer::new(move || {
        App::new()
            .wrap(IdentityMiddleware::default())
            .wrap(SessionMiddleware::new(
                CookieSessionStore::default(),
                secret_key.clone(),
            ))
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .service(
                web::scope("/auth")
                    .service(logout)
                    .service(login)
                    .service(signin)
                    .service(signup)
                    .service(register),
            )
            .service(Files::new("/assets", "./assets"))
            .service(
                web::scope("")
                    .wrap(RedirectUnauthorized)
                    .service(index)
                    .service(send_email)
                    .service(delete_email)
                    .service(retry_email)
                    .service(track_email)
                    .service(settings)
                    .service(settings_add)
                    .service(settings_activate)
                    .service(settings_save)
                    .service(settings_delete)
                    .service(recipients)
                    .service(recipients_add)
                    .service(recipients_delete)
                    .service(recipients_clean)
                    .service(recipients_upload)
                    .service(recipients_modal)
                    .service(recipients_save)
                    .service(groups)
                    .service(groups_add)
                    .service(groups_delete)
                    .service(groups_assign)
                    .service(groups_unassign)
                    .service(upload_image)
                    .service(
                        Files::new("/upload", "./upload")
                            .show_files_listing()
                            .files_listing_renderer(file_manager),
                    ),
            )
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(zmq_config.clone()))
    })
    .bind((address, port))?
    .run()
    .await
}
