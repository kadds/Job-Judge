use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{transport::Server, Request, Response, Status};

mod test {
    pub mod rpc {
        tonic::include_proto!("test.rpc");
    }
}
pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("descriptor");
use test::rpc::test_svr_server::{TestSvr, TestSvrServer};
use test::rpc::*;

pub struct TestSvrImpl {}

#[tonic::async_trait]
impl TestSvr for TestSvrImpl {
    async fn echo(&self, _request: Request<EchoReq>) -> Result<Response<EchoRsp>, Status> {
        log::info!("echo request");
        Ok(Response::new(EchoRsp {}))
    }

    async fn message_echo(&self, request: Request<MessageEchoReq>) -> Result<Response<MessageEchoRsp>, Status> {
        let req = request.into_inner();
        log::info!("message echo request {:?}", req);
        Ok(Response::new(MessageEchoRsp { pack: req.pack }))
    }
}

async fn daemon(addr: SocketAddr, server: Arc<micro_service::Server>) {
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    loop {
        let channel = server.channel_static(addr).await;
        let mut cli = test::rpc::test_svr_client::TestSvrClient::new(channel);
        let req = EchoReq {};

        let rsp = cli.echo(Request::new(req.clone())).await;
        if let Err(e) = rsp {
            log::error!("echo error {}", e);
        }
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

pub async fn get(server: Arc<micro_service::Server>, listener: TcpListener) {
    let test_svr = TestSvrServer::new(TestSvrImpl {});

    let reflection_svr = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build()
        .unwrap();

    tokio::spawn(daemon(listener.local_addr().unwrap(), server.clone()));

    Server::builder()
        .add_service(test_svr)
        .add_service(reflection_svr)
        .serve_with_incoming_shutdown(TcpListenerStream::new(listener), server.wait_stop_signal())
        .await
        .expect("start server fail");
}
