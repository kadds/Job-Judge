use crate::cfg::*;
use crate::discover::*;
use log::*;
use rand::{Rng, SeedableRng};
use std::{cmp::max, sync::Arc};
use std::{net::SocketAddr, time::Duration};
use tokio::sync::mpsc;
use tokio::sync::watch;
use tonic::transport::{Channel, Endpoint};

#[derive(Debug)]
pub struct Module {
    pub(crate) channel: Channel,
    module_discover: ModuleDiscover,
}

impl Module {
    async fn build_discover_changes(
        &self,
        sender: &mpsc::Sender<tower::discover::Change<SocketAddr, Endpoint>>,
        changes: Vec<Change>,
    ) {
        for change in changes {
            match change {
                Change::Add((_, address)) | Change::Update((_, address)) => {
                    let endpoint = match Endpoint::from_shared(format!("http://{}", address)) {
                        Ok(v) => v.timeout(Duration::from_secs(5)).concurrency_limit(32).tcp_nodelay(true),
                        Err(err) => {
                            error!("error uri {}", err);
                            continue;
                        }
                    };
                    let _ = sender.send(tower::discover::Change::Insert(address, endpoint)).await;
                }
                Change::Remove((_, address)) => {
                    let _ = sender.send(tower::discover::Change::Remove(address)).await;
                }
            }
        }
    }

    async fn discover(
        self: Arc<Self>,
        ttl: u32,
        mut stop_rx: watch::Receiver<()>,
        sender: mpsc::Sender<tower::discover::Change<SocketAddr, Endpoint>>,
    ) {
        let mut rng = rand::rngs::StdRng::from_entropy();
        loop {
            match self.module_discover.watch().await {
                Ok(v) => self.build_discover_changes(&sender, v).await,
                Err(e) => {
                    error!("discover fail: {}", e);
                }
            }

            let mut sleep_millis: i32 = ttl as i32 * 1000;
            sleep_millis += rng.gen_range(-1000..=1000) * 10;
            sleep_millis = max(120000, sleep_millis);

            let sleep = tokio::time::sleep(Duration::from_millis(sleep_millis as u64));
            let changed = stop_rx.changed();
            // sleep random interval
            tokio::select! {
                _ = sleep => {
                },
                Ok(_) = changed => {
                    debug!("{} discover shutdown", self.module_discover.module);
                    break;
                }
            }
        }
    }

    pub fn channel(&self) -> Channel {
        self.channel.clone()
    }

    async fn make_config(module: String, config: Arc<MicroServiceConfig>, rx: watch::Receiver<()>) -> Arc<Self> {
        let (channel, sender) = Channel::balance_channel(100);
        let discover = ConfigDiscover::new(config.discover.file.clone().unwrap());
        let m = Arc::new(Module {
            channel,
            module_discover: ModuleDiscover::new(Box::new(discover), module),
        });
        tokio::spawn(m.clone().discover(config.discover.ttl, rx, sender));
        m
    }

    async fn make_k8s(module: String, config: Arc<MicroServiceConfig>, rx: watch::Receiver<()>) -> Arc<Self> {
        let (channel, sender) = Channel::balance_channel(100);
        let discover =
            K8sDiscover::make(config.discover.suffix.to_owned(), config.discover.name_server.to_owned()).await;
        let m = Arc::new(Module {
            channel,
            module_discover: ModuleDiscover::new(Box::new(discover), module),
        });
        tokio::spawn(m.clone().discover(config.discover.ttl, rx, sender));
        m
    }

    pub async fn make(module: String, config: Arc<MicroServiceConfig>, rx: watch::Receiver<()>) -> Arc<Self> {
        if config.discover.file.is_none() {
            Self::make_k8s(module, config, rx).await
        } else {
            Self::make_config(module, config, rx).await
        }
    }

    pub async fn fetch_static(addr: SocketAddr) -> Channel {
        Channel::from_shared(format!("http://{}", addr))
            .unwrap()
            .connect()
            .await
            .unwrap()
    }
}
