use actix_web::{
    get, HttpResponse, Responder
};


#[get("/list")]
pub async fn list() -> impl Responder {
    HttpResponse::Ok().json({})
}

#[get("/rpcs")]
pub async fn get_rpcs(_service_name: String) -> impl Responder {
    HttpResponse::Ok().json({})
}

#[get("/rpc")]
pub async fn get_rpc_info(_service_name: String, _rpc_name: String) -> impl Responder {
    HttpResponse::Ok().json({})
}

