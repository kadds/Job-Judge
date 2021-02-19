use super::cfg::EtcdConfig;
use super::error::{Error, Result};
use super::load_balancer::*;
use super::log;
use super::ServerInfo;
use std::time::{Duration, SystemTime};
use tonic;
use tokio::{
    signal::unix::{signal, SignalKind},
    time::sleep,
};
use tokio_stream::StreamExt;

use etcd_rs::{
    Client, ClientConfig, EventType, KeyRange, LeaseGrantRequest,
    LeaseKeepAliveRequest, PutRequest, DeleteRequest
};
use std::collections::{HashMap};
use std::sync::Arc;
use tokio::sync::{watch, RwLock};

type LoadBalancerHashMap = HashMap<String, Box<dyn LoadBalancer>>;

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceLevel  {
    Test,
    Pre,
    Prod,
}

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
    stop_rx: watch::Receiver<u64>,
    map: RwLock<LoadBalancerHashMap>,
    level: ServiceLevel,
    address: String,
}

async fn retry_connect_client(etcd_config: &EtcdConfig, retry_times: u32, name: &str) -> Result<Client> {
    for i in 0..retry_times {
        let client = Client::connect(ClientConfig {
            endpoints: etcd_config.endpoints.to_owned(),
            auth: Some((etcd_config.username.to_owned(), etcd_config.password.to_owned())),
            tls: None,
        })
        .await;
        match client {
            Ok(client) => {
                return Ok(client);
            },
            Err(e) => {
                early_log_warn!(name, "etcd connect fail result {:?} times {}", e, i);
                sleep(Duration::from_secs(1)).await;
            }
        };
    }
    early_log_error!(name, "etcd connect failed. try {} times", retry_times);
    Err(Error::ConnectionFailed)
}

async fn retry_lease_id(client: &Client, retry_times: u32, name: &str, ttl: Duration) -> Result<u64> {
    let mut err  = None;
    for _ in 0..retry_times {
        // new lease id for path
        match client.lease().grant(LeaseGrantRequest::new(ttl)).await {
            Ok(v) => {
                if v.id() == 0 {
                    early_log_error!(name, "request lease fail, lease id returns 0");
                    sleep(Duration::from_secs(1)).await;
                }
                else {
                    early_log_info!(name, "request lease id is {}", v.id());
                    return Ok(v.id())
                }
            }
            Err(e) => {
                early_log_error!(name, "request lease fail result {:?}", e);
                err = Some(e);
                sleep(Duration::from_secs(1)).await;
            }
        };
    }
    if let Some(err) = err {
        Err(Error::OperationError(err))
    }
    else {
        Err(Error::ResourceLimit)
    }
}

async fn retry_write_with_lease(client: &Client, retry_times: u32, name: &str, lease_id: u64, key: String, data: String) 
    -> Result<()> {
    let mut err  = None;
    for _ in 0..retry_times {
        let mut req = PutRequest::new(key.clone(), data.clone()); 
        req.set_lease(lease_id);
        match client.kv().put(req).await {
            Ok(_) => {
                return Ok(());
            },
            Err(e) => {
                early_log_error!(name, "write server info (etcd) fail result {:?}", e);
                err = Some(e);
                sleep(Duration::from_secs(1)).await;
            }
        };
    }
    let _ = client.lease().shutdown().await;
    early_log_error!(name, "write server info (etcd) fail");
    Err(Error::OperationError(err.unwrap()))
}

impl MicroService{
    pub async fn init(
        etcd_config: EtcdConfig,
        module: String,
        name: String,
        address: String,
        retry_times: u32,
        level: ServiceLevel,
    )-> Result<Arc<MicroService>> {
        crate::error::panic_hook();
        assert!(etcd_config.ttl >= 30);
        early_log_debug!(name, "connecting micro-service center address {:?} at prefix {}", etcd_config.endpoints, etcd_config.prefix);

        // connect to etcd
        let client = retry_connect_client(&etcd_config, retry_times, &name).await?;

        // new lease id
        let ttl = Duration::from_secs(etcd_config.ttl.into());
        let lease_id = retry_lease_id(&client, retry_times, &name, ttl).await?;

        // write server config
        let ctime = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |v| v.as_millis() as i64);
        let key = format!(
            "{}/{}/{}/info",
            etcd_config.prefix, module, name
        );
        let value = ServerInfo {
            address: address.clone(),
            enabled: false,
            ctime: ctime,
            mtime: ctime
        }.to_json();
        retry_write_with_lease(&client, retry_times, &name, lease_id, key.clone(), value).await?;

