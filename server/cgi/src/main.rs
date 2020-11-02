extern crate actix_rt;
extern crate actix_web;
#[macro_use]
extern crate micro_service;

mod api;
use actix_web::{web, App, HttpServer};
use std::env::var;
use std::sync::Arc;
use std::time::Duration;

pub static mut MS: Option<Arc<micro_service::service::MicroService>> = None;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let config = tokio::fs::read("./config.toml").await.unwrap();
    let config: micro_service::cfg::MicroServiceCommConfig =
        serde_json::from_slice(&config).unwrap();

    micro_service::log::init_tcp_logger(format!("{}:{}", config.log_host, config.log_port));

    let ms = micro_service::service::MicroService::init(
        config.etcd,
        var("MODULE").unwrap(),
        var("SERVER_NAME").unwrap(),
        format!("{}:8080", var("HOST_IP").unwrap()),
        Duration::from_secs(60 * 2),
        3,
    )
    .await
    .unwrap();

    unsafe {
        MS = Some(ms);
    }

    HttpServer::new(|| {
        App::new()
            .service(
                web::scope("/user")
                    .service(api::user::login)
                    .service(api::user::logout)
                    .service(api::user::info)
                    .service(api::user::update_info)
                    .service(api::user::register),
            )
            .service(
                web::scope("/judge")
                    .service(api::judge::commit)
                    .service(api::judge::lang_list),
            )
            .service(web::scope("/problem").service(api::problem::problem))
            .service(web::scope("/run").service(api::run::run_source))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
