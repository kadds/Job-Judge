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

    pub async fn client<T: RpcClient<T>>(self: Arc<Self>) -> T {
        let channel = self.channel(T::name()).await;
        T::make(channel)
    }

    async fn watch_signal_main(self: Arc<Self>) {
        let sigint = signal::ctrl_c();
        let mut rx = self.rx.clone();
        let changed = rx.changed();
        let signal_type = if cfg!(unix) {
            let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate()).unwrap();
            let sigterm = sigterm.recv();
            tokio::select! {
                _ = sigint => {"SIGINT"},
                _ = sigterm => {"SIGTERM"},
                Ok(_) = changed => {
                    "SIG_UNKNOWN"
                },
            }
        } else {
            tokio::select! {
                _ = sigint => {"SIGINT"},
                Ok(_) = changed => {
                    "SIG_UNKNOWN"
                },
            }
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
    pub fn config(&self) -> Arc<MicroServiceConfig> {
        self.config.clone()
    }
}

pub trait RpcClient<T> {
    fn name() -> &'static str;
    fn make(ch: Channel) -> T;
}

#[macro_export]
macro_rules! define_client {
    ($type: ident, $client: ident, $name: tt) => {
        pub type $client = $type<tonic::transport::Channel>;
        impl micro_service::server::RpcClient<$client> for $client {
            fn name() -> &'static str {
                $name
            }
            fn make(ch: tonic::transport::Channel) -> $client {
                $type::new(ch)
            }
        }
    };
}
