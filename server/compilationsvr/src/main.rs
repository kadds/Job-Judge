mod svr;
use etcd_rs::*;
use tokio::prelude::*;
use tonic::transport::Server;
extern crate liblog;

#[tokio::main]
async fn main() {
    liblog::init_async_logger().unwrap();
    let addr = "0.0.0.0:50051".parse().unwrap();

    Server::builder()
        .add_service(svr::get())
        .serve(addr)
        .await
        .unwrap();
}
