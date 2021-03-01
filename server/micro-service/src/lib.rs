pub mod cfg;
pub mod error;
pub mod load_balancer;
pub mod service;
pub mod util;
use std::time::Instant;

#[derive(Debug, Clone)]
// remote server
pub struct ServerInfo {
    pub enabled: bool,
    pub ctime: Instant,
    pub mtime: Instant,
}
