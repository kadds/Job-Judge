use crate::cfg::*;
use log::*;

use crate::error::Result;
use crate::load_balancer::*;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::{watch, RwLock};

type LoadBalancerHashMap = HashMap<String, Box<dyn LoadBalancer>>;

pub struct MicroService {
    config: MicroServiceConfig,
    // channel for stop signal
    stop_signal: watch::Sender<u64>,
    stop_rx: watch::Receiver<u64>,
    map: RwLock<LoadBalancerHashMap>,
}

impl MicroService {
    pub async fn init(config: MicroServiceConfig) -> Result<Arc<MicroService>> {
        debug!("start micro-service");

        // make struct
        let (stop_signal, stop_signal_rx) = watch::channel(1);

        let res = Arc::new(MicroService {
            config,
            stop_rx: stop_signal_rx.clone(),
            map: RwLock::<LoadBalancerHashMap>::new(LoadBalancerHashMap::new()),
            stop_signal,
        });

        tokio::spawn(res.clone().watch_signal_main(stop_signal_rx));
        return Ok(res);
    }

    async fn module_query(self: Arc<Self>, module: &str) -> std::io::Result<HashSet<String>> {
        let mut map = HashSet::new();
        let fmt = self.config.meta.dns_template.clone();
        let host = fmt.replace("{}", module);
        let res = tokio::net::lookup_host(host);
        for i in res.await? {
            map.insert(i.to_string());
        }

        Ok(map)
    }

    async fn watch_main(self: Arc<Self>, module: String) -> Result<()> {
        let mut stop_rx = self.stop_rx.clone();
        loop {
            let servers = tokio::select! {
                Ok(servers) = self.clone().module_query(&module) => {
                    servers
                }
                Ok(_) = stop_rx.changed() => {
                    break;
                }
            };
            let mut map = self.map.write().await;
            let load_balancer = map.get_mut(&module).unwrap();

            load_balancer.on_update(servers).await;
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
            return load_balancer.fetch_channel(uin, flags).await;
        }
        None
    }

    pub async fn listen_module(
        self: Arc<Self>,
        module: String,
        load_balancer: Box<dyn LoadBalancer>,
    ) -> Option<()> {
        let res = self
            .map
            .write()
            .await
            .insert(module.to_owned(), load_balancer)
            .map(|_| ());
        tokio::spawn(self.watch_main(module));
        res
    }

    pub fn stop_self(&self) {
        debug!("sending stop signal");
        let _ = self.stop_signal.send(0);
    }

    pub fn service_name(&self) -> String {
        self.config.meta.name.clone()
    }

    pub fn service_signal(&self) -> watch::Receiver<u64> {
        self.stop_rx.clone()
    }

    pub fn service_level(&self) -> ServiceLevel {
        self.config.meta.level.clone()
    }

    pub fn service_address(&self) -> String {
        self.config.meta.ip.clone()
    }
    pub fn comm_database_url(&self) -> String {
        self.config.comm_database.url.clone()
    }
}

#[macro_export]
macro_rules! register_module_with_random {
    ($ms:expr, $module: expr) => {
        $ms.listen_module(
            $module.into(),
            Box::new($crate::load_balancer::RandomLoadBalancer::new()),
        )
        .await;
    };
}
