use std::env;
use std::{thread, time};

use dotenvy::dotenv;
use log::{error, info};

fn send_email(email_id: i32) -> Result<(), String> {
    println!("Sending email for email_id: {}", email_id);
    thread::sleep(time::Duration::from_secs(5));
    println!("Email sent for email_id: {}", email_id);
    Ok(())
}

fn main() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    dotenv().ok(); // Load .env file

    let zmq_address = env::var("ZMQ_ADDRESS").unwrap_or("tcp://127.0.0.1:5555".to_string());

    let context = zmq::Context::new();
    let responder = context.socket(zmq::PULL).expect("Cannot create zmq socket");
    responder
        .bind(&zmq_address)
        .expect("Cannot bind to zmq port");

    info!("Starting email worker");
    loop {
        let mut buffer = [0; 4];
        match responder.recv_into(&mut buffer, 0) {
            Ok(_) => {
                let email_id: i32 = i32::from_be_bytes(buffer);
                if let Err(e) = send_email(email_id) {
                    error!("Error receiving message: {}", e);
                }
            }
            Err(e) => {
                error!("Error receiving message: {}", e);
                continue;
            }
        }
    }
}
