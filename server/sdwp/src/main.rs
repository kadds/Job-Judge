#![feature(async_closure)]
mod grpc;
mod middleware;
mod router;
mod token;
use actix_web::{http::HeaderName, middleware::Logger, web, App, HttpServer};
use log::*;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AppData {
    config: Arc<micro_service::cfg::MicroServiceConfig>,
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let config = micro_service::cfg::init_from_env().unwrap();
    let app_data = AppData { config };
    let port: u16 = app_data.config.meta.bind_port;

    info!("bind at 0.0.0.0:{}", port);

    HttpServer::new(move || {
        let cors = actix_cors::Cors::default()
            .allow_any_origin()
            .allow_any_header()
            .allow_any_method()
            .expose_headers([HeaderName::from_static("cost")])
            .max_age(3600);
        App::new()
            .app_data(web::PayloadConfig::new(1024 * 1024 * 100))
            .app_data(web::Data::new(app_data.clone()))
            .wrap(cors)
            .wrap(middleware::RequestMetrics::new())
            .wrap(Logger::default())
            .service(actix_files::Files::new("/static", "./static").prefer_utf8(true))
            .wrap(middleware::Auth::new())
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/service")
                            .service(router::service::list)
                            .service(router::service::rpc_detail)
                            .service(router::service::list_rpc)
                            .service(router::service::invoke),
                    )
                    .service(web::scope("/user").service(router::user::login)),
            )
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await?;

    info!("exit...");
    Ok(())
}
