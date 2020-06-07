use log::{debug, info, trace, warn};
use rpc::user_svr_server::{UserSvr, UserSvrServer};
use rpc::*;
use std::fs::File;
use std::io::{Read, Write};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tonic::{Request, Response, Status};

pub mod rpc {
    tonic::include_proto!("rpc");
}
pub mod user {
    tonic::include_proto!("user");
}

#[derive(Debug, Default)]
pub struct UserSvrImpl {}

#[tonic::async_trait]
impl UserSvr for UserSvrImpl {
    async fn create_user(
        &self,
        request: Request<CreateUserRequest>,
    ) -> Result<Response<CreateUserResult>, Status> {
    }
    async fn valid_user(
        &self,
        request: Request<ValidUserRequest>,
    ) -> Result<Response<ValidUserResult>, Status> {
    }
    async fn get_user(
        &self,
        request: Request<GetUserRequest>,
    ) -> Result<Response<GetUserResult>, Status> {
    }
    async fn update_user(
        &self,
        request: Request<UpdateUserRequest>,
    ) -> Result<Response<UpdateUserResult>, Status> {
    }
    async fn has_user(
        &self,
        request: Request<HasUserRequest>,
    ) -> Result<Response<HasUserResult>, Status> {
    }
}

pub fn get() -> UserSvrImpl {
    return;
}
