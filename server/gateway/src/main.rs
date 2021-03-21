use log::*;
mod api;
mod middleware;
mod util;
use actix_web::middleware::Logger;
mod rpc;
use actix_web::{web, App, HttpServer};
use micro_service::Server;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

pub struct AppData {
    server: Arc<micro_service::Server>,
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let config = micro_service::cfg::init_from_env().unwrap();
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), config.bind_port);

    info!("init service bind at 0.0.0.0:{}", config.bind_port);

    let ms = Server::new(config);
    let mut rx = ms.server_signal();

    let server = HttpServer::new(move || {
        let app_data = AppData { server: ms.clone() };
        App::new()
            .data(app_data)
            .wrap(Logger::new("%a  %t-%D %b"))
            .wrap(middleware::Auth::new())
            .service(
                web::scope("/user")
                    .service(api::user::login)
                    .service(api::user::logout)
                    .service(api::user::info)
                    .service(api::user::register),
            )
            .service(web::scope("/comm").service(api::comm::ping))
    })
    .bind(addr)
    .unwrap()
    .disable_signals();

    let server = server.run();

    let ret = tokio::select! {
        ret = server => {
            ret
        }
        Ok(_) = rx.changed() => {
            tokio::time::sleep(std::time::Duration::from_millis(800)).await;
            actix_rt::System::current().stop();
            Ok(())
        }
    };
    info!("main gateway loop is stopped");
    ret
}
