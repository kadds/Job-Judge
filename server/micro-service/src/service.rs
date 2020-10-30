use super::error::{Error, Result};
use super::load_balancer::{LoadBalancer, ServerChangeType};
use super::log;
use tokio::stream::StreamExt;
use crate::server_info::ServerInfo;

use etcd_rs::{
    Client, ClientConfig, EventType, KeyRange, Kv, Lease, LeaseGrantRequest, LeaseGrantResponse,
    LeaseKeepAliveRequest, LeaseKeepAliveResponse, PutRequest, PutResponse,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

type LoadBalancerHashMap = HashMap<String, RwLock<Arc<dyn LoadBalancer + Send + Sync>>>;

async fn do_keep_alive(lease: &mut Lease, lease_id: u64) -> Result<()> {
    let _rsp = lease.keep_alive(LeaseKeepAliveRequest::new(lease_id)).await;
    Ok(())
}

async fn do_update(kv: &mut Kv, prefix: &str, module: &str, name: &str, info: ServerInfo) -> Result<()> {
    kv.put(PutRequest::new(
        format!("{}/{}/{}/info", prefix, module, name),
        info.to_json(),
    ))
    .await
    .and(Ok(()))
    .or(Err(Error::ConnectionFailed))
}

pub struct EtcdConfig {
    endpoints: Vec<String>,
    user: String,
    password: String,
    prefix: String,
}

pub struct MicroService {
    pub etcd_config: EtcdConfig,
    pub etcd: Client,
    pub module: String,
    pub name: String,
    pub lease_id: u64,
    pub map: RwLock<LoadBalancerHashMap>,
}


impl MicroService {
    pub async fn init(etcd_config: EtcdConfig, module: String, name: String, retry_times: u32) -> Result<MicroService> {
        for _ in 0..retry_times {
            let client = Client::connect(ClientConfig {
                endpoints: etcd_config.endpoints.to_owned(),
                auth: Some((etcd_config.user.to_owned(), etcd_config.password.to_owned())),
                tls: None,
            })
            .await;
            if let Ok(v) = client {
                return Ok(MicroService {
                    etcd: v,
                    etcd_config,
                    name,
                    module,
                    lease_id: 0,
                    map: RwLock::<LoadBalancerHashMap>::new(LoadBalancerHashMap::new()),
                });
            }
        }
        error!("etcd connect failed. try {} times", retry_times);
        Err(Error::ConnectionFailed)
    }

    async fn register_self(&mut self, ip: u32, port: u16, ttl: u64, retry_times: u32) -> Result<()> {
        let mut lease_id: u64 = 0;
        for _ in 0..retry_times {
            if let Ok(v) = self
                .etcd
                .lease()
                .grant(LeaseGrantRequest::new(std::time::Duration::from_millis(
                    ttl,
                )))
                .await
            {
                lease_id = v.id();
                break;
            }

            warn!("micro-service load lease failed, retry...");
        }

        if lease_id == 0 {
            error!("request lease failed. try {} times", retry_times);
            return Err(Error::ResourceLimit);
        }

        for _ in 0..retry_times {
            let mut req = PutRequest::new(
                format!("{}/{}/{}/info", self.etcd_config.prefix, self.module, self.name),
                ServerInfo{
                    fail_percent: 0,
                    workload: 0,
                    ip, 
                    port
                }.to_json(),
            );
            req.set_lease(lease_id);
            if self.etcd.kv().put(req).await.is_ok() {
                self.lease_id = lease_id;
                return Ok(());
            }
            warn!("micro-service register failed, retry...");
        }
        error!(
            "micro-service '{}:{}' with address {} register failed. try {} times",
            self.name, ip, port, retry_times
        );

        Err(Error::ResourceLimit)
    }

    async fn update_self(&mut self, info: ServerInfo) -> Result<()> {
        let mut lease = self.etcd.lease();
        let mut kv = self.etcd.kv();
        let res = tokio::try_join!(
            do_keep_alive(&mut lease, self.lease_id),
            do_update(&mut kv, &self.etcd_config.prefix, &self.module, &self.name, info)
        );

        match res {
            Ok(_) => Ok(()),
            Err(err) => {
                error!("micro-service heartbeat failed");
                Err(Error::ConnectionFailed)
            }
        }
    }

    async fn watch_module(
        &mut self,
        module: String,
        load_balancer: RwLock<Arc<dyn LoadBalancer + Send + Sync>>,
    ) -> Result<()> {
        self.map
            .write().await
            .insert(module.to_owned(), load_balancer);

        let mut wc = self.etcd.watch_client();
        let mut inbound = wc.watch(KeyRange::prefix(self.etcd_config.prefix + &module)).await;

        tokio::spawn(async move {
            while let Some(r) = inbound.next().await {
                let mut vec = match r {
                    Ok(v) => v,
                    Err(err) => {
                        error!("watch server failed");
                        return ();
                    }
                };
                let events = vec.take_events();
                let mut put_vec: Vec<(String, String)> = Vec::new();
                let mut del_vec: Vec<(String, String)> = Vec::new();

                for mut it in events {
                    let kv = match it.take_kvs() {
                        Some(v) => v,
                        None => {
                            continue;
                        }
                    };
                    match it.event_type() {
                        EventType::Put => {
                            put_vec.push((kv.key_str().to_string(), kv.value_str().to_string()));
                        }
                        EventType::Delete => {
                            del_vec.push((kv.key_str().to_owned(), kv.value_str().to_string()));
                        }
                    }
                }
                let load_balancer = self.map.read().await.get(&module).unwrap();

                if put_vec.len() > 0 {
                    load_balancer
                        .write().await
                        .on_server_change(put_vec, ServerChangeType::Add);
                }
                if del_vec.len() > 0 {
                    load_balancer
                        .write().await
                        .on_server_change(del_vec, ServerChangeType::Remove);
                }
            }
        });

        Ok(())
    }

    async fn get_load_balance_server(&self, module: String, uin: u64, flags: u64) -> Option<String> {
        if let Some(load_balancer) = self.map.read().await.get(&module) {
            return load_balancer
                .read().await
                .get_server(uin, flags);
        }
        None
    }
}
