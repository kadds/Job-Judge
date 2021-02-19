#[macro_use]
extern crate micro_service;
extern crate hex;
extern crate sha2;
use micro_service::cfg;
use micro_service::service::{MicroService, ServiceLevel};
use std::env::var;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tokio;
use tokio::sync::watch;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::Server;
mod svr;
mod table;

async fn wait_stop_signal(mut signal: watch::Receiver<u64>) -> () {
    let _ = signal.changed().await;
    ()
}

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    let module = "usersvr";
    let config = tokio::fs::read("./config.toml").await.unwrap();
    let config: micro_service::cfg::MicroServiceCommConfig = toml::from_slice(&config).unwrap();

    match config.comm.log_type {
        cfg::LogType::Tcp => {
            micro_service::init_tcp_logger(format!(
                "{}:{}",
                config.comm.log_host, config.comm.log_port
            ));
        }
        cfg::LogType::Console => {
            micro_service::init_console_logger();
        }
    }

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let port = listener.local_addr().unwrap().port();

    let server_name = var("SERVER_NAME").unwrap();
    let host = var("HOST_IP").unwrap();
    let flag: String = var("ENV_FLAG").unwrap_or_default();
    let service_level = match flag.as_str() {
        "1" => ServiceLevel::Test,
        _ => ServiceLevel::Prod,
    };

    early_log_info!(
        server_name,
        "init service info: module {} server {} bind at {}:{}",
        module,
        server_name,
        host,
        port
    );

    let ms = MicroService::init(
        config.etcd,
        module.to_string(),
        server_name.to_string(),
        format!("{}:{}", host, port).parse().unwrap(),
        3,
        service_level,
    )
    .await
    .unwrap();

    let stop_rx = ms.service_signal();
    let service = svr::get(&config.database.url, ms).await;
    if let Err(err) = Server::builder()
        .add_service(service)
        .serve_with_incoming_shutdown(TcpListenerStream::new(listener), wait_stop_signal(stop_rx))
        .await
    {
        early_log_error!(server_name, "startup server failed, err {}", err);
    }
}
