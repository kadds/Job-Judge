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
    if let Some(u) = data.config.comm.username.as_ref() {
        let passwd = data.config.comm.password.as_ref().map_or("", |v| v.as_str());
        if form.username == *u && form.password == passwd {
            let (token, end) = token::create().await;
            HttpResponse::Ok().json(&json!({"token": token, "end": end}))
        } else {
            HttpResponse::Ok().json(&json!({"err": "password error"}))
        }
    } else {
        HttpResponse::Ok().json(&json!({}))
    }
}
