use log::*;
use micro_service::discover::*;
use std::net::SocketAddr;
use thiserror::Error;
#[derive(Error, Debug)]
pub enum GrpcError {
    #[error("network error")]
    NetError,

    #[error("io error {0}")]
    IOError(#[from] std::io::Error),

    #[error("service not found. {0}")]
    ServiceNotFound(String),

    #[error("rpc not found. {0}")]
    RpcNotFound(String),

    #[error("instance not found. {0}")]
    InstanceNotFound(String),

    #[error("empty instance in module {0}")]
    EmptyInstance(String),

    #[error("invoke rpc fail {0}")]
    InvokeFail(#[from] tonic::Status),

    #[error("invalid input parameters")]
    InvalidParameters,

    #[error("invalid url")]
    InvalidUri,

    #[error("encode fail. {0}")]
    EncodeError(#[from] any_message::EncodeError),

    #[error("format fail. {0}")]
    DecodeError(#[from] prost::DecodeError),

    #[error("logic fail. {0}")]
    LogicError(&'static str),
}

pub type GrpcResult<T> = std::result::Result<T, GrpcError>;

async fn get_channel(addr: SocketAddr) -> GrpcResult<tonic::transport::Channel> {
    let endpoint = match tonic::transport::Endpoint::from_shared(format!("http://{}", addr)) {
        Ok(v) => v.timeout(std::time::Duration::from_secs(5)).tcp_nodelay(true),
        Err(err) => {
            error!("error uri {}", err);
            return Err(GrpcError::InvalidUri);
        }
    };

    let channel = endpoint.connect().await.map_err(|_| GrpcError::NetError)?;
    Ok(channel)
}

async fn get_module_address(
    cfg: &micro_service::cfg::DiscoverConfig,
    module: &str,
) -> Result<Vec<(String, SocketAddr)>, std::io::Error> {
    if let Some(file) = &cfg.file {
        let d = ConfigDiscover::new(file.to_owned());
        d.get_from_module(module).await
    } else {
        let d = K8sDiscover::make(cfg.suffix.to_owned(), cfg.name_server.to_owned()).await;
        d.get_from_module(module).await
    }
}

#[allow(dead_code)]
async fn get_module_instance_address(
    cfg: &micro_service::cfg::DiscoverConfig,
    module: &str,
    name: &str,
) -> Result<Option<SocketAddr>, std::io::Error> {
    if let Some(file) = &cfg.file {
        let d = ConfigDiscover::new(file.to_owned());
        d.get_from_server(module, name).await
    } else {
        let d = K8sDiscover::make(cfg.suffix.to_owned(), cfg.name_server.to_owned()).await;
        d.get_from_server(module, name).await
    }
}

pub async fn list_modules(cfg: &micro_service::cfg::DiscoverConfig) -> Result<Vec<String>, std::io::Error> {
    if let Some(file) = &cfg.file {
        let d = ConfigDiscover::new(file.to_owned());
        d.list_modules().await
    } else {
        let d = K8sDiscover::make(cfg.suffix.to_owned(), cfg.name_server.to_owned()).await;
        d.list_modules().await
    }
}

mod any_message;
pub mod reflection;
pub use reflection::RequestContext;
