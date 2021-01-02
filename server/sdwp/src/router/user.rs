
use actix_web::{
    post, HttpResponse, Responder, web
};
use serde_json::json;
use super::super::{token, AppData};

#[derive(Deserialize)]
pub struct LoginForm {
    username: String,
    password: String,
}

#[post("/login")]
pub async fn login(data: web::Data<AppData>, form: web::Json<LoginForm>) -> impl Responder {
    if form.username == data.username && form.password == data.password {
        let (token, end) = token::create().await;
        HttpResponse::Ok().json(json!({"token": token, "end": end}))
    }
    else {
        HttpResponse::Ok().json(json!({"err": "password error"}))
    }
}