        let value = ServerInfo {
            address: address.clone(),
            enabled: true,
            ctime,
            mtime: ctime
        }.to_json();
        retry_write_with_lease(&client, retry_times, &name, lease_id, key, value).await?;

        // make struct
        let (stop_signal, mut stop_signal_rx) = watch::channel(1);
        let _ = stop_signal_rx.changed().await;

        let res = Arc::new(MicroService {
            etcd: client,
            etcd_config,
            name: Arc::new(name),
            module,
            address,
            stop_rx: stop_signal_rx.clone(),
            lease_id,
            map: RwLock::<LoadBalancerHashMap>::new(LoadBalancerHashMap::new()),
            stop_signal,
            level,
        });

        tokio::spawn(log::make_empty_context(res.name.clone(), res.clone().ttl_main(ttl, stop_signal_rx.clone())));
        tokio::spawn(log::make_empty_context(res.name.clone(), res.clone().watch_signal_main(stop_signal_rx.clone())));
        return Ok(res);
    }

    async fn ttl_main(self: Arc<Self>, ttl: Duration, mut stop_rx: watch::Receiver<u64>) {
        let ttl = ttl - Duration::from_secs(10);
        loop {
            let res = self
                .etcd
                .lease()
                .keep_alive(LeaseKeepAliveRequest::new(self.lease_id))
                .await;
            let target = match res {
                Ok(_) => sleep(ttl),
                Err(err) => { warn!("send ttl message fail {:?}", err); sleep(Duration::from_secs(1))}
            };
            tokio::select! {
                _ = target => {},
                Ok(_) = stop_rx.changed() => {
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
        debug!("ttl main is stopped. lease removed");
    }

    async fn watch_main(self: Arc<Self>, module: String) -> Result<()> {
        let mut wc = self.etcd.watch_client();
        wc.watch(KeyRange::prefix(self.etcd_config.prefix.clone() + &module))
        .await?;

        let mut watch_recver = wc.take_receiver().await;

        let mut stop_rx = self.stop_rx.clone();

        loop {
            let r = tokio::select! {
                Some(r) = watch_recver.next() => {
                    r
                }
                Ok(_) = stop_rx.changed() => {
                    break;
                }
            };
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

        debug!("watch ({}) main is stopped", module);
        Ok(())
    }

    async fn watch_signal_main(self: Arc<Self>, mut stop_rx: watch::Receiver<u64>) {
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        let mut sigquit = signal(SignalKind::quit()).unwrap();
        let mut sigter = signal(SignalKind::terminate()).unwrap();
        let signal_type = tokio::select! {
            _ = sigint.recv() => {"SIGINT"},
            _= sigquit.recv() => {"SIGQUIT"},
            _= sigter.recv() => {"SIGTER"},
            Ok(_) = stop_rx.changed() => {
                "SIG_UNKNOWN"
            },
        };
        info!("signal {} received. Stopping server", signal_type);
        let _ = self.stop_signal.send(0);
    }

    pub async fn channel(
        &self,
        module: &str,
        uin: u64,
        flags: u64,
    ) -> Option<tonic::transport::Channel> {
        if let Some(load_balancer) = self.map.read().await.get(module) {
            return load_balancer.one_of_channel(uin, flags).await;
        }
        None
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

    pub fn stop_self(&self) {
        debug!("sending stop signal");
        let _ = self.stop_signal.send(0);
    }

    pub fn service_name(&self) -> Arc<String> {
        self.name.clone()
    }

    pub fn service_signal(&self) -> watch::Receiver<u64> {
        self.stop_rx.clone()
    }

    pub fn service_level(&self) -> ServiceLevel {
        self.level.clone()
    }

    pub fn service_address(&self) -> String{
        self.address.clone()
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