use std::error::Error;

use log::info;

use crate::models::zmq::ZmqConfig;

pub fn send_zmq_email_id(id: i32, zmq_config: &ZmqConfig) -> Result<(), Box<dyn Error>> {
    let context = zmq::Context::new();
    let requester = context.socket(zmq::PUSH)?;
    requester.connect(&zmq_config.address)?;

    let buffer = id.to_be_bytes().to_vec();

    requester.send(buffer, 0)?;

    info!("Sent email id: {}", id);

    Ok(())
}
