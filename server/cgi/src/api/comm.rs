use actix_web::{
    connect, delete, get, head, options, patch, post, put, trace, web::Json, web::Path,
    HttpResponse, Responder
};


#[get("/ping")]
pub async fn ping(ping: String) -> impl Responder {
    HttpResponse::Ok().body(ping)
}