pub mod breaker;
pub mod cfg;
pub mod discover;
pub mod error;
pub mod server;
pub mod service;
pub mod util;

pub use server::Server;
use std::time::Instant;

#[derive(Debug, Clone)]
// remote server
pub struct ServerInfo {
    pub enabled: bool,
    pub ctime: Instant,
    pub mtime: Instant,
}
