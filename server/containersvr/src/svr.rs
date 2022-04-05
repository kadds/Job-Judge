use log::error;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

pub mod container {
    pub mod rpc {
        tonic::include_proto!("container.rpc");
    }
}
pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("descriptor");
use container::rpc::container_svr_server::{ContainerSvr, ContainerSvrServer};
use container::rpc::*;

use crate::mgr::Mgr;
use crate::{config, daemon};

#[derive(Debug)]
pub struct ContainerSvrImpl {
    cfg: config::Config,
}

#[tonic::async_trait]
impl ContainerSvr for ContainerSvrImpl {
    async fn startup(&self, request: Request<StartupReq>) -> Result<Response<StartupRsp>, Status> {
        let mut mgr = Mgr::new(&self.cfg);
        let req = request.into_inner();
        match mgr.startup(req).await {
            Ok(rsp) => Ok(Response::new(rsp)),
            Err(e) => {
                error!("{}", e);
                Err(Status::internal(format!("container svr inner error: {}", e)))
            }
        }
    }
    async fn get_state(&self, _request: Request<GetStateReq>) -> Result<Response<GetStateRsp>, Status> {
        todo!()
    }
    async fn shutdown(&self, _request: Request<ShutdownReq>) -> Result<Response<ShutdownRsp>, Status> {
        todo!()
    }
}

pub async fn get(server: Arc<micro_service::Server>, listener: TcpListener) {
    // load containers config from file
    let cfg = config::read(&server.config()).await.unwrap();
    daemon::start(cfg.clone());
    let svr = ContainerSvrServer::new(ContainerSvrImpl { cfg });
    let reflection_svr = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();

    Server::builder()
        .add_service(svr)
        .add_service(reflection_svr)
        .serve_with_incoming_shutdown(TcpListenerStream::new(listener), server.wait_stop_signal())
        .await
        .expect("start server fail");
}
