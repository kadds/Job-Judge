mod svr;
use tokio::prelude::*;
use tonic::transport::Server;
extern crate liblog;
use log::error;

#[tokio::main(core_threads = 1, max_threads = 1)]
async fn main() {
    liblog::init_async_logger().unwrap();
    let addr = "0.0.0.0:50050".parse().unwrap();
    if let Err(err) = Server::builder().add_service(svr::get()).serve(addr).await {
        log::error!("startup server failed, err {}", err);
    }
}
