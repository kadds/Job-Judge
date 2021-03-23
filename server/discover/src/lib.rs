use async_trait::async_trait;
use std::net::SocketAddr;
use std::{collections::HashMap, io::Error};
use tokio::sync::Mutex;
type Result<T> = std::result::Result<T, Error>;

#[async_trait]
pub trait Discover {
    async fn get_from_module(&self, module_name: &str) -> Result<Vec<(String, SocketAddr)>>;
    async fn get_from_server(
        &self,
        module_name: &str,
        server_name: &str,
    ) -> Result<Option<SocketAddr>>;
}

pub mod config;
pub mod k8s;
pub use config::ConfigDiscover;
pub use k8s::K8sDiscover;

pub struct ModuleDiscover {
    pub discover: Box<dyn Discover + Send + Sync>,
    pub module: String,
    map: Mutex<HashMap<String, SocketAddr>>,
}

pub enum Change {
    Add((String, SocketAddr)),
    Update((String, SocketAddr)),
    Remove((String, SocketAddr)),
}

impl ModuleDiscover {
    pub fn new(discover: Box<dyn Discover + Send + Sync>, module: String) -> Self {
        ModuleDiscover {
            discover,
            map: Mutex::new(HashMap::new()),
            module,
        }
    }
    pub async fn watch(&self) -> Result<Vec<Change>> {
        let res = self.discover.get_from_module(self.module.as_ref()).await?;
        let mut map = HashMap::new();
        let mut old_map = self.map.lock().await;
        let mut ret = vec![];
        for (name, addr) in res {
            if let Some(r) = old_map.get(&name) {
                if *r != addr {
                    ret.push(Change::Update((name.clone(), addr)));
                } else {
                    // eq
                }
            } else {
                ret.push(Change::Add((name.clone(), addr)));
            }
            map.insert(name, addr);
        }
        for (key, _) in old_map.iter() {
            if let Some(addr) = map.get(key) {
                ret.push(Change::Remove((key.clone(), addr.to_owned())));
            }
        }
        *old_map = map;
        Ok(ret)
    }
}
