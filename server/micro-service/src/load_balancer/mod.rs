use crate::ServerInfo;
use async_trait::async_trait;
use log::*;
use std::collections::HashMap;
use tokio::sync::Mutex;
use tonic::transport::{Channel, Endpoint};
pub mod random;
pub use random::RandomLoadBalancer;
use std::collections::HashSet;

#[async_trait]
pub trait LoadBalancer: Sync + Send {
    async fn fetch_channel(&self, uin: u64, flags: u64) -> Option<Channel>;
    async fn on_update(&mut self, load_balancer: HashSet<String>);
    async fn on_rpc_update(&mut self, s: Vec<(String, ServerInfo)>);
}

pub struct ClientCache {
    map: Mutex<HashMap<String, Channel>>,
}

impl ClientCache {
    fn new() -> ClientCache {
        ClientCache {
            map: Mutex::new(HashMap::<String, Channel>::new()),
        }
    }
    async fn get_client(&self, address: &str) -> Option<Channel> {
        if !self.map.lock().await.contains_key(address) {
            // FIXME: connect once
            let channel = match Endpoint::from_shared(format!("http://{}", address))
                .unwrap()
                .connect()
                .await
            {
                Ok(v) => v,
                Err(err) => {
                    error!("{}", err);
                    return None;
                }
            };
            self.map.lock().await.insert(address.to_string(), channel);
        }
        self.map.lock().await.get(address).map(|v| v.clone())
    }
}
