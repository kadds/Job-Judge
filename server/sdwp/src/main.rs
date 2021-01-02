extern crate actix_rt;
extern crate actix_web;
#[macro_use]
extern crate micro_service;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

mod middleware;
mod router;
mod token;

use actix_web::{web, App, HttpServer};
// use micro_service::service::MicroService;
use std::env::var;
use std::sync::Arc;
use actix_web::middleware::Logger;

pub static mut MS: Option<Arc<micro_service::service::MicroService>> = None;

#[derive(Debug, Clone)]
pub struct AppData {
    username: String,
    password: String,
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let username = var("SDWP_USER").unwrap_or_else(|_| "admin".to_owned());
    let password = var("SDWP_PASSWORD").unwrap_or_else(|_| "12345678".to_owned());

    let app_data = AppData{
        username, password
    };

    let port: u16 = var("PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(6550);
    info!("bind at 0.0.0.0:{}", port);

    let _ = HttpServer::new(move || {
        App::new()
            .data(app_data.clone())
            .wrap(Logger::default())
            .wrap(middleware::Auth::new())
            .service(
                web::scope("/service")
                    .service(router::service::list)
                    .service(router::service::get_rpc_info)
                    .service(router::service::get_rpcs)
            )
            .service(
                web::scope("/user")
                    .service(router::user::login)
            )
            .service(
                web::scope("/request")
                    .service(router::request::request)
            )
    })
        .bind(format!("0.0.0.0:{}", port))?
        .run().await;

    info!("exit...");
    Ok(())
}

