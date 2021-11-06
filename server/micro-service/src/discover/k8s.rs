use super::{Discover, Result};
use async_trait::*;
use core::fmt::Debug;
use std::{
    io::Error,
    io::ErrorKind,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
};
use trust_dns_resolver::{
    proto::{
        rr::{RData, RecordType},
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
    async fn query_host_from_ip(&self, ip: &Ipv4Addr) -> Result<String> {
        let octets = ip.octets();
        let dns_url = format!("{}.{}.{}.{}.in-addr.arpa", octets[3], octets[2], octets[1], octets[0]);
        let res = self
            .resolver
            .lookup(Name::from_str(&dns_url).unwrap(), RecordType::PTR, DnsRequestOptions::default())
            .await?;
        let res = res
            .record_iter()
            .next()
            .ok_or_else(|| Error::new(ErrorKind::InvalidData, "empty result set"))
            .and_then(|item| {
                if let RData::PTR(name) = item.rdata() {
                    return Ok(name.to_string());
                }
                Err(Error::new(ErrorKind::InvalidData, "PTR type not found"))
            });
        res
    }
    async fn query_module_name_from_ip(&self, ip: &Ipv4Addr) -> Result<String> {
        let host = self.query_host_from_ip(ip).await?;
        return Ok(host
            .replace(&self.suffix, "")
            .rsplit('.')
            .find(|item| !item.is_empty())
            .unwrap_or_default()
            .to_owned());
    }

    async fn query_pod_name_from_ip(&self, ip: &Ipv4Addr) -> Result<String> {
        let host = self.query_host_from_ip(ip).await?;
        return Ok(host
            .replace(&self.suffix, "")
            .split('.')
            .find(|item| !item.is_empty())
            .unwrap_or_default()
            .to_owned());
    }

    async fn query_pod_name_and_ip(&self, ip: Ipv4Addr) -> Result<(String, SocketAddr)> {
        let ret = self.query_pod_name_from_ip(&ip).await?;
        Ok((ret, SocketAddr::new(IpAddr::V4(ip), 11100)))
    }
}

#[async_trait]
impl Discover for K8sDiscover {
    async fn list_instances(&self, module_name: &str) -> Result<Vec<(String, SocketAddr)>> {
        let dns_url = format!("{}.{}", module_name, self.suffix);
        let lookup = self
            .resolver
            .lookup(Name::from_str(&dns_url)?, RecordType::A, DnsRequestOptions::default())
            .await?;

        let mut log_string = String::new();
        let all_fut = lookup.record_iter().filter_map(|record| {
            if let RData::A(ip) = record.rdata() {
                log_string.push_str(&format!("{},", ip));
                Some(self.query_pod_name_and_ip(ip.to_owned()))
            } else {
                None
            }
        });

        let res = futures::future::join_all(all_fut).await.into_iter().collect();

        log::debug!("request instance lists: {}", log_string);
        res
    }

    async fn list_modules(&self) -> Result<Vec<String>> {
        let dns_url = format!("*.{}", self.suffix);
        let lookup = self
            .resolver
            .lookup(Name::from_str(&dns_url)?, RecordType::A, DnsRequestOptions::default())
            .await?;

        // for each of record, query service name

        let mut log_string = String::new();
        let all_fut = lookup.record_iter().filter_map(|record| {
            if let RData::A(ip) = record.rdata() {
                log_string.push_str(&format!("{},", ip));
                Some(self.query_module_name_from_ip(ip))
            } else {
                None
            }
        });

        let mut res: Vec<String> = futures::future::join_all(all_fut)
            .await
            .into_iter()
            .collect::<Result<Vec<String>>>()?;
        res.sort();
        log::debug!("request module lists (ip):{}", log_string);

        let mut final_set = Vec::<String>::new();
        for item in res.into_iter() {
            if let Some(ele) = final_set.last() {
                if *ele != item {
                    final_set.push(item);
                }
            } else {
                final_set.push(item);
            }
        }

        Ok(final_set)
    }
}
