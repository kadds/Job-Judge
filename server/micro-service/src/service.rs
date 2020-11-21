use super::cfg::EtcdConfig;
use super::error::{Error, Result};
use super::load_balancer::*;
use super::log;
use super::ServerInfo;
use std::time::Duration;
use tokio::stream::StreamExt;
use tokio::time::delay_for;
use tonic;
use tokio::signal::unix::{signal, SignalKind};

use etcd_rs::{
    Client, ClientConfig, EventType, KeyRange, LeaseGrantRequest,
    LeaseKeepAliveRequest, PutRequest, DeleteRequest
};
use std::collections::{HashMap};
use std::sync::Arc;
use tokio::sync::{watch, RwLock};

type LoadBalancerHashMap = HashMap<String, Box<dyn LoadBalancer>>;

pub struct MicroService {
    etcd_config: EtcdConfig,
    etcd: Client,
    // module name
    module: String,
    // micro service name
    name: Arc<String>,
    lease_id: u64,
    // channel for stop signal
    stop_signal: watch::Sender<u64>,
    map: RwLock<LoadBalancerHashMap>,
}

impl MicroService{
    pub async fn init(
        etcd_config: EtcdConfig,
        module: String,
        name: String,
        address: String,
        retry_times: u32,
    )-> Result<Arc<MicroService>> {
        super::error::panic_hook();
        assert!(etcd_config.ttl >= 30);
        early_log!("debug", name, "connecting micro-service center address {:?} at prefix {}", etcd_config.endpoints, etcd_config.prefix);
        let ttl = Duration::from_secs(etcd_config.ttl.into());
        for i in 0..retry_times {
            let client = Client::connect(ClientConfig {
                endpoints: etcd_config.endpoints.to_owned(),
                auth: Some((etcd_config.username.to_owned(), etcd_config.password.to_owned())),
                tls: None,
            })
            .await;
            match client {
                Ok(client) => {
                    // new lease id for path
                    let lease_id = match client.lease().grant(LeaseGrantRequest::new(ttl)).await {
                        Ok(v) => {
                            if v.id() == 0 {
                                early_log_error!(name, "request lease fail, lease id returns 0");
                                return Err(Error::ResourceLimit);
                            }
                            early_log_info!(name, "request lease id is {}", v.id());
                            v.id()
                        }
                        Err(e) => {
                            early_log_error!(name, "request lease fail result {:?}", e);
                            return Err(Error::ResourceLimit);
                        }
                    };
                    let (stop_signal, stop_signal_rx) = watch::channel(1);

                    // make struct
                    let res = Arc::new(MicroService {
                        etcd: client,
                        etcd_config,
                        name: Arc::new(name),
                        module,
                        lease_id,
                        map: RwLock::<LoadBalancerHashMap>::new(LoadBalancerHashMap::new()),
                        stop_signal,
                    });

                    // write server config
                    let mut req = PutRequest::new(
                        format!(
                            "{}/{}/{}/info",
                            res.etcd_config.prefix, res.module, res.name
                        ),
                        ServerInfo {
                            address: address,
                        }
                        .to_json(),
                    );
                    // put server address to etcd
                    req.set_lease(res.lease_id);
                    match res.etcd.kv().put(req).await {
                        Ok(v) => v,
                        Err(e) => {
                            let name = res.name.clone();
                            early_log_error!(name, "write server info (etcd) fail result {:?}", e);
                            return Err(Error::Unknown);
                        }
                    };
                    tokio::spawn(log::make_context(0, 0, 0, 0, res.name.clone(), res.clone().ttl_main(ttl, stop_signal_rx.clone())));
                    tokio::spawn(log::make_context(0, 0, 0, 0, res.name.clone(), res.clone().watch_signal_stop_main(stop_signal_rx.clone())));
                    return Ok(res);
                }
                Err(e) => {
                    early_log_warn!(name, "etcd connect fail result {:?} times {}", e, i);
                }
            }
        }

        early_log_error!(name, "etcd connect failed. try {} times", retry_times);
        Err(Error::ConnectionFailed)
    }

