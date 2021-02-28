use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Config {
    #[serde(rename = "etcd_endpoints", skip)]
    pub etcd_endpoints: Vec<String>,
    #[serde(rename = "etcd_username", skip)]
    pub etcd_username: String,
    #[serde(rename = "etcd_password", skip)]
    pub etcd_password: String,
    #[serde(rename = "etcd_prefix", skip)]
    pub etcd_prefix: String,
    // user verify
    #[serde(default = "default_username")]
    pub verify_username: String,
    #[serde(default = "default_password")]
    pub verify_password: String,
    #[serde(default = "default_no_verify")]
    pub no_verify: bool,

    #[serde(default = "default_port")]
    pub port: u16,
}

fn default_username() -> String {
    "admin".to_owned()
}

fn default_password() -> String {
    "12345678".to_owned()
}

fn default_port() -> u16 {
    6550
}

fn default_no_verify() -> bool {
    false
}
