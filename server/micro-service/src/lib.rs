#[macro_use]
pub mod log;
pub mod cfg;
pub mod error;
pub mod load_balancer;
pub mod service;
pub mod util;
pub use log::{init_console_logger, init_tcp_logger};
use std::time::Instant;

#[derive(Debug, Clone)]
// remote server
pub struct ServerInfo {
    pub enabled: bool,
    pub ctime: Instant,
    pub mtime: Instant,
}
