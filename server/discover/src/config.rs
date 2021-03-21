use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Serialize, Deserialize)]
struct Server {
    address: String,
}

#[derive(Serialize, Deserialize)]
struct Module {
    servers: HashMap<String, Server>,
}

#[derive(Serialize, Deserialize)]
struct Config {
    modules: HashMap<String, Module>,
}

pub struct ConfigDiscover {
    config_file: String,
}

impl ConfigDiscover {
    pub fn new(config_file: String) -> Self {
        ConfigDiscover { config_file }
    }
}

#[async_trait]
impl Discover for ConfigDiscover {
    async fn get_from_module(&self, module_name: &str) -> Result<Vec<(String, SocketAddr)>> {
        let bytes = tokio::fs::read(&self.config_file).await?;
        let config: Config = toml::from_slice(&bytes)?;
        if let Some(v) = config.modules.get(module_name) {
            return v
                .servers
                .iter()
                .map(|v| {
                    v.1.address
                        .parse()
                        .map(|s| (v.0.to_owned(), s))
                        .map_err(|e| Error::new(std::io::ErrorKind::InvalidData, e))
                })
                .collect();
        }
        Err(Error::from(std::io::ErrorKind::InvalidInput))
    }
    async fn get_from_server(
        &self,
        module_name: &str,
        server_name: &str,
    ) -> Result<Option<SocketAddr>> {
        let bytes = tokio::fs::read(&self.config_file).await?;
        let config: Config = toml::from_slice(&bytes)?;
        if let Some(v) = config.modules.get(module_name) {
            return if let Some(v) = v.servers.get(server_name) {
                Ok(Some(v.address.parse().map_err(|e| {
                    Error::new(std::io::ErrorKind::InvalidData, e)
                })?))
            } else {
                Ok(None)
            };
        }
        Err(Error::from(std::io::ErrorKind::InvalidInput))
    }
}
