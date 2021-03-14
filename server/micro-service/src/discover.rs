use crate::service::Module;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    io::{Error, ErrorKind, Result},
    net::SocketAddr,
};
use tokio::{fs::read, net::lookup_host};
use tower::discover::Change;

#[derive(Serialize, Deserialize)]
struct ModuleConfig {
    pub services: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct DiscoverConfig {
    pub modules: HashMap<String, ModuleConfig>,
}
impl Module {
    pub(crate) async fn discover_from_dns(&self) -> Result<Vec<Change<SocketAddr, ()>>> {
        let mut changes = Vec::<Change<SocketAddr, ()>>::new();
        let mut set = HashSet::new();
        for address in lookup_host(&self.dns_url).await? {
            set.insert(address);
            if self.services.lock().await.get(&address).is_none() {
                changes.push(Change::Insert(address, ()));
            }
        }
        for address in self.services.lock().await.keys() {
            if !set.contains(address) {
                changes.push(Change::Remove(address.to_owned()))
            }
        }
        Ok(changes)
    }
    pub(crate) async fn discover_from_file(
        &self,
        file: &str,
    ) -> Result<Vec<Change<SocketAddr, ()>>> {
        let mut changes = Vec::<Change<SocketAddr, ()>>::new();
        let mut set = HashSet::new();
        let config_json = read(file).await?;
        let config: DiscoverConfig = serde_json::from_slice(&config_json)?;
        let module = match config.modules.get(&self.module) {
            Some(v) => v,
            None => {
                return Ok(changes);
            }
        };

        for address in module.services.iter() {
            let address = address
                .parse()
                .map_err(|e| Error::new(ErrorKind::InvalidData, e))?;
            set.insert(address);
            if self.services.lock().await.get(&address).is_none() {
                changes.push(Change::Insert(address, ()));
            }
        }
        for address in self.services.lock().await.keys() {
            if !set.contains(address) {
                changes.push(Change::Remove(address.to_owned()))
            }
        }
        Ok(changes)
    }
}
