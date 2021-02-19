use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize, Serialize)]
pub struct EtcdConfig {
    pub endpoints: Vec<String>,
    pub username: String,
    pub password: String,
    pub prefix: String,
    pub ttl: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum LogType {
    Tcp,
    Console,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Comm {
    pub log_port: u16,
    pub log_host: String,
    pub log_type: LogType,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Database {
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MicroServiceCommConfig {
    pub comm: Comm,
    pub etcd: EtcdConfig,
    pub database: Database,
}