    async fn ttl_main(self: Arc<Self>, ttl: Duration, stop_rx: watch::Receiver<u64>) {
        let mut stop_rx = stop_rx;
        let ttl = ttl - Duration::from_secs(10);
        let _ = stop_rx.recv().await;
        loop {
            let res = self
                .etcd
                .lease()
                .keep_alive(LeaseKeepAliveRequest::new(self.lease_id))
                .await;
            let target = match res {
                Ok(_) => delay_for(ttl),
                Err(err) => { warn!("send ttl message fail {:?}", err); delay_for(Duration::from_secs(1))}
            };
            tokio::select! {
                _ = target => {},
                Some(_) = stop_rx.recv() => {
                    break;
                }
            }
        }
        let _ = self.etcd.watch_client().shutdown().await;
        // close lease
        let _ = self.etcd.lease().shutdown().await;
        // remove key on etcd 
        let _ = self.etcd.kv().delete(DeleteRequest::new(
            KeyRange::prefix(format!("{}/{}/{}", self.etcd_config.prefix, self.module, self.name)))
        ).await;
        info!("ttl main is stopped. lease removed");
    }

    pub async fn listen_module(
        self: Arc<Self>,
        module: String,
        load_balancer: Box<dyn LoadBalancer>,
    ) -> Option<()> {
        let res = self.map
            .write()
            .await
            .insert(module.to_owned(), load_balancer)
            .map(|_| ());
        tokio::spawn(log::make_context(0, 0, 0, 0, self.name.clone(), self.watch_main(module)));
        res
    }

    async fn watch_main(self: Arc<Self>, module: String) -> Result<()> {
        let mut wc = self.etcd.watch_client();
        let mut inbound = wc
            .watch(KeyRange::prefix(self.etcd_config.prefix.clone() + &module))
            .await;

        while let Some(r) = inbound.next().await {
            let mut vec = match r {
                Ok(v) => v,
                Err(err) => {
                    error!("watch server fail {}", err);
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
                load_balancer
                    .on_update(put_vec, ServerChangeType::Add)
                    .await;
            }
            if del_vec.len() > 0 {
                load_balancer
                    .on_update(del_vec, ServerChangeType::Remove)
                    .await;
            }
        }

        Ok(())
    }

    pub async fn get_channel(
        &self,
        module: &str,
        uin: u64,
        flags: u64,
    ) -> Option<tonic::transport::Channel> {
        if let Some(load_balancer) = self.map.read().await.get(module) {
            return load_balancer.get_server(uin, flags).await;
        }
        None
    }

    pub fn stop(&self) {
        debug!("stop signal is sent");
        let _ = self.stop_signal.broadcast(0);
    }

    async fn watch_signal_stop_main(self: Arc<Self>, stop_rx: watch::Receiver<u64>) {
        let mut stop_rx = stop_rx;
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        let mut sigquit = signal(SignalKind::quit()).unwrap();
        let mut sigter = signal(SignalKind::terminate()).unwrap();
        let _ = stop_rx.recv().await;
        tokio::select! {
            _ = sigint.recv() => {},
            _= sigquit.recv() => {},
            _= sigter.recv() => {},
            Some(_) = stop_rx.recv() => {
            },
        };
        info!("recv stop signal");
        let _ = self.stop_signal.broadcast(0);
        // TODO: async callback
    }
    pub fn get_server_name(&self) -> Arc<String> {
        self.name.clone()
    }

}

#[macro_export]
macro_rules! register_module_with_random {
    ($ms:expr, $module: expr) => {
        $ms.listen_module(
            $module.into(),
            Box::new($crate::load_balancer::RandomLoadBalancer::new()),
        ).await;
    };
}