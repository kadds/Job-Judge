mod svr;
use tonic::transport::Server;
extern crate liblog;
extern crate tokio_postgres;
use log::error;

#[tokio::main(core_threads = 5, max_threads = 10)]
async fn main() {
    liblog::init_async_logger().unwrap();

    let addr = "0.0.0.0:50055".parse().unwrap();
    if let Err(err) = Server::builder()
        .add_service(svr::get().await)
        .serve(addr)
        .await
    {
        error!("startup server failed, err {}", err);
    }
}
