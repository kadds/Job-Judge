use actix_web::{
    connect, delete, get, head, options, patch, post, put, trace, web::Path, HttpResponse,
    Responder
};
use crate::rpc::*;
use crate::MS;


#[post("/login")]
pub async fn login() -> impl Responder {
    let uin: u64 = 0;
    let client = unsafe {
        UserSvrClient::new(match MS.clone().unwrap().get_channel("usersvr", uin, 0).await {
            Some(v) => v,
            None => {
                return HttpResponse::Ok();
            }
        })
    };
    
    HttpResponse::Ok()
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
