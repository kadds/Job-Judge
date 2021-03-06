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
    #[error("grpc not found")]
    NotFound,
    #[error("grpc call fail {0}")]
    CallFail(#[from] tonic::Status),
    #[error("grpc call result fail")]
    InvalidResult,
    #[error("invalid url")]
    InvalidUri,
}
type GrpcResult<T> = std::result::Result<T, GrpcError>;

async fn get_channel(addr: SocketAddr) -> GrpcResult<tonic::transport::Channel> {
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
    match cfg.discover_file.len() {
        0 => {
            let d = discover::K8sDiscover::make(
                cfg.discover_suffix.to_owned(),
                cfg.discover_name_server.to_owned(),
            )
            .await;
            d.get_from_module(module).await
        }
        _ => {
            let d = discover::ConfigDiscover::new(cfg.discover_file.to_owned());
            d.get_from_module(module).await
        }
    }
}

async fn get_module_instance_address(
    cfg: &crate::cfg::Config,
    module: &str,
    name: &str,
) -> Result<Option<SocketAddr>, std::io::Error> {
    match cfg.discover_file.len() {
        0 => {
            let d = discover::K8sDiscover::make(
                cfg.discover_suffix.to_owned(),
                cfg.discover_name_server.to_owned(),
            )
            .await;
            d.get_from_server(module, name).await
        }
        _ => {
            let d = discover::ConfigDiscover::new(cfg.discover_file.to_owned());
            d.get_from_server(module, name).await
        }
    }
}

pub mod reflection;
