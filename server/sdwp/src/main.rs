mod grpc;
mod middleware;
mod router;
mod token;
use actix_web::{
    http::header::HeaderName,
    middleware::{Compress, Logger},
    web, App, HttpServer,
};
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
        let cors = actix_cors::Cors::permissive()
            .expose_headers([HeaderName::from_static("cost"), HeaderName::from_static("token")])
            .max_age(3600);
        App::new()
            .app_data(web::PayloadConfig::new(1024 * 1024 * 100))
            .app_data(web::Data::new(app_data.clone()))
            .wrap(cors)
            .wrap(Compress::default())
            .wrap(middleware::RequestMetrics::new())
            .wrap(Logger::default())
            .service(
                web::scope("/api")
                    .wrap(middleware::Auth::new())
                    .service(
                        web::scope("/service")
                            .service(router::service::list)
                            .service(router::service::rpc_detail)
                            .service(router::service::list_rpc)
                            .service(router::service::invoke),
                    )
                    .service(web::scope("/user").service(router::user::login)),
            )
            .service(
                actix_files::Files::new("/", "./web/dist")
                    .prefer_utf8(true)
                    .index_file("index.html"),
            )
    })
    .bind(format!("0.0.0.0:{}", port))?
    .run()
    .await?;

    info!("exit...");
    Ok(())
}
