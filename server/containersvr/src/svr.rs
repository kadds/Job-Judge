use super::config;
use super::docker;
use log::{debug, info, trace, warn};
use rpc::container_svr_server::{ContainerSvr, ContainerSvrServer};
use rpc::{Instance, ShutdownResult, StartupRequest, State};
use std::io::{Read, Write};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tonic::{Request, Response, Status};

pub mod rpc {
    tonic::include_proto!("rpc");
}

#[derive(Debug, Default)]
pub struct ContainerSvrImpl {}

#[tonic::async_trait]
impl ContainerSvr for ContainerSvrImpl {
    async fn startup(&self, request: Request<StartupRequest>) -> Result<Instance, Status> {
        let req = request.into_inner();
        let cfg = match config::from(req.config) {
            Some(v) => v,
            None => {
                return Err(Status::not_found("config not find"));
            }
        };

        if cfg.container == "docker" {
            docker::run(
                name: req.name,
                cfg.img_src,
                cfg.mem_limit,
                cfg.vcpu_cnt,
                cfg.vcpu_percent,
                cfg.io_speed_limit,
            );
        }

        Ok(Instance {})
    }

    async fn state(&self, request: Request<Instance>) -> Result<State, Status> {}

    async fn shutdown(&self, request: Request<Instance>) -> Result<ShutdownResult, Status> {}
}

pub fn get() -> ContainerSvrServer<ContainerSvrImpl> {
    return ContainerSvrServer::new(ContainerSvrImpl::default());
}
