pub mod runner {
    tonic::include_proto!("runner");
}
pub mod user {
    pub mod rpc {
        tonic::include_proto!("user.rpc");
    }
    tonic::include_proto!("user");
}
pub mod session {
    pub mod rpc {
        tonic::include_proto!("session.rpc");
    }
}

use user::rpc::user_svr_client::UserSvrClient;
micro_service::define_client!(UserSvrClient, UserSvrCli, "usersvr");

use session::rpc::session_svr_client::SessionSvrClient;
micro_service::define_client!(SessionSvrClient, SessionSvrCli, "sessionsvr");
