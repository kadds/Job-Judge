use actix_web::{get, post, web, HttpResponse, Responder};
// use anyhow::Result;
use super::super::AppData;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FetchServiceError {
    #[error("string format error")]
    Format(#[from] std::string::FromUtf8Error),
    // #[error("broken data")]
    // DataUnException,
    #[error("not found")]
    NotFound,
    // #[error("unknown data store error")]
    // Unknown,
}
type FetchServiceResult<T> = std::result::Result<T, FetchServiceError>;

#[derive(Serialize, Deserialize)]
pub struct ServicePair {
    pub module_name: String,
    pub server_name: String,
}
#[derive(Serialize, Deserialize)]
pub struct RpcPair {
    pub module_name: String,
    pub server_name: String,
    pub rpc_name: String,
}

impl From<ServicePair> for RpcPair {
    fn from(pair: ServicePair) -> Self {
        RpcPair {
            module_name: pair.module_name,
            server_name: pair.server_name,
            rpc_name: "".to_owned(),
        }
    }
}

struct ServiceDetail {
    pub name: String,
    pub address: String,
}

#[derive(Serialize, Deserialize)]
struct ModuleServices {
    pub name: String,
    pub services: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct ListResult {
    pub list: Vec<ModuleServices>,
}

struct RpcMeta {
    pub name: String,
}

struct ServiceMeta {
    pub rpcs: Vec<RpcMeta>,
}

async fn get_service_meta(_address: &str) -> anyhow::Result<ServiceMeta> {
    Err(anyhow::Error::msg("fail"))
}

async fn get_servers_info(
    _config: &crate::cfg::Config,
) -> FetchServiceResult<HashMap<String, Vec<ServiceDetail>>> {
    panic!("TODO");
}

#[get("/list")]
pub async fn list(data: web::Data<Arc<AppData>>) -> impl Responder {
    let services = match get_servers_info(&data.config).await {
        Ok(v) => v,
        Err(err) => {
            let err = format!("{}", err);
            error!("{}", err);
            return HttpResponse::Ok().json(&json!({ "err_msg": err }));
        }
    };

    let mut rsp = ListResult { list: Vec::new() };

    for module in services {
        rsp.list.push(ModuleServices {
            name: module.0,
            services: module.1.into_iter().map(|v| v.name).collect(),
        });
    }

    HttpResponse::Ok().json(&rsp)
}

fn get(
    services: &HashMap<String, Vec<ServiceDetail>>,
    service: web::Json<ServicePair>,
) -> FetchServiceResult<&ServiceDetail> {
    if let Some(module) = services.get(&service.module_name) {
        if let Some(service) = module.iter().find(|v| v.name == service.server_name) {
            Ok(service)
        } else {
            Err(FetchServiceError::NotFound)
        }
    } else {
        Err(FetchServiceError::NotFound)
    }
}

#[get("/rpcs")]
pub async fn get_rpcs(
    data: web::Data<Arc<AppData>>,
    service: web::Json<ServicePair>,
) -> impl Responder {
    let services = match get_servers_info(&data.config).await {
        Ok(v) => v,
        Err(err) => {
            let err = format!("{}", err);
            error!("{}", err);
            return HttpResponse::Ok().json(&json!({ "err_msg": err }));
        }
    };
    let service = match get(&services, service) {
        Ok(v) => v,
        Err(err) => {
            let err = format!("{}", err);
            error!("{}", err);
            return HttpResponse::Ok().json(&json!({ "err_msg": err }));
        }
    };

    let _meta = match get_service_meta(&service.address).await {
        Ok(v) => v,
        Err(err) => {
            let err = format!("{}", err);
            error!("{}", err);
            return HttpResponse::Ok().json(&json!({ "err_msg": err }));
        }
    };

    HttpResponse::Ok().json(&{})
}

#[get("/rpc")]
pub async fn get_rpc_info(
    data: web::Data<Arc<AppData>>,
    _rpc: web::Json<RpcPair>,
) -> impl Responder {
    let _services = match get_servers_info(&data.config).await {
        Ok(v) => v,
        Err(err) => {
            let err = format!("{}", err);
            error!("{}", err);
            return HttpResponse::Ok().json(&json!({ "err_msg": err }));
        }
    };
    HttpResponse::Ok().json(&{})
}

#[post("/request")]
pub async fn request(_req: web::Json<Value>) -> impl Responder {
    HttpResponse::Ok().json(&{})
}
