#![allow(dead_code)]

//! Helpers for integration tests.

use std::net::TcpListener;
use std::sync::Arc;
use std::time::Duration;

use actix_files::Files;
use actix_identity::{Identity, IdentityMiddleware};
use actix_session::{SessionMiddleware, storage::CookieSessionStore};
use actix_web::cookie::Key;
use actix_web::rt::time::sleep;
use actix_web::{
    App, HttpMessage, HttpRequest, HttpResponse, HttpServer, Responder, middleware, post, web,
};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use pushkind_common::db::{DbPool, establish_connection_pool};
use pushkind_common::domain::auth::AuthenticatedUser;
use pushkind_common::middleware::RedirectUnauthorized;
use pushkind_common::models::config::CommonServerConfig;
use pushkind_common::routes::logout;
use pushkind_common::zmq::{ZmqSender, ZmqSenderOptions};
use reqwest::{Client, StatusCode, redirect::Policy};
use tempfile::NamedTempFile;

use pushkind_emailer::models::config::AppConfig;
use pushkind_emailer::repository::DieselRepository;
use pushkind_emailer::routes::api::{
    api_v1_email_history, api_v1_emails, api_v1_group, api_v1_groups, api_v1_hub_settings,
    api_v1_iam, api_v1_no_access, api_v1_recipient, api_v1_recipients,
    api_v1_unsubscribed_recipients,
};
use pushkind_emailer::routes::aux::not_assigned;
use pushkind_emailer::routes::groups::{group_add, group_assign, group_delete, groups_show};
use pushkind_emailer::routes::main::{
    delete_email, export_email_recipients, index, resend_email, send_email, track_email,
};
use pushkind_emailer::routes::recipients::{
    recipient_add, recipient_save, recipients_clean, recipients_delete, recipients_show,
    recipients_source, recipients_upload,
};
use pushkind_emailer::routes::settings::{
    history_download, history_show, settings_save, settings_show, unsubscribed_show,
};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!(); // assumes migrations/ exists
pub const HUB_ID: i32 = 11;

/// Temporary database used in integration tests.
pub struct TestDb {
    _tempfile: NamedTempFile,
    pool: DbPool,
}

pub struct TestApp {
    test_db: TestDb,
    address: String,
}

impl TestDb {
    pub fn new() -> Self {
        let tempfile = NamedTempFile::new().expect("Failed to create temp file");
        let pool = establish_connection_pool(tempfile.path().to_str().unwrap())
            .expect("Failed to establish SQLite connection.");
        let mut conn = pool
            .get()
            .expect("Failed to get SQLite connection from pool.");
        conn.run_pending_migrations(MIGRATIONS)
            .expect("Migrations failed");
        TestDb {
            _tempfile: tempfile,
            pool,
        }
    }

    pub fn pool(&self) -> DbPool {
        self.pool.clone()
    }

    pub fn get_db_path(&self) -> String {
        self._tempfile.path().to_str().unwrap().to_string()
    }
}

impl TestApp {
    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn db_pool(&self) -> DbPool {
        self.test_db.pool()
    }

    pub fn repo(&self) -> DieselRepository {
        DieselRepository::new(self.db_pool())
    }
}

#[derive(serde::Deserialize)]
struct LoginRequest {
    hub_id: i32,
    email: String,
    name: String,
    roles: Vec<String>,
}

#[post("/test/login")]
async fn test_login(
    request: HttpRequest,
    payload: web::Json<LoginRequest>,
    common_config: web::Data<CommonServerConfig>,
) -> impl Responder {
    let mut user = AuthenticatedUser {
        sub: payload.email.clone(),
        email: payload.email.clone(),
        hub_id: payload.hub_id,
        name: payload.name.clone(),
        roles: payload.roles.clone(),
        exp: 0,
    };
    user.set_expiration(7);

    let token = user
        .to_jwt(&common_config.secret)
        .expect("JWT generation should succeed for test users.");
    Identity::login(&request.extensions(), token).expect("Test login should persist identity.");

    HttpResponse::Ok().finish()
}

