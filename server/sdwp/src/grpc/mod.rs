use discover::Discover;
use log::*;
use std::net::SocketAddr;
use thiserror::Error;
#[derive(Error, Debug)]
pub enum GrpcError {
    #[error("network error")]
    NetError,
    #[error("io error {0}")]
    IOError(#[from] std::io::Error),
    #[error("not found")]
    NotFound,
    #[error("invalid url")]
    InvalidUri,
}

async fn get_channel(addr: SocketAddr) -> Result<tonic::transport::Channel, GrpcError> {
    let endpoint = match tonic::transport::Endpoint::from_shared(format!("http://{}", addr)) {
        Ok(v) => v
            .timeout(std::time::Duration::from_secs(5))
            .tcp_nodelay(true),
        Err(err) => {
            error!("error uri {}", err);
            return Err(GrpcError::InvalidUri);
        }
    };

    let channel = endpoint.connect().await.map_err(|_| GrpcError::NetError)?;
    Ok(channel)
}

async fn get_module_address(
    cfg: &crate::cfg::Config,
    module: &str,
) -> Result<Vec<(String, SocketAddr)>, std::io::Error> {
    let d = discover::K8sDiscover::make(
        cfg.discover_suffix.to_owned(),
        cfg.discover_name_server.to_owned(),
    )
    .await;
    let r = d.get_from_module(module).await;
    d.stop();
    r
}

async fn get_server_address(
    cfg: &crate::cfg::Config,
    module: &str,
    name: &str,
) -> Result<Option<SocketAddr>, std::io::Error> {
    let d = discover::K8sDiscover::make(
        cfg.discover_suffix.to_owned(),
        cfg.discover_name_server.to_owned(),
    )
    .await;
    let r = d.get_from_server(module, name).await;
    d.stop();
    r
}
