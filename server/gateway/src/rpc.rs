pub mod runner {
    tonic::include_proto!("runner");
}
pub mod user {
    pub mod rpc {
        tonic::include_proto!("user.rpc");
    }
    tonic::include_proto!("user");
}
use user::rpc::user_svr_client::UserSvrClient;
micro_service::define_client!(UserSvrClient, UserSvrCli, "usersvr");
