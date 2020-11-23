mod svr;
#[macro_use]
extern crate micro_service;
extern crate tokio_postgres;
use std::env::var;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use micro_service::service::MicroService;
use micro_service::cfg;
use tonic::transport::Server;
use tokio;
use tokio::sync::watch;

async fn wait_stop_signal(mut signal: watch::Receiver<u64>) -> () {
    debug!("start");
    signal.recv().await;
    debug!("stop");
    ()
}

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

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server_name = var("SERVER_NAME").unwrap();
    let host = var("HOST_IP").unwrap();

    early_log_info!(server_name, "init service info: module {} server {} bind at {}:{}", module, server_name, host, port);

    let ms = MicroService::init(
        config.etcd,
        module.to_string(),
        server_name.to_string(),
        format!("{}:{}", host, port).parse().unwrap(),
        3,
    )
    .await
    .unwrap();

    let mut stop_rx = ms.get_stop_signal();
    let service = svr::get(&config.database.url, ms).await;
    if let Err(err) = Server::builder()
        .add_service(service)
        .serve_with_incoming_shutdown(listener, wait_stop_signal(stop_rx)).await
    {
        early_log_error!(server_name, "startup server failed, err {}", err);
    }
}
