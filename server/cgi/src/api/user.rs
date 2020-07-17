use actix_web::{
    connect, delete, get, head, options, patch, post, put, trace, web::Path, HttpResponse,
    Responder,
};

#[post("/login")]
pub async fn login() -> impl Responder {
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
