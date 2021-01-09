use super::*;
use crate::log;
use crate::ServerInfo;
use async_trait::async_trait;
use rand::prelude::*;
use std::collections::HashMap;
use tonic::transport::{Channel};

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

#[async_trait]
impl LoadBalancer for RandomLoadBalancer {
    async fn on_update(&mut self, s: Vec<(String, ServerInfo)>, change_type: ServerChangeType) {
        for (name, si) in s {
            let idx = self.set.get(&name);
            match change_type {
                ServerChangeType::Add => match idx {
                    Some(i) => {
                        // address update
                        self.servers[i.to_owned()] = (name, si);
                    }
                    None => {
                        self.servers.push((name.clone(), si));
                        self.set.insert(name, self.servers.len());
                    }
                },
                ServerChangeType::Remove => match idx {
                    Some(i) => {
                        let vec_idx = i.to_owned();
                        self.set.remove(&name);
                        *self.set.get_mut(&self.servers.last().unwrap().0).unwrap() = vec_idx;
                        self.servers.swap_remove(vec_idx);
                    }
                    None => {
                        warn!("can't find available server in map");
                    }
                },
            }
        }
    }

    async fn on_rpc_update(&mut self, _s: Vec<(String, ServerInfo)>) {}

    async fn one_of_channel(&self, _uin: u64, _flags: u64) -> Option<Channel> {
        let vec = &self.servers;
        match vec.len() {
            0 => None,
            v => {
                let v = &vec[rand::thread_rng().gen_range(0, v)];
                self.cache.get_client(&v.1.address).await
            }
        }
    }
}
