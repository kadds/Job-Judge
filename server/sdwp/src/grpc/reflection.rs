use std::net::SocketAddr;

use log::*;
use reflection::client::*;

use super::{get_channel, get_module_address, GrpcError, GrpcResult};
pub struct Meta {
    pub inner_services: Vec<String>,
    pub description: String,
    pub instances: Vec<(String, SocketAddr)>,
}

pub async fn get_meta(cfg: &crate::cfg::Config, module_name: &str) -> GrpcResult<Meta> {
    let addrs = get_module_address(cfg, module_name).await?;

    let addr = addrs.first().ok_or(GrpcError::NotFound)?.1;

    let channel = get_channel(addr).await?;
    let mut client = ReflectionSvrClient::new(channel);
    let req = GetMetaReq {};
    let rsp = client
        .get_meta(req)
        .await
        .map_err(|_| GrpcError::NotFound)?
        .into_inner();
    info!("get {} description {}", module_name, rsp.description);
    Ok(Meta {
        inner_services: rsp.services,
        description: rsp.description,
        instances: addrs,
    })
}

pub async fn get_instance_address(
    cfg: &crate::cfg::Config,
    module_name: &str,
    instance_name: &str,
) -> GrpcResult<(String, SocketAddr)> {
    let meta = get_meta(cfg, module_name).await?;
    Ok(meta
        .instances
        .into_iter()
        .find(|v| v.0 == instance_name)
        .ok_or(GrpcError::NotFound)?)
}

pub async fn get_rpcs(service_name: String, addr: SocketAddr) -> Result<Vec<String>, GrpcError> {
    let channel = get_channel(addr).await?;
    let mut client = ReflectionSvrClient::new(channel);
    let req = GetRpcReq {
        service_name,
        rpc_name: "".to_owned(),
    };
    let rsp = client
        .get_rpc(req)
        .await
        .map_err(|_| GrpcError::NotFound)?
        .into_inner();
    Ok(rsp.rpc.into_iter().map(|v| v.name).collect())
}
