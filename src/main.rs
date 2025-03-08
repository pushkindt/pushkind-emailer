use std::env;

use actix_files::Files;
use actix_identity::IdentityMiddleware;
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use actix_web::{App, HttpServer, middleware, web};
use dotenvy::dotenv;

use pushkind_emailer::db::establish_connection_pool;
use pushkind_emailer::middleware::RedirectUnauthorized;
use pushkind_emailer::routes::auth::{login, logout, signin, signup};
use pushkind_emailer::routes::main::index;
use pushkind_emailer::routes::recipients::{
    recipients, recipients_add, recipients_assign, recipients_clean, recipients_delete,
    recipients_group_add, recipients_group_delete, recipients_unassign,
};
use pushkind_emailer::routes::settings::{
    settings, settings_activate, settings_add, settings_save,
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    dotenv().ok(); // Load .env file
    let database_url = env::var("DATABASE_URL").unwrap_or("app.db".to_string());
    let port = env::var("PORT").unwrap_or("8080".to_string());
    let port = port.parse::<u16>().expect("PORT must be a number");
    let address = env::var("ADDRESS").unwrap_or("127.0.0.1".to_string());

    let pool = establish_connection_pool(database_url);

    let secret_key = env::var("SECRET_KEY");
    let secret_key = match secret_key {
        Ok(key) => Key::from(key.as_bytes()),
        Err(_) => Key::generate(),
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
                    .service(signup), // .service(register),
            )
            .service(Files::new("/assets", "./assets"))
            .service(
                web::scope("")
                    .wrap(RedirectUnauthorized)
                    .service(index)
                    .service(recipients)
                    .service(settings)
                    .service(settings_add)
                    .service(settings_activate)
                    .service(settings_save)
                    .service(recipients_add)
                    .service(recipients_delete)
                    .service(recipients_group_add)
                    .service(recipients_group_delete)
                    .service(recipients_assign)
                    .service(recipients_unassign)
                    .service(recipients_clean),
            )
            .app_data(web::Data::new(pool.clone()))
    })
    .bind((address, port))?
    .run()
    .await
}
