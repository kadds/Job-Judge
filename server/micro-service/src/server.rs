use crate::cfg::ServiceLevel;
use crate::{cfg::MicroServiceConfig, service::Module};
use log::*;
use std::{collections::HashMap, sync::Arc};
use tokio::{
    signal,
    sync::{watch, Mutex},
};
use tonic::transport::Channel;

pub struct Server {
    config: Arc<MicroServiceConfig>,
    modules: Mutex<HashMap<String, Arc<Module>>>,
    // channel for stop signal
    tx: watch::Sender<()>,
    rx: watch::Receiver<()>,
}

impl Server {
    pub async fn new(config: Arc<MicroServiceConfig>) -> Arc<Server> {
        debug!("start micro-service");

        // make struct
        let (tx, rx) = watch::channel(());

        let res = Arc::new(Server {
            config,
            tx,
            rx,
            modules: Mutex::new(HashMap::new()),
        });

        tokio::spawn(res.clone().watch_signal_main());
        res
    }

    pub async fn channel(self: Arc<Self>, module: &str) -> Channel {
        let mut map = self.modules.lock().await;
        match map.get(module) {
            Some(v) => v.channel(),
            None => {
                let m = Module::new(module.to_owned(), self.config.clone(), self.rx.clone());
                let channel = m.channel();
                map.insert(module.to_owned(), m);
                channel
            }
        }
    }

    async fn watch_signal_main(self: Arc<Self>) {
        let sigint = signal::ctrl_c();
        let mut rx = self.rx.clone();
        let changed = rx.changed();
        let signal_type = tokio::select! {
            _ = sigint => {"SIGINT"},
            Ok(_) = changed => {
                "SIG_UNKNOWN"
            },
        };
        info!("signal {} received. Stopping server", signal_type);
        let _ = self.tx.send(());
    }

    pub fn stop_self(&self) {
        debug!("sending stop signal");
        let _ = self.tx.send(());
    }

    pub fn server_name(&self) -> String {
        self.config.meta.name.clone()
    }
    pub fn server_signal(&self) -> watch::Receiver<()> {
        self.rx.clone()
    }
    pub fn service_level(&self) -> ServiceLevel {
        self.config.meta.level.clone()
    }
    pub fn server_address(&self) -> String {
        self.config.meta.ip.clone()
    }
    pub fn comm_database_url(&self) -> String {
        self.config.comm_database.url.clone()
    }
    pub fn config(&self) -> Arc<MicroServiceConfig> {
        self.config.clone()
    }
}
