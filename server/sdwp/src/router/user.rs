use super::super::{token, AppData};
use actix_web::{post, web, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

#[post("/login")]
pub async fn login(data: web::Data<Arc<AppData>>, form: web::Json<LoginForm>) -> impl Responder {
    if form.username == data.config.user.username && form.password == data.config.user.password {
        let (token, end) = token::create().await;
        HttpResponse::Ok().json(json!({"token": token, "end": end}))
    } else {
        HttpResponse::Ok().json(json!({"err": "password error"}))
    }
}
