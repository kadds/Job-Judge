use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct Config {
    #[serde(default = "default_dns_name_server")]
    pub discover_name_server: String,
    #[serde(default = "default_dns_suffix")]
    pub discover_suffix: String,
    pub discover_file: String,
    // user verify
    #[serde(default = "default_username")]
    pub verify_username: String,
    #[serde(default = "default_password")]
    pub verify_password: String,
    #[serde(default = "default_no_verify")]
    pub no_verify: bool,

    #[serde(default = "default_port")]
    pub bind_port: u16,

    #[serde(default = "default_modules")]
    pub modules: Vec<String>,
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
    true
}

fn default_dns_name_server() -> String {
    "".to_owned()
}

fn default_dns_suffix() -> String {
    "cluster.local".to_owned()
}

fn default_modules() -> Vec<String> {
    vec!["gateway", "idsvr", "sessionsvr", "usersvr"]
        .into_iter()
        .map(|v| v.to_owned())
        .collect()
}
