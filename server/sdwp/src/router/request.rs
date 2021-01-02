use actix_web::{
    post, HttpResponse, Responder, web
};
use serde_json::Value;


#[post("")]
pub async fn request(req: web::Json<Value>) -> impl Responder {
    HttpResponse::Ok().json({})
}