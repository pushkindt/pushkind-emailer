use lazy_static::lazy_static;
use tera::Tera;

pub mod db;
pub mod forms;
pub mod middleware;
pub mod models;
pub mod repository;
pub mod routes;
pub mod schema;
pub mod utils;

lazy_static! {
    pub static ref TEMPLATES: Tera = {
        match Tera::new("templates/**/*") {
            Ok(t) => t,
            Err(e) => {
                println!("Parsing error(s): {}", e);
                ::std::process::exit(1);
            }
        }
    };
}
