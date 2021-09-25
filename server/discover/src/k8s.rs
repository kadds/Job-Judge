use crate::*;
use std::{future::Future, str::FromStr};
use tokio::{
    net::UdpSocket,
    sync::{mpsc, Mutex},
};
use trust_dns_client::client::ClientHandle;
use trust_dns_client::udp::UdpClientStream;
use trust_dns_client::{
    client::AsyncClient,
    rr::{DNSClass, Name, RecordType},
};
pub struct K8sDiscover {
    suffix: String,
    client: Mutex<AsyncClient>,
    tx: mpsc::Sender<()>,
}

async fn run_exchange<T>(d: T, mut rx: mpsc::Receiver<()>)
where
    T: Future,
{
    let c = rx.recv();
    tokio::select! {
        _ = c => {
        },
        _ = d => {
        }
    }
}

impl K8sDiscover {
    pub async fn make(suffix: String, name_server: String) -> Self {
        let (tx, rx) = mpsc::channel(1);
        let stream = UdpClientStream::<UdpSocket>::new(name_server.parse().unwrap());
        let (client, d) = AsyncClient::connect(stream).await.unwrap();
        tokio::spawn(run_exchange(d, rx));
        K8sDiscover {
            suffix,
            client: Mutex::new(client),
            tx,
        }
    }
}

impl Drop for K8sDiscover {
    fn drop(&mut self) {
        let _ = self.tx.send(());
    }
}

#[async_trait]
impl Discover for K8sDiscover {
    async fn get_from_module(&self, module_name: &str) -> Result<Vec<(String, SocketAddr)>> {
        let dns_url = format!("{}.{}", module_name, self.suffix);
        let dns = self
            .client
            .lock()
            .await
            .query(Name::from_str(&dns_url)?, DNSClass::IN, RecordType::A)
            .await?;

        dns.answers()
            .iter()
            .map(|record| {
                record
                    .rdata()
                    .to_ip_addr()
                    .map(|v| (record.name().to_string(), SocketAddr::new(v, 11100)))
                    .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "not valid ip addr"))
            })
            .collect()
    }

    async fn get_from_server(&self, module_name: &str, server_name: &str) -> Result<Option<SocketAddr>> {
        let dns_url = format!("{}.{}.{}", server_name, module_name, self.suffix);
        if let Some(record) = self
            .client
            .lock()
            .await
            .query(Name::from_str(&dns_url)?, DNSClass::IN, RecordType::A)
            .await?
            .answers()
            .first()
        {
            return Ok(record.rdata().to_ip_addr().map(|v| SocketAddr::new(v, 11100)));
        }
        Ok(None)
    }
}
