use super::Context;
use crate::rpc::*;
use actix_web::{
    get, post, put,
    web::{Json, Query},
    HttpResponse, Responder,
};
use log::*;
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
#[derive(Deserialize)]
pub struct RegisterForm {
    #[serde(default)]
    username: String,
    password: String,
    #[serde(default)]
    _email: String,
}

#[derive(Deserialize)]
pub struct InfoQuery {
    vid: u64,
}

#[post("/login")]
pub async fn login(ctx: Context, form: Json<LoginForm>) -> impl Responder {
    let mut client: UserSvrCli = ctx.server.clone().client().await;
    let request = user::rpc::ValidUserReq {
        username: form.username.to_owned(),
        password: form.password.to_owned(),
        email: form.email.to_owned(),
    };
    let result = check_rpc!(client.valid_user(request).await);
    info!("user login with vid {}", result.vid);
    let token = result.vid;
    let json = json!({ "token": token, "vid": result.vid });
    HttpResponse::Ok().json(&json)
}

#[post("/logout")]
pub async fn logout() -> impl Responder {
    let json = json!({});
    HttpResponse::Ok().json(&json)
}

#[get("/info")]
pub async fn info(ctx: Context, form: Query<InfoQuery>) -> impl Responder {
    let mut client: UserSvrCli = ctx.server.clone().client().await;
    let request = user::rpc::GetUserReq { vid: form.vid };
    let result = check_rpc!(client.get_user(request).await).userinfo;
    let result = match result {
        Some(v) => v,
        None => {
            let json = json!({"err": "not found any user"});
            return HttpResponse::Ok().json(&json);
        }
    };
    let json = json!({ "vid": result.vid, "avatar": result.avatar, "nickname": result.nickname });
    HttpResponse::Ok().json(&json)
}

#[put("/register")]
pub async fn register(ctx: Context, form: Json<RegisterForm>) -> impl Responder {
    let mut client: UserSvrCli = ctx.server.clone().client().await;
    let request = user::rpc::CreateUserReq {
        username: form.username.to_owned(),
        password: form.password.to_owned(),
    };
    let result = check_rpc!(client.create_user(request).await);
    info!("user register with vid {}", result.vid);
    let json = json!({});
    HttpResponse::Ok().json(&json)
}
