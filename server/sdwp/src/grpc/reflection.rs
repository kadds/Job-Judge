use std::net::SocketAddr;

use log::*;
use reflection::client::*;

use super::{get_channel, get_module_address, GrpcError, GrpcResult};
#[derive(Debug)]
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
    let rsp = client.get_meta(req).await?.into_inner();
    debug!(
        "{} description {} services {:?}",
        module_name, rsp.description, rsp.services
    );
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
) -> GrpcResult<SocketAddr> {
    let meta = get_meta(cfg, module_name).await?;
    Ok(meta
        .instances
        .into_iter()
        .find(|v| v.0 == instance_name)
        .ok_or(GrpcError::NotFound)?
        .1)
}

pub async fn get_rpcs(service_name: &str, addr: SocketAddr) -> Result<Vec<String>, GrpcError> {
    info!("{}", addr);
    let channel = get_channel(addr).await?;
    let mut client = ReflectionSvrClient::new(channel);
    let req = GetRpcReq {
        service_name: service_name.to_owned(),
        rpc_name: "".to_owned(),
    };
    let rsp = client.get_rpc(req).await?.into_inner();
    let rsp = match rsp.res.ok_or(GrpcError::InvalidResult)? {
        get_rpc_rsp::Res::Rpcs(v) => v.name,
        _ => return Err(GrpcError::InvalidResult),
    };
    Ok(rsp)
}
