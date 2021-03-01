use actix_web::{get, HttpResponse, Responder};
use log::*;

#[get("/ping")]
pub async fn ping(ping: String) -> impl Responder {
    debug!("request /ping content: {}", ping);
    HttpResponse::Ok().body(ping)
}
