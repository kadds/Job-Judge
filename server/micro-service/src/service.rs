use crate::cfg::*;
use chrono::NaiveDateTime;
use log::*;
use rand::{Rng, SeedableRng};
use std::{cmp::max, sync::Arc, time::UNIX_EPOCH};
use std::{
    collections::HashMap,
    net::SocketAddr,
    time::{Duration, SystemTime},
};
use tokio::sync::mpsc;
use tokio::sync::{watch, Mutex};
use tonic::transport::{Channel, Endpoint};
use tower::discover::Change;

pub struct Module {
    pub(crate) module: String, // module name
    pub(crate) dns_url: String,
    pub(crate) services: Mutex<HashMap<SocketAddr, Mutex<Service>>>,
    pub(crate) channel: Channel,
    pub(crate) config: Arc<MicroServiceConfig>,
}

impl Module {
    pub fn new(
        module: String,
        config: Arc<MicroServiceConfig>,
        rx: watch::Receiver<()>,
    ) -> Arc<Self> {
        let dns = config.discover.dns_template.clone().replace("{}", &module);
        let (channel, sender) = Channel::balance_channel(100);
        let m = Arc::new(Module {
            module,
            dns_url: dns,
            services: Mutex::new(HashMap::new()),
            channel,
            config,
        });
        tokio::spawn(m.clone().discover(rx, sender));
        m
    }

    async fn build_discover_changes(
        &self,
        sender: &mpsc::Sender<Change<SocketAddr, Endpoint>>,
        changes: Vec<Change<SocketAddr, ()>>,
    ) {
        let now = SystemTime::now();
        for service in self.services.lock().await.values_mut() {
            service.lock().await.mtime = now;
        }
        for change in changes {
            match change {
                Change::Insert(address, _) => {
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
                    self.services
                        .lock()
                        .await
                        .insert(address, Mutex::new(Service::new(address)));
                    let _ = sender.send(Change::Insert(address, endpoint)).await;
                }
                Change::Remove(address) => {
                    self.services.lock().await.remove(&address);
                    let _ = sender.send(Change::Remove(address)).await;
                }
            }
        }
    }

    pub async fn discover(
        self: Arc<Self>,
        mut rx: watch::Receiver<()>,
        sender: mpsc::Sender<Change<SocketAddr, Endpoint>>,
    ) {
        let mut rng = rand::rngs::StdRng::from_entropy();
        loop {
            trace!("{} discover loading", self.module);
            let mut sleep_millis = 10000;
            let res = match &self.config.discover.file {
                None => {
                    sleep_millis = rng.gen_range(-1000..=1000) * 10 + 60000;
                    self.discover_from_dns().await
                }
                Some(f) => self.discover_from_file(f).await,
            };
            match res {
                Ok(v) => {
                    self.build_discover_changes(&sender, v).await;
                }
                Err(e) => {
                    error!("discover fail error {}", e);
                    sleep_millis -= 30000;
                }
            };
            sleep_millis = max(10000, sleep_millis);
            let sleep = tokio::time::sleep(Duration::from_millis(sleep_millis as u64));
            let changed = rx.changed();
            // sleep random interval
            tokio::select! {
                _ = sleep => {
                },
                Ok(_) = changed => {
                    debug!("{} discover shutdown", self.module);
                    break;
                }
            }
        }
    }

    pub fn channel(&self) -> Channel {
        self.channel.clone()
    }
}
pub struct Service {
    pub(crate) address: SocketAddr,
    pub(crate) ctime: SystemTime,
    pub(crate) mtime: SystemTime,
}

impl std::fmt::Display for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} created at {} modified at {}",
            self.address,
            NaiveDateTime::from_timestamp(
                self.ctime
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |v| v.as_secs() as i64),
                0
            ),
            NaiveDateTime::from_timestamp(
                self.mtime
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |v| v.as_secs() as i64),
                0
            ),
        )
    }
}

impl Service {
    pub fn new(address: SocketAddr) -> Self {
        let now = SystemTime::now();
        Service {
            address,
            ctime: now,
            mtime: now,
        }
    }
}
