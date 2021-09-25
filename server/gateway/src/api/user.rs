use super::Context;
use crate::rpc::*;
use actix_web::{
    get, post, put,
    web::{Json, Query},
    HttpRequest, HttpResponse, Responder,
};
use log::*;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

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
    uid: i64,
}

#[post("/login")]
pub async fn login(ctx: Context, form: Json<LoginForm>) -> impl Responder {
    let mut cli: UserSvrCli = ctx.server.clone().client().await;
    let req = user::rpc::ValidUserReq {
        username: form.username.to_owned(),
        password: form.password.to_owned(),
        email: form.email.to_owned(),
    };
    let res = check_rpc!(cli.valid_user(req).await);
    info!("user login with uid {}", res.id);
    let mut cli: SessionSvrCli = ctx.server.clone().client().await;
    let req = session::rpc::CreateSessionReq {
        timeout: 60 * 60 * 24,
        uid: res.id,
        comm_data: HashMap::new(),
    };
    let token = check_rpc!(cli.create_session(req).await).key;
    let json = json!({ "token": token, "uid": res.id });
    HttpResponse::Ok().json(&json).into()
}

#[post("/logout")]
pub async fn logout(ctx: Context, http: HttpRequest) -> impl Responder {
    let mut cli: SessionSvrCli = ctx.server.clone().client().await;
    let req = session::rpc::InvalidSessionReq {
        key: http.headers().get("TOKEN").unwrap().to_str().unwrap().to_owned(),
    };
    let _ = check_rpc!(cli.invalid_session(req).await);
    let json = json!({});
    HttpResponse::Ok().json(&json).into()
}

#[get("/info")]
pub async fn info(ctx: Context, form: Query<InfoQuery>) -> impl Responder {
    let mut cli: UserSvrCli = ctx.server.clone().client().await;
    let req = user::rpc::GetUserReq { id: form.uid };
    let res = check_rpc!(cli.get_user(req).await).userinfo;
    let res = match res {
        Some(v) => v,
        None => {
            let json = json!({"err": "not found any user"});
            return HttpResponse::Ok().json(&json).into();
        }
    };
    let json = json!({ "uid": res.id, "avatar": res.avatar, "nickname": res.nickname });
    HttpResponse::Ok().json(&json).into()
}

#[put("/register")]
pub async fn register(ctx: Context, form: Json<RegisterForm>) -> impl Responder {
    let mut cli: UserSvrCli = ctx.server.clone().client().await;
    let req = user::rpc::CreateUserReq {
        username: form.username.to_owned(),
        password: form.password.to_owned(),
    };
    let res = check_rpc!(cli.create_user(req).await);
    info!("user register with uid {}", res.id);
    let json = json!({"uid": res.id});
    HttpResponse::Ok().json(&json).into()
}
