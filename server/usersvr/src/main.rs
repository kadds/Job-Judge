mod svr;
#[macro_use]
extern crate micro_service;
extern crate tokio_postgres;
use std::env::var;
use std::time::Duration;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tonic::transport::Server;
use micro_service::service::MicroService;
use micro_service::cfg;

#[tokio::main(core_threads = 4, max_threads = 10)]
async fn main() {
    let module = "usersvr";
    let config = tokio::fs::read("./config.toml").await.unwrap();
    let config: micro_service::cfg::MicroServiceCommConfig =
        toml::from_slice(&config).unwrap();

    match config.comm.log_type {
        cfg::LogType::Tcp => {
            micro_service::init_tcp_logger(format!("{}:{}", config.comm.log_host, config.comm.log_port));
        },
        cfg::LogType::Console => {
            micro_service::init_console_logger();
        }
    }

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

    let mut listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let server_name = var("SERVER_NAME").unwrap();
    let host = var("HOST_IP").unwrap();
    info!("init service info: module {} server {} bind at {}:{}", module, server_name, host, port);
    let ms = MicroService::init(
        config.etcd,
        module.to_string(),
        server_name,
        format!(
            "{}:{}",
            host,
            port,
        ),
        Duration::from_secs(60 * 2),
        3,
    )
    .await
    .unwrap();

    if let Err(err) = Server::builder()
        .add_service(svr::get(ms).await)
        .serve_with_incoming(listener)
        .await
    {
        error!("startup server failed, err {}", err);
    }
}
