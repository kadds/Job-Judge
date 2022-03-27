use std::net::{IpAddr, Ipv4Addr, SocketAddr};
mod config;
mod daemon;
mod mgr;
mod svr;
use log::*;

#[tokio::main]
async fn main() {
    env_logger::init();

    let config = micro_service::cfg::init_from_env().unwrap();
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), config.meta.bind_port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    info!("init service bind at 0.0.0.0:{}", config.meta.bind_port);

    let ms = micro_service::Server::new(config);
    svr::get(ms, listener).await;
}
