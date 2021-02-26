pub mod runner {
    tonic::include_proto!("runner");
}
pub mod user {
    pub mod rpc {
        tonic::include_proto!("user.rpc");
    }
    tonic::include_proto!("user");
}

pub use user::rpc::user_svr_client::UserSvrClient;
