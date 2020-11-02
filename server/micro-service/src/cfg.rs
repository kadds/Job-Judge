use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize, Serialize)]
pub struct EtcdConfig {
    pub endpoints: Vec<String>,
    pub user: String,
    pub password: String,
    pub prefix: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MicroServiceCommConfig {
    pub log_port: u16,
    pub log_host: String,
    pub etcd: EtcdConfig,
}
