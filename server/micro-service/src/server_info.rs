use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ServerInfo {
    pub fail_percent: u16,
    pub workload: u32,
    // ip port string
    pub ip: u32,
    pub port: u16,
}

impl ServerInfo {
    pub fn from_json(json: &str) -> ServerInfo {
        serde_json::from_str(json).unwrap_or_default()
    }
    pub fn to_json(self) -> String {
        serde_json::to_string(&self).unwrap_or_else(|_| "{}".to_owned())
    }
}