#[macro_use]
pub mod log;
pub mod cfg;
pub mod error;
pub mod load_balancer;
pub mod service;
pub mod tool;
pub use log::{init_console_logger, init_tcp_logger};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default)]
// remote server
pub struct ServerInfo {
    // ip port string
    pub address: String,
}

impl ServerInfo {
    pub fn from_json(json: &str) -> ServerInfo {
        serde_json::from_str(json).unwrap_or_default()
    }
    pub fn to_json(self) -> String {
        serde_json::to_string(&self).unwrap_or_else(|_| "{}".to_owned())
    }
}
