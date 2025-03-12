#[derive(Clone)]
pub struct ServerConfig {
    pub zmq_address: String,
    pub secret: String,
}
