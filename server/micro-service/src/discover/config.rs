use super::{Discover, Error, Result};
use async_trait::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, net::SocketAddr};
#[derive(Serialize, Deserialize)]
struct Server {
    address: String,
}

#[derive(Serialize, Deserialize)]
struct Config {
    modules: HashMap<String, HashMap<String, Server>>,
}

#[derive(Debug)]
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
    async fn list_instances(&self, module_name: &str) -> Result<Vec<(String, SocketAddr)>> {
        let bytes = tokio::fs::read(&self.config_file).await?;
        let config: Config = toml::from_slice(&bytes)?;
        if let Some(v) = config.modules.get(module_name) {
            return v
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

    async fn list_modules(&self) -> Result<Vec<String>> {
        let bytes = tokio::fs::read(&self.config_file).await?;
        let config: Config = toml::from_slice(&bytes)?;
        config.modules.into_iter().map(|(name, _)| Ok(name)).collect()
    }
}