#[actix_web::get("/test/recipient-source")]
async fn test_recipient_source(request: HttpRequest) -> impl Responder {
    if request.cookie("id").is_none() {
        return HttpResponse::Unauthorized().finish();
    }

    HttpResponse::Ok().json(serde_json::json!([
        {
            "name": "Source User",
            "email": "source@example.com",
            "fields": {
                "region": "EMEA"
            },
            "groups": ["Source Group"]
        },
        {
            "name": "Missing Email",
            "email": null
        }
    ]))
}

fn random_zmq_endpoint() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to reserve a local port.");
    let port = listener
        .local_addr()
        .expect("Failed to read the local socket address.")
        .port();
    drop(listener);

    format!("tcp://127.0.0.1:{port}")
}

async fn wait_until_server_is_ready(address: &str) {
    let client = Client::builder()
        .redirect(Policy::none())
        .timeout(Duration::from_millis(100))
        .build()
        .expect("Failed to create the test HTTP client.");
    let url = format!("{address}/");

    for _ in 0..20 {
        match client.get(&url).send().await {
            Ok(response)
                if response.status() == StatusCode::SEE_OTHER
                    || response.status() == StatusCode::OK =>
            {
                return;
            }
            Ok(_) | Err(_) => sleep(Duration::from_millis(25)).await,
        }
    }

    panic!("Test server did not become ready at {url}");
}

pub async fn spawn_app() -> TestApp {
    let test_db = TestDb::new();
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind a random local port.");
    let port = listener
        .local_addr()
        .expect("Failed to read the local socket address.")
        .port();

    let zmq_emailer_pub = random_zmq_endpoint();
    let app_config = AppConfig {
        domain: "localhost".to_string(),
        database_url: test_db.get_db_path(),
        zmq_emailer_pub: zmq_emailer_pub.clone(),
        zmq_emailer_sub: random_zmq_endpoint(),
        zmq_replier_pub: random_zmq_endpoint(),
        zmq_replier_sub: random_zmq_endpoint(),
        secret: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        auth_service_url: "https://users.pushkind.test/auth/signin".to_string(),
        crm_service_url: "https://crm.pushkind.test".to_string(),
        files_service_url: "https://files.pushkind.test".to_string(),
    };
    let common_config = CommonServerConfig {
        auth_service_url: app_config.auth_service_url.clone(),
        secret: app_config.secret.clone(),
    };
    let secret_key = Key::from(app_config.secret.as_bytes());
    let repo = DieselRepository::new(test_db.pool());
    let zmq_sender = Arc::new(
        ZmqSender::start(ZmqSenderOptions::pub_default(&zmq_emailer_pub))
            .expect("Failed to start test ZMQ sender."),
    );

    let server = HttpServer::new(move || {
        App::new()
            .wrap(IdentityMiddleware::default())
            .wrap(
                SessionMiddleware::builder(CookieSessionStore::default(), secret_key.clone())
                    .cookie_secure(false)
                    .build(),
            )
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::default())
            .service(Files::new("/assets", "./assets").prefer_utf8(true))
            .service(test_login)
            .service(test_recipient_source)
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
    .listen(listener)
    .expect("Failed to listen with the test server.")
    .run();

    actix_web::rt::spawn(server);
    let address = format!("http://127.0.0.1:{port}");

    wait_until_server_is_ready(&address).await;

    TestApp { test_db, address }
}

pub fn build_reqwest_client() -> Client {
    Client::builder()
        .cookie_store(true)
        .build()
        .expect("Can't create a request client")
}

pub fn build_no_redirect_client() -> Client {
    Client::builder()
        .cookie_store(true)
        .redirect(Policy::none())
        .build()
        .expect("Can't create a request client")
}

pub async fn login_as(
    client: &Client,
    address: &str,
    email: &str,
    name: &str,
    hub_id: i32,
    roles: &[&str],
) {
    let response = client
        .post(format!("{address}/test/login"))
        .json(&serde_json::json!({
            "hub_id": hub_id,
            "email": email,
            "name": name,
            "roles": roles,
        }))
        .send()
        .await
        .expect("Failed to submit test login.");

    assert_eq!(response.status(), StatusCode::OK);
}
