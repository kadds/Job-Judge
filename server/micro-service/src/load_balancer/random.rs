use super::*;
use crate::ServerInfo;
use async_trait::async_trait;
use rand::prelude::*;
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use tonic::transport::Channel;

pub struct RandomLoadBalancer {
    servers: Vec<(String, ServerInfo)>,
    set: HashMap<String, usize>,
    cache: ClientCache,
}

impl RandomLoadBalancer {
    pub fn new() -> RandomLoadBalancer {
        RandomLoadBalancer {
            servers: Vec::<(String, ServerInfo)>::new(),
            set: HashMap::<String, usize>::new(),
            cache: ClientCache::new(),
        }
    }
}

impl Default for RandomLoadBalancer {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LoadBalancer for RandomLoadBalancer {
    async fn on_update(&mut self, servers: HashSet<String>) {
        let mut new_servers = vec![];
        let mut new_set = HashMap::<String, usize>::new();
        for address in &servers {
            if let Some(idx) = self.set.get(address) {
                let mut server_info = self.servers.get(idx.to_owned()).unwrap().1.clone();
                server_info.mtime = Instant::now();
                new_servers.push((address.clone(), server_info));
                new_set.insert(address.clone(), new_servers.len());
            } else {
                info!("new server {} join", address);
                new_servers.push((
                    address.clone(),
                    ServerInfo {
                        enabled: false,
                        ctime: Instant::now(),
                        mtime: Instant::now(),
                    },
                ));
                new_set.insert(address.clone(), new_servers.len());
            }
        }
        for address in self.set.keys() {
            if !servers.contains(address) {
                info!("exist server {} exit", address);
            }
        }
        self.servers = new_servers;
        self.set = new_set;
    }

    async fn on_rpc_update(&mut self, _s: Vec<(String, ServerInfo)>) {}

    async fn fetch_channel(&self, _uin: u64, _flags: u64) -> Option<Channel> {
        let vec = &self.servers;
        match vec.len() {
            0 => None,
            v => {
                let v = &vec[rand::thread_rng().gen_range(0..v)];
                self.cache.get_client(&v.0).await
            }
        }
    }
}
