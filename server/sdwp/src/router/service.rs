use crate::{grpc, AppData};
use actix_web::{get, post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
pub struct ListRpcRequest {
    pub module: String,
    pub service: String,
    pub instance: String,
}

#[derive(Serialize, Deserialize)]
struct ListRpcResult {
    pub rpcs: Vec<String>,
    pub service: String,
    pub services: Vec<String>,
    pub instance: String,
    pub instances: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct RpcDetailRequest {
    pub module: String,
    pub service: String,
    pub instance: String,
    pub method: String,
}

#[derive(Serialize, Deserialize)]
pub struct RpcDetailResult {
    rpc: grpc::reflection::RpcInfo,
}

#[derive(Serialize, Deserialize)]
pub struct ListResult {
    pub list: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct InvokeRequest {
    pub body: Value,
    pub module: String,
    pub service: String,
    pub instance: String,
    pub method: String,
}

#[get("/list")]
pub async fn list(data: web::Data<AppData>) -> impl Responder {
    HttpResponse::Ok().json(&ListResult {
        list: data.config.modules.clone(),
    })
}

#[get("/rpcs")]
pub async fn list_rpc(data: web::Data<AppData>, req: web::Json<ListRpcRequest>) -> impl Responder {
    let f = async || -> grpc::GrpcResult<ListRpcResult> {
        let ctx = grpc::RequestContext::new(&data.config, &req.module, &req.instance).await?;
        let (service, services) = ctx.pick_services_or(&req.service).await?;
        let rpcs = ctx.list_rpcs(&service).await?;
        let (instance, instances) = ctx.instance();
        Ok(ListRpcResult {
            rpcs,
            instance,
            instances,
            service,
            services,
        })
    };
    match f().await {
        Ok(rsp) => HttpResponse::Ok().json(&rsp),
        Err(err) => HttpResponse::InternalServerError().body(format!("{}", err)),
    }
}

#[get("/rpc")]
pub async fn rpc_detail(
    data: web::Data<AppData>,
    req: web::Json<RpcDetailRequest>,
) -> impl Responder {
    let f = async || -> grpc::GrpcResult<RpcDetailResult> {
        let ctx = grpc::RequestContext::new(&data.config, &req.module, &req.instance).await?;
        if req.service.is_empty() {
            return Err(grpc::GrpcError::InvalidParameters);
        }
        let rpc = ctx.rpc_info(&req.service, &req.method).await?;
        Ok(RpcDetailResult { rpc })
    };
    match f().await {
        Ok(rsp) => HttpResponse::Ok().json(&rsp),
        Err(err) => HttpResponse::InternalServerError().body(format!("{}", err)),
    }
}

#[post("/invoke")]
pub async fn invoke(data: web::Data<AppData>, req: web::Json<InvokeRequest>) -> impl Responder {
    let f = async || -> grpc::GrpcResult<Value> {
        let ctx = grpc::RequestContext::new(&data.config, &req.module, &req.instance).await?;
        if req.service.is_empty() {
            return Err(grpc::GrpcError::InvalidParameters);
        }
        let req_body = req.body.clone();
        let resp = ctx.invoke(&req.service, &req.method, req_body).await?;
        Ok(resp)
    };
    match f().await {
        Ok(rsp) => HttpResponse::Ok().json(&rsp),
        Err(err) => HttpResponse::InternalServerError().body(format!("{}", err)),
    }
}
