use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EtcdConfig {
    pub endpoints: Vec<String>,
    pub username: String,
    pub password: String,
    pub prefix: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UserConfig {
    #[serde(default = "default_username")]
    pub username: String,
    #[serde(default = "default_password")]
    pub password: String,
    #[serde(default = "default_no_verify")]
    pub no_verify: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CommConfig {
    #[serde(default = "default_port")]
    pub port: u16,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub etcd: EtcdConfig,
    #[serde(default)]
    pub user: UserConfig,
    #[serde(default)]
    pub comm: CommConfig,
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

impl Default for CommConfig {
    fn default() -> Self {
        CommConfig {
            port: default_port(),
        }
    }
}

impl Default for UserConfig {
    fn default() -> Self {
        UserConfig {
            username: default_username(),
            password: default_password(),
            no_verify: default_no_verify(),
        }
    }
}
