use hmac::{Hmac, NewMac};
use jwt::{SignWithKey, VerifyWithKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex;
use tonic::{Request, Response, Status};

pub mod session {
    pub mod rpc {
        tonic::include_proto!("session.rpc");
    }
}

use session::rpc::session_svr_server::{SessionSvr, SessionSvrServer};
use session::rpc::*;

pub struct SessionSvrImpl {
    black_map: Mutex<HashMap<String, u64>>,
    key: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct Content {
    timeout: u32,
    expire_at: u64,
    map: HashMap<String, String>,
}

fn gen_session_token(key: &[u8], content: Content) -> Result<String, jwt::Error> {
    let k: Hmac<Sha256> = Hmac::new_varkey(key).unwrap();
    content.sign_with_key(&k)
}

fn to_content(key: &[u8], token: String) -> Result<Content, jwt::Error> {
    let k: Hmac<Sha256> = Hmac::new_varkey(key).unwrap();
    token.verify_with_key(&k)
}

#[tonic::async_trait]
impl SessionSvr for SessionSvrImpl {
    async fn create_session(
        &self,
        request: Request<CreateSessionReq>,
    ) -> Result<Response<CreateSessionRsp>, Status> {
        let req = request.into_inner();
        let content = Content {
            timeout: req.timeout,
            expire_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |v| v.as_secs() as u64)
                + req.timeout as u64,
            map: req.comm_data,
        };
        match gen_session_token(&self.key, content) {
            Ok(token) => Ok(Response::new(CreateSessionRsp { key: token })),
            Err(e) => Err(Status::internal(format!("{}", e))),
        }
    }

    async fn get_session(
        &self,
        request: Request<GetSessionReq>,
    ) -> Result<Response<GetSessionRsp>, Status> {
        let req = request.into_inner();
        {
            let mut map = self.black_map.lock().await;
            if let Some((_, v)) = map.get_key_value(&req.key) {
                // delete timeout key
                if *v
                    <= SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map_or(0, |v| v.as_secs() as u64)
                {
                    map.remove(&req.key);
                }
                return Err(Status::not_found("session key not found")); // in black list
            }
        }
        match to_content(&self.key, req.key) {
            Ok(content) => Ok(Response::new(GetSessionRsp {
                timeout: content.timeout,
                comm_data: content.map,
            })),
            Err(_) => Err(Status::not_found("session key not found")),
        }
    }

    async fn delay_session(
        &self,
        request: Request<DelaySessionReq>,
    ) -> Result<Response<DelaySessionRsp>, Status> {
        let req = request.into_inner();
        let mut content = match to_content(&self.key, req.key) {
            Ok(content) => content,
            Err(_) => return Err(Status::not_found("session key not found")),
        };
        if req.timeout != 0 {
            content.timeout = req.timeout;
        }
        match gen_session_token(&self.key, content) {
            Ok(token) => Ok(Response::new(DelaySessionRsp { new_key: token })),
            Err(e) => Err(Status::internal(format!("{}", e))),
        }
    }

    async fn invalid_session(
        &self,
        request: Request<InvalidSessionReq>,
    ) -> Result<Response<InvalidSessionRsp>, Status> {
        let req = request.into_inner();
        let content = match to_content(&self.key, req.key.clone()) {
            Ok(content) => content,
            Err(_) => return Ok(Response::new(InvalidSessionRsp {})),
        };
        if content.expire_at
            > SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |v| v.as_secs() as u64)
        {
            self.black_map
                .lock()
                .await
                .insert(req.key, content.expire_at);
        }
        Ok(Response::new(InvalidSessionRsp {}))
    }
}

pub async fn get(server: Arc<micro_service::Server>) -> SessionSvrServer<SessionSvrImpl> {
    return SessionSvrServer::new(SessionSvrImpl {
        black_map: Mutex::new(HashMap::new()),
        key: server.config().session_key.as_bytes().to_vec(),
    });
}
