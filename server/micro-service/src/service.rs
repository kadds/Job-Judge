use super::cfg::EtcdConfig;
use super::error::{Error, Result};
use super::load_balancer::{LoadBalancer, ServerChangeType};
use super::log;
use super::ServerInfo;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::stream::StreamExt;
use tokio::time::delay_for;
use tonic;
use tonic::transport::{Channel, Endpoint};

use etcd_rs::{
    Client, ClientConfig, EventType, KeyRange, Kv, Lease, LeaseGrantRequest, LeaseGrantResponse,
    LeaseKeepAliveRequest, LeaseKeepAliveResponse, PutRequest, PutResponse,
};
use std::collections::{HashMap, HashSet};
use std::sync::{atomic::AtomicUsize, Arc};
use tokio::sync::{watch, RwLock};

type LoadBalancerHashMap = HashMap<String, Box<dyn LoadBalancer>>;

pub struct MicroService {
    etcd_config: EtcdConfig,
    etcd: Client,
    // module name
    module: String,
    // micro service name
    name: String,
    lease_id: u64,
    // channel for stop signal
    stop_signal: watch::Sender<u64>,
    stop_signal_rx: watch::Receiver<u64>,

    map: RwLock<LoadBalancerHashMap>,
}

impl MicroService {
    pub async fn init(
        etcd_config: EtcdConfig,
        module: String,
        name: String,
        address: SocketAddr,
        ttl: Duration,
        retry_times: u32,
    ) -> Result<Arc<MicroService>> {
        assert!(ttl >= Duration::from_secs(60));
        for i in 0..retry_times {
            let client = Client::connect(ClientConfig {
                endpoints: etcd_config.endpoints.to_owned(),
                auth: Some((etcd_config.user.to_owned(), etcd_config.password.to_owned())),
                tls: None,
            })
            .await;
            match client {
                Ok(client) => {
                    // new lease id for path
                    let lease_id = match client.lease().grant(LeaseGrantRequest::new(ttl)).await {
                        Ok(v) => {
                            if v.id() == 0 {
                                error!("request lease fail, lease id returns 0");
                                return Err(Error::ResourceLimit);
                            }
                            info!("request lease id is {}", v.id());
                            v.id()
                        }
                        Err(e) => {
                            error!("request lease fail result {}", e);
                            return Err(Error::ResourceLimit);
                        }
                    };
                    let (stop_signal, stop_signal_rx) = watch::channel(1);

                    // make struct
                    let res = Arc::new(MicroService {
                        etcd: client,
                        etcd_config,
                        name,
                        module,
                        lease_id,
                        map: RwLock::<LoadBalancerHashMap>::new(LoadBalancerHashMap::new()),
                        stop_signal,
                        stop_signal_rx,
                    });

                    // write server config
                    let mut req = PutRequest::new(
                        format!(
                            "{}/{}/{}/info",
                            res.etcd_config.prefix, res.module, res.name
                        ),
                        ServerInfo {
                            address: address.to_string(),
                        }
                        .to_json(),
                    );
                    req.set_lease(res.lease_id);
                    match res.etcd.kv().put(req).await {
                        Ok(v) => v,
                        Err(e) => {
                            error!("write server info (etcd) fail result {}", e);
                            return Err(Error::Unknown);
                        }
                    };
                    tokio::spawn(res.clone().ttl_main(ttl));
                    return Ok(res);
                }
                Err(e) => {
                    warn!("etcd connect fail result {} at {}", e, i);
                }
            }
        }

        error!("etcd connect failed. try {} times", retry_times);
        Err(Error::ConnectionFailed)
    }

    async fn ttl_main(self: Arc<MicroService>, ttl: Duration) {
        let mut stop_rx = self.stop_signal_rx.clone();
        let mut is_running = true;
        let ttl = ttl - Duration::from_secs(10);
        while is_running {
            let res = self
                .etcd
                .lease()
                .keep_alive(LeaseKeepAliveRequest::new(self.lease_id))
                .await;
            tokio::select! {
                _  = match res {
                    Ok(_) => delay_for(ttl),
                    Err(err) => delay_for(Duration::from_secs(1)),
                } => {},
                Some(_) = stop_rx.recv() =>{
                    is_running = false;
                    ()
                }
            }
        }
        // close lease
        let _ = self.etcd.lease().shutdown().await;
    }

    async fn listen_module(
        &mut self,
        module: String,
        load_balancer: Box<dyn LoadBalancer>,
    ) -> Option<()> {
        self.map
            .write()
            .await
            .insert(module.to_owned(), load_balancer)
            .map(|_| ())
    }

    async fn watch_main(self: Arc<MicroService>, module: String) -> Result<()> {
        let mut wc = self.etcd.watch_client();
        let mut inbound = wc
            .watch(KeyRange::prefix(self.etcd_config.prefix.clone() + &module))
            .await;

        while let Some(r) = inbound.next().await {
            let mut vec = match r {
                Ok(v) => v,
                Err(err) => {
                    error!("watch server failed");
                    return Err(Error::ResourceLimit);
                }
            };
            let events = vec.take_events();
            let mut put_vec: Vec<(String, ServerInfo)> = Vec::new();
            let mut del_vec: Vec<(String, ServerInfo)> = Vec::new();

            for mut it in events {
                let kv = match it.take_kvs() {
                    Some(v) => v,
                    None => {
                        continue;
                    }
                };
                match it.event_type() {
                    EventType::Put => {
                        put_vec.push((
                            kv.key_str().to_string(),
                            ServerInfo::from_json(kv.value_str()),
                        ));
                    }
                    EventType::Delete => {
                        del_vec.push((
                            kv.key_str().to_owned(),
                            ServerInfo::from_json(kv.value_str()),
                        ));
                    }
                }
            }
            let mut map = self.map.write().await;
            let load_balancer = map.get_mut(&module).unwrap();

            if put_vec.len() > 0 {
                load_balancer.on_update(put_vec, ServerChangeType::Add);
            }
            if del_vec.len() > 0 {
                load_balancer.on_update(del_vec, ServerChangeType::Remove);
            }
        }

        Ok(())
    }

    async fn get_remote_channel(
        &self,
        module: String,
        uin: u64,
        flags: u64,
    ) -> Option<tonic::transport::Channel> {
        if let Some(load_balancer) = self.map.read().await.get(&module) {
            return load_balancer.get_server(uin, flags);
        }
        None
    }

    async fn stop(&self) {
        self.stop_signal.broadcast(0);
    }
}
