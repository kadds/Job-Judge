mod srv;
use etcd_rs::*;
use tokio::prelude::*;
use tonic::transport::Server;
extern crate liblog;

#[tokio::main(core_threads = 1, max_threads = 1)]
async fn main() {
    liblog::init_async_logger().unwrap();
    let addr = "0.0.0.0:50051".parse().unwrap();

    Server::builder()
        .add_service(srv::get())
        .serve(addr)
        .await
        .unwrap();
}
