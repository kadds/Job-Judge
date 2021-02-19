extern crate actix_rt;
extern crate actix_web;
#[macro_use]
extern crate log;
#[macro_use]
extern crate lazy_static;

mod cfg;
mod middleware;
mod router;
mod token;
use std::sync::Arc;

use actix_web::{middleware::Logger, web, App, HttpServer};

#[derive(Debug, Clone)]
pub struct AppData {
    config: cfg::Config,
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let config = tokio::fs::read("./config.toml").await.unwrap();
    let config: cfg::Config = toml::from_slice(&config).unwrap();

    let port: u16 = config.comm.port;
    let app_data = Arc::new(AppData { config });

    info!("bind at 0.0.0.0:{}", port);

    let _ = HttpServer::new(move || {
        App::new()
            .data(app_data.clone())
            .app_data(app_data.clone())
            .wrap(Logger::default())
            .wrap(middleware::Auth::new())
            .service(
                web::scope("/service")
                    .service(router::service::list)
                    .service(router::service::get_rpc_info)
                    .service(router::service::get_rpcs)
                    .service(router::service::request),
            )
            .service(web::scope("/user").service(router::user::login))
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await;

    info!("exit...");
    Ok(())
}
