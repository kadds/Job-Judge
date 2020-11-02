use crate::ServerInfo;
use std::collections::HashMap;
use tonic::transport::{Channel, Endpoint};

pub enum ServerChangeType {
    Add,
    Remove,
}

pub trait LoadBalancer: Sync + Send {
    fn get_server(&self, uin: u64, flags: u64) -> Option<Channel>;
    fn on_update(&mut self, s: Vec<(String, ServerInfo)>, change_type: ServerChangeType);
    fn on_rpc_update(&mut self, s: Vec<(String, ServerInfo)>);
}

struct ClientCache {
    map: HashMap<String, Channel>,
}

impl ClientCache {
    fn new() -> ClientCache {
        ClientCache {
            map: HashMap::<String, Channel>::new(),
        }
    }
    async fn get_client(&mut self, address: &str) -> Option<Channel> {
        if !self.map.contains_key(address) {
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
            self.map.insert(address.to_string(), channel);
        }
        self.map.get(address).map(|v| v.clone())
    }
}
