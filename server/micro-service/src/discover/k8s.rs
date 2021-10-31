use super::{Discover, Result};
use async_trait::*;
use core::fmt::Debug;
use std::{net::SocketAddr, str::FromStr};
use trust_dns_resolver::{
    proto::{
        rr::{Record, RecordType},
        xfer::DnsRequestOptions,
    },
    Name, TokioAsyncResolver,
};

pub struct K8sDiscover {
    suffix: String,
    name_server: String,
    resolver: TokioAsyncResolver,
}

impl Debug for K8sDiscover {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("K8sDiscover")
            .field("suffix", &self.suffix)
            .field("name_server", &self.name_server)
            .finish()
    }
}

impl K8sDiscover {
    pub async fn make(suffix: String, name_server: String) -> Self {
        let resolver = TokioAsyncResolver::tokio_from_system_conf().unwrap();
        K8sDiscover {
            suffix,
            name_server,
            resolver,
        }
    }
}

fn record_map(record: &Record) -> Result<(String, SocketAddr)> {
    record
        .rdata()
        .to_ip_addr()
        .map(|v| (record.name().to_string(), SocketAddr::new(v, 11100)))
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "not valid ip addr"))
}

#[async_trait]
impl Discover for K8sDiscover {
    async fn get_from_module(&self, module_name: &str) -> Result<Vec<(String, SocketAddr)>> {
        let dns_url = format!("{}.{}", module_name, self.suffix);
        let lookup = self
            .resolver
            .lookup(Name::from_str(&dns_url)?, RecordType::A, DnsRequestOptions::default())
            .await?;

        lookup.record_iter().map(record_map).collect()
    }

    async fn get_from_server(&self, module_name: &str, server_name: &str) -> Result<Option<SocketAddr>> {
        let dns_url = format!("{}.{}.{}", server_name, module_name, self.suffix);
        let lookup = self
            .resolver
            .lookup(Name::from_str(&dns_url)?, RecordType::A, DnsRequestOptions::default())
            .await?;
        for item in lookup.record_iter() {
            let ret = record_map(item)?;
            return Ok(Some(ret.1));
        }
        Ok(None)
    }

    async fn list_modules(&self) -> Result<Vec<String>> {
        let dns = self
            .resolver
            .lookup(Name::default(), RecordType::ANY, DnsRequestOptions::default())
            .await?;

        dns.record_iter().map(|record| Ok(record.name().to_string())).collect()
    }
}
