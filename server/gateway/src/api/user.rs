use crate::rpc::*;
use crate::util::build_fail_response;
use crate::AppData;
use actix_web::{get, post, put, web, HttpResponse, Responder};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
pub struct LoginForm {
    #[serde(default)]
    username: String,
    password: String,
    #[serde(default)]
    email: String,
}

#[post("/login")]
pub async fn login(data: web::Data<AppData>, form: web::Json<LoginForm>) -> impl Responder {
    let server = data.server.clone();
    let mut client = UserSvrClient::new(server.channel("usersvr").await);
    let request = user::rpc::ValidUserReq {
        username: form.username.to_owned(),
        password: form.password.to_owned(),
        email: form.email.to_owned(),
    };
    let result = client.valid_user(request).await;
    match result {
        Ok(res) => {
            let res: user::rpc::ValidUserRsp = res.into_inner();
            let token = res.vid;
            let json = json!({ "token": token });
            HttpResponse::Ok().json(&json)
        }
        Err(e) => build_fail_response(e),
    }
}

#[post("/logout")]
pub async fn logout() -> impl Responder {
    HttpResponse::Ok()
}

#[get("/info")]
pub async fn info() -> impl Responder {
    HttpResponse::Ok()
}

#[put("/info")]
pub async fn update_info() -> impl Responder {
    HttpResponse::Ok()
}

#[put("/register")]
pub async fn register() -> impl Responder {
    HttpResponse::Ok()
}
