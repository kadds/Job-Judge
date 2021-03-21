use crate::cfg::*;
use log::*;
use rand::{Rng, SeedableRng};
use std::{cmp::max, sync::Arc};
use std::{net::SocketAddr, time::Duration};
use tokio::sync::mpsc;
use tokio::sync::watch;
use tonic::transport::{Channel, Endpoint};
use tower::discover::Change;

pub struct Module {
    pub(crate) channel: Channel,
    pub(crate) config: Arc<MicroServiceConfig>,
    module_discover: discover::ModuleDiscover<discover::K8sDiscover>,
}

impl Module {
    pub async fn make(
        module: String,
        config: Arc<MicroServiceConfig>,
        rx: watch::Receiver<()>,
    ) -> Arc<Self> {
        let (channel, sender) = Channel::balance_channel(100);
        let discover = discover::K8sDiscover::make(
            config.discover.suffix.to_owned(),
            config.discover.name_server.to_owned(),
        )
        .await;
        let m = Arc::new(Module {
            channel,
            config,
            module_discover: discover::ModuleDiscover::new(discover, module),
        });
        tokio::spawn(m.clone().discover(rx, sender));
        m
    }

    async fn build_discover_changes(
        &self,
        sender: &mpsc::Sender<Change<SocketAddr, Endpoint>>,
        changes: Vec<discover::Change>,
    ) {
        for change in changes {
            match change {
                discover::Change::Add((_, address)) | discover::Change::Update((_, address)) => {
                    let endpoint = match Endpoint::from_shared(format!("http://{}", address)) {
                        Ok(v) => v
                            .timeout(Duration::from_secs(5))
                            .concurrency_limit(32)
                            .tcp_nodelay(true),
                        Err(err) => {
                            error!("error uri {}", err);
                            continue;
                        }
                    };
                    let _ = sender.send(Change::Insert(address, endpoint)).await;
                }
                discover::Change::Remove((_, address)) => {
                    let _ = sender.send(Change::Remove(address)).await;
                }
            }
        }
    }

    pub async fn discover(
        self: Arc<Self>,
        mut stop_rx: watch::Receiver<()>,
        sender: mpsc::Sender<Change<SocketAddr, Endpoint>>,
    ) {
        let mut rng = rand::rngs::StdRng::from_entropy();
        loop {
            match self.module_discover.watch().await {
                Ok(v) => self.build_discover_changes(&sender, v).await,
                Err(e) => {
                    error!("discover fail: {}", e);
                }
            }

            let mut sleep_millis: i32 = self.config.discover.ttl as i32 * 1000;
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
        self.module_discover.discover.stop();
    }

    pub fn channel(&self) -> Channel {
        self.channel.clone()
    }
}
