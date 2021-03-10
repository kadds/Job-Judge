use std::{
    sync::Arc,
};
use tonic::{Request, Response, Status};

pub mod id {
    pub mod rpc {
        tonic::include_proto!("id.rpc");
    }
}

use id::rpc::id_svr_server::{IdSvr, IdSvrServer};
use id::rpc::*;

pub struct IdSvrImpl {
    replica_id: u32,
}

// TODO: query id segment
async fn gen_uid(replica_id: u32) -> i64 {
    (replica_id + 1) as i64
}

#[tonic::async_trait]
impl IdSvr for IdSvrImpl {
    async fn create_uid(
        &self,
        _request: Request<CreateUidReq>,
    ) -> Result<Response<CreateUidRsp>, Status> {
        let uid = gen_uid(self.replica_id).await;
        Ok(Response::new(CreateUidRsp { uid }))
    }

}

pub async fn get(server: Arc<micro_service::Server>) -> IdSvrServer<IdSvrImpl> {
    return IdSvrServer::new(IdSvrImpl {
        replica_id: 0,
    });
}
