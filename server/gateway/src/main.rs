use log::*;

mod api;
mod middleware;
mod rpc;
use actix_web::{web, App, HttpServer};
use micro_service::register_module_with_random;
use micro_service::service::MicroService;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

pub static mut MS: Option<Arc<micro_service::service::MicroService>> = None;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let config = micro_service::cfg::init_from_env().unwrap();
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), config.bind_port);

    info!("init service bind at 0.0.0.0:{}", config.bind_port);

    let ms = MicroService::init(config).await.unwrap();

    unsafe {
        MS = Some(ms.clone());
    }
    register_module_with_random!(ms.clone(), "usersvr");
    let mut stop_rx = ms.service_signal();

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
            .service(web::scope("/comm").service(api::comm::ping))
    })
    .bind(addr)
    .unwrap()
    .disable_signals();

    let running_server = server.run();

    let ret = tokio::select! {
        ret = running_server => {
            ret
        }
        Ok(_) = stop_rx.changed() => {
            tokio::time::sleep(std::time::Duration::from_millis(800)).await;
            actix_rt::System::current().stop();
            Ok(())
        }
    };
    info!("main gateway loop is stopped");
    ret
}
