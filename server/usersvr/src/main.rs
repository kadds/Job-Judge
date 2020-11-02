mod svr;
#[macro_use]
extern crate micro_service;
extern crate tokio_postgres;
use std::env::var;
use std::time::Duration;
use tonic::transport::Server;

#[tokio::main(core_threads = 4, max_threads = 10)]
async fn main() {
    let config = tokio::fs::read("./config.toml").await.unwrap();
    let config: micro_service::cfg::MicroServiceCommConfig =
        serde_json::from_slice(&config).unwrap();

    micro_service::log::init_tcp_logger(format!("{}:{}", config.log_host, config.log_port));

    let addr = "0.0.0.0:0".parse().unwrap();
    let mut listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let ms = micro_service::service::MicroService::init(
        config.etcd,
        var("MODULE").unwrap(),
        var("SERVER_NAME").unwrap(),
        format!(
            "{}:{}",
            var("HOST_IP").unwrap(),
            listener.local_addr().unwrap().port()
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
