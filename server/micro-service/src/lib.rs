use async_trait::async_trait;
use etcd_rs::*;
use heim_net;
use log::{debug, error, info, warn};
use tokio::stream::StreamExt;
use tonic;

pub enum Error {
    ConnectionFailed,
    Timeout,
    CheckFailed,
    ResourceLimit,
    Unknown,
}

pub struct MicroService {
    etcd: Client,
    prefix: String,
    name: String,
    lease_id: u64,
}

type Result<T> = std::result::Result<T, Error>;

#[async_trait]
pub trait MicroServiceImpl {
    async fn init(
        urls: Vec<String>,
        user: String,
        password: String,
        prefix: String,
        name: String,
        retry_times: u32,
    ) -> Result<MicroService>;

    async fn register_self(&mut self, addr: String, ttl: u64, retry_times: u32) -> Result<()>;

    async fn update_self(&mut self, workload: u32) -> Result<()>;

    async fn watch_server(
        &mut self,
        prefix: String,
        f: fn(Vec<(String, String)>, bool) -> (),
    ) -> Result<()>;

    fn get_server(&mut self, uin: u64, prefix: String) -> Option<String>;
}

async fn do_keep_alive(lease: &mut Lease, lease_id: u64) -> Result<()> {
    let _rsp = lease.keep_alive(LeaseKeepAliveRequest::new(lease_id)).await;
    Ok(())
}

async fn do_put_workload(kv: &mut Kv, prefix: &String, name: &String, workload: u32) -> Result<()> {
    kv.put(PutRequest::new(
        format!("/jj/servers/{}/{}/workload", prefix, name),
        format!("{}", workload),
    ))
    .await
    .and(Ok(()))
    .or(Err(Error::ConnectionFailed))
}

#[async_trait]
impl MicroServiceImpl for MicroService {
    async fn init(
        urls: Vec<String>,
        user: String,
        password: String,
        prefix: String,
        name: String,
        retry_times: u32,
    ) -> Result<MicroService> {
        for _ in 0..retry_times {
            let client = Client::connect(ClientConfig {
                endpoints: urls.to_owned(),
                auth: Some((user.to_owned(), password.to_owned())),
                tls: None,
            })
            .await;
            if let Ok(v) = client {
                return Ok(MicroService {
                    etcd: v,
                    prefix,
                    name,
                    lease_id: 0,
                });
            }
        }
        error!("etcd connect failed. try {} times", retry_times);
        Err(Error::ConnectionFailed)
    }

    async fn register_self(&mut self, addr: String, ttl: u64, retry_times: u32) -> Result<()> {
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
                format!("/jj/servers/{}/{}/address", self.prefix, self.name),
                addr.to_owned(),
            );
            req.set_lease(lease_id);
            if self.etcd.kv().put(req).await.is_ok() {
                self.lease_id = lease_id;
                return Ok(());
            }
            warn!("micro-service register failed, retry...");
        }
        error!(
            "micro-service '{}' with address {} register failed. try {} times",
            self.name, addr, retry_times
        );

        Err(Error::ResourceLimit)
    }

    async fn update_self(&mut self, workload: u32) -> Result<()> {
        let mut lease = self.etcd.lease();
        let mut kv = self.etcd.kv();
        let res = tokio::try_join!(
            do_keep_alive(&mut lease, self.lease_id),
            do_put_workload(&mut kv, &mut self.prefix, &mut self.name, workload)
        );

        match res {
            Ok(_) => Ok(()),
            Err(err) => {
                error!("micro-service heartbeat failed");
                Err(Error::ConnectionFailed)
            }
        }
    }

    async fn watch_server(
        &mut self,
        prefix: String,
        f: fn(Vec<(String, String)>, bool) -> (),
    ) -> Result<()> {
        let mut wc = self.etcd.watch_client();
        let mut inbound = wc.watch(KeyRange::prefix(prefix)).await;
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
                        EventType::Delete => {}
                    }
                }

                if put_vec.len() > 0 {
                    f(put_vec, true);
                }
                if del_vec.len() > 0 {
                    f(del_vec, false);
                }
            }
        });

        Ok(())
    }
}

fn address_to_string(addr: heim_net::Address) -> String {
    match addr {
        heim_net::Address::Inet(d) => d.to_string(),
        heim_net::Address::Inet6(d) => d.to_string(),
        heim_net::Address::Link(d) => d.to_string(),
        _ => "Unknown".to_owned(),
    }
}

pub async fn get_nic_ip(eth: String) -> Option<String> {
    let mut t = heim_net::nic();
    while let Some(ret) = t.next().await {
        match ret {
            Ok(v) => {
                info!(
                    "NIC {} address {} mask {}",
                    v.name(),
                    address_to_string(v.address()),
                    v.netmask()
                        .map(|d| address_to_string(d))
                        .unwrap_or("Unknown".to_owned())
                        .to_string()
                );

                if v.name() == eth {
                    match v.address() {
                        heim_net::Address::Inet(addr) => return Some(addr.to_string()),
                        _ => {
                            error!("get NIC ipv4 address failed");
                        }
                    };
                }
            }
            Err(err) => {
                error!("get NIC info failed, error {}", err);
            }
        };
    }
    None
}
