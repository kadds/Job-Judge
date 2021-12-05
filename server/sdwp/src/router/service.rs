use crate::{grpc, AppData};
use actix_web::{get, post, web, HttpResponse, Responder};
use micro_service::cfg::DiscoverConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
pub struct ListRpcRequest {
    pub module: String,
    pub service: Option<String>,
    pub instance: Option<String>,
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

async fn list_inner(discover: &DiscoverConfig) -> grpc::GrpcResult<ListResult> {
    let ret = grpc::list_modules(discover)
        .await?
        .into_iter()
        .filter(|name| name != "sdwp" && name != "gateway")
        .collect();
    Ok(ListResult { list: ret })
}

#[get("/list")]
pub async fn list(data: web::Data<AppData>) -> impl Responder {
    match list_inner(&data.config.discover).await {
        Ok(rsp) => HttpResponse::Ok().json(&rsp),
        Err(err) => HttpResponse::InternalServerError().body(format!("{}", err)),
    }
}

async fn list_rpc_inner(discover: &DiscoverConfig, req: web::Query<ListRpcRequest>) -> grpc::GrpcResult<ListRpcResult> {
    let instance = req.instance.clone().unwrap_or_default();
    let service = req.service.clone().unwrap_or_default();

    let ctx = grpc::RequestContext::new(discover, &req.module, &instance, true).await?;
    let (service, services) = ctx.pick_services_or(&service).await?;
    let rpcs = ctx.list_rpcs(&service).await?;
    let (instance, instances) = ctx.instance();
    Ok(ListRpcResult {
        rpcs,
        instance,
        instances,
        service,
        services,
    })
}

#[get("/rpcs")]
pub async fn list_rpc(data: web::Data<AppData>, req: web::Query<ListRpcRequest>) -> impl Responder {
    match list_rpc_inner(&data.config.discover, req).await {
        Ok(rsp) => HttpResponse::Ok().json(&rsp),
        Err(err) => HttpResponse::InternalServerError().body(format!("{}", err)),
    }
}

async fn rpc_detail_inner(
    discover: &DiscoverConfig,
    req: web::Query<RpcDetailRequest>,
) -> grpc::GrpcResult<RpcDetailResult> {
    let ctx = grpc::RequestContext::new(discover, &req.module, &req.instance, true).await?;
    if req.service.is_empty() {
        return Err(grpc::GrpcError::InvalidParameters);
    }
    let rpc = ctx.rpc_info(&req.service, &req.method).await?;
    Ok(RpcDetailResult { rpc })
}

#[get("/rpc")]
pub async fn rpc_detail(data: web::Data<AppData>, req: web::Query<RpcDetailRequest>) -> impl Responder {
    match rpc_detail_inner(&data.config.discover, req).await {
        Ok(rsp) => HttpResponse::Ok().json(&rsp),
        Err(err) => HttpResponse::InternalServerError().body(format!("{}", err)),
    }
}

async fn invoke_inner(discover: &DiscoverConfig, req: web::Json<InvokeRequest>) -> grpc::GrpcResult<Value> {
    let ctx = grpc::RequestContext::new(discover, &req.module, &req.instance, false).await?;
    if req.service.is_empty() {
        return Err(grpc::GrpcError::InvalidParameters);
    }
    let req_body = req.body.clone();
    let resp = ctx.invoke(&req.service, &req.method, req_body).await?;
    Ok(resp)
}

#[post("/invoke")]
pub async fn invoke(data: web::Data<AppData>, req: web::Json<InvokeRequest>) -> impl Responder {
    match invoke_inner(&data.config.discover, req).await {
        Ok(rsp) => HttpResponse::Ok().json(&rsp),
        Err(err) => HttpResponse::InternalServerError().body(format!("{}", err)),
    }
}
