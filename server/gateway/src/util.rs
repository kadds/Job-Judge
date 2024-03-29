use actix_http::{body::BoxBody, Response, ResponseBuilder, StatusCode};
use log::info;
use std::sync::Arc;
use tonic::{Code, Status};

pub fn build_fail_response(status: Status) -> Response<BoxBody> {
    let msg = status.message().to_owned();
    let code = status.code();
    let code = match code {
        Code::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        Code::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        Code::Unauthenticated => StatusCode::UNAUTHORIZED,
        Code::NotFound => StatusCode::NOT_FOUND,
        Code::PermissionDenied => StatusCode::FORBIDDEN,
        Code::Unimplemented => StatusCode::NOT_IMPLEMENTED,
        Code::OutOfRange => StatusCode::RANGE_NOT_SATISFIABLE,
        Code::FailedPrecondition => StatusCode::PRECONDITION_FAILED,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    ResponseBuilder::new(code).body(msg).map_into_boxed_body()
}

pub async fn is_valid_token(server: Arc<micro_service::Server>, token: String) -> bool {
    use crate::rpc::session::rpc::*;
    use crate::rpc::SessionSvrCli;
    let mut cli: SessionSvrCli = server.client().await;
    let req = GetSessionReq { key: token };
    match cli.get_session(req).await {
        Ok(_) => true,
        Err(e) => {
            info!("{}", e.message());
            false
        }
    }
}
