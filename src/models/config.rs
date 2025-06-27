#[derive(Clone)]
pub struct ServerConfig {
    pub zmq_address: String,
    pub secret: String,
    pub auth_service_url: String,
}
