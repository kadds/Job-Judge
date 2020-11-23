extern crate actix_rt;
extern crate prost;
extern crate actix_web;
extern crate futures;
#[macro_use]
extern crate micro_service;

mod rpc;
mod api;
mod middleware;
use actix_web::{web, App, HttpServer};
use micro_service::service::MicroService;
use micro_service::cfg;
use std::env::var;
use std::sync::Arc;

pub static mut MS: Option<Arc<micro_service::service::MicroService>> = None;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let module = "cgi";
    let port: u16 = 8080;

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

    let server_name = var("SERVER_NAME").unwrap();
    let host = var("HOST_IP").unwrap();

    early_log_info!(server_name, "init service info: module {} server {} bind at {}:{}", module, server_name, host, port);
    let ms = MicroService::init(
        config.etcd,
        module.to_string(),
        server_name.clone(),
        format!("{}:{}", host, port).parse().unwrap(),
        3,
    )
    .await
    .unwrap();
    unsafe {
        MS = Some(ms.clone());
    }
    register_module_with_random!(ms.clone(), "usersvr");
    let mut stop_rx = ms.get_stop_signal();

    let server = HttpServer::new(move || {
        App::new()
            .wrap(middleware::logger::Logger::new(ms.clone()))
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
            .service(
                web::scope("/comm")
                    .service(api::comm::ping)
            )
            .service(web::scope("/problem").service(api::problem::problem))
            .service(web::scope("/run").service(api::run::run_source))
    })
    .bind(format!("0.0.0.0:{}", port)).unwrap()
    .disable_signals();

    let running_server = server.run();

    let ret = 
    tokio::select! {
        ret = running_server => {
            ret
        }
        Some(_) = stop_rx.recv() => {
            tokio::time::delay_for(std::time::Duration::from_millis(800)).await;
            actix_rt::System::current().stop();
            Ok(())
        }
    };
    early_log_info!(server_name, "main cgi loop is stopped");
    ret
}
