extern crate hex;
extern crate micro_service;
extern crate sha2;
#[macro_use]
extern crate log;

use micro_service::service::MicroService;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio;
use tokio::sync::watch;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;
mod svr;
mod table;

async fn wait_stop_signal(mut signal: watch::Receiver<u64>) {
    let _ = signal.changed().await;
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    env_logger::init();

    let config = micro_service::cfg::init_from_env().unwrap();
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), config.bind_port);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    info!("init service bind at 0.0.0.0:{}", config.bind_port);

    let ms = MicroService::init(config).await.unwrap();

    let stop_rx = ms.service_signal();
    let service = svr::get(ms).await;
    if let Err(err) = Server::builder()
        .add_service(service)
        .serve_with_incoming_shutdown(TcpListenerStream::new(listener), wait_stop_signal(stop_rx))
        .await
    {
        error!("startup server failed, error {}", err);
    }
}
