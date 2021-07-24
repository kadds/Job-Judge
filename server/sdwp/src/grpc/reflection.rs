use futures_util::stream;
use std::net::SocketAddr;

tonic::include_proto!("grpc.reflection.v1alpha");

use log::*;
use server_reflection_client::ServerReflectionClient;

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
    let mut client = ServerReflectionClient::new(channel);
    let req = ServerReflectionRequest::default();
    let rsp = client
        .server_reflection_info(tonic::Request::new(stream::iter([req])))
        .await?
        .into_inner();
    todo!();
    // Ok(Meta {
    //     inner_services: rsp.services,
    //     description: rsp.description,
    //     instances: addrs,
    // })
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
    let mut client = ServerReflectionClient::new(channel);

    let req = ServerReflectionRequest::default();
    let rsp = client
        .server_reflection_info(tonic::Request::new(stream::iter([req])))
        .await?
        .into_inner();
    todo!();
    Ok(rsp)
}
