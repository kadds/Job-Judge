use rand::prelude::*;
use crate::server_info::ServerInfo;
use crate::log;
use tonic::transport::{Channel, Endpoint};

pub struct RandomLoadBalancer {
    servers: Vec<ServerInfo>,
    set: HashMap<String, usize>,
    cache: ClientCache,
}

impl LoadBalancer for RandomLoadBalancer {
    fn new() -> RandomLoadBalancer{
        RandomLoadBalancer {
            servers: Vec!(),
            set: HashMap<String, usize>::new(),
            cache: ClientCache::new(),
        }
    }
    fn on_update(&mut self, s: Vec<(String, ServerInfo)>, change_type: ServerChangeType) {
        for (name, si) in s {
            let idx = self.set.get(&name);
            match change_type {
                ServerChangeType::Add => match idx {
                    Some(i) => {
                        // address update
                        self.servers[i.to_owned()] = si;
                    }
                    None => {
                        self.servers.push(si);
                        self.set.insert(name.to_owned(), self.servers.len());
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

    fn get_server(&self, uin: u64, flags: u64) -> Option<String> {
        let vec = &self.servers;
        match vec.len() {
            0 => None,
            v => {let v = &vec[rand::thread_rng().gen_range(0, v)]; self.cache.get_client(v.address.clone())}
        }
    }
}