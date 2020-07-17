extern crate actix_rt;
extern crate actix_web;
extern crate liblog;

mod api;
use actix_web::{web, App, HttpServer};

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    liblog::init_async_logger().unwrap();

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
