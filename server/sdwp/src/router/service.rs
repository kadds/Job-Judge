use crate::{grpc, AppData};
use actix_web::{get, post, web, HttpResponse, Responder};
use grpc::reflection;
use log::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize)]
pub struct ServerPair {
    pub module_name: String,
    pub service_name: String,
    pub instance_name: String,
}
#[derive(Serialize, Deserialize)]
pub struct RpcPair {
    pub module_name: String,
    pub service_name: String,
    pub instance_name: String,
    pub rpc_name: String,
}

impl From<ServerPair> for RpcPair {
    fn from(pair: ServerPair) -> Self {
        RpcPair {
            module_name: pair.module_name,
            service_name: pair.service_name,
            instance_name: pair.instance_name,
            rpc_name: "".to_owned(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct SingleService {
    pub module_name: String,
    pub services_names: Vec<String>,
    pub description: String,
    pub instances: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct ListResult {
    pub list: Vec<SingleService>,
}

#[derive(Serialize, Deserialize)]
struct ListRpcResult {
    pub list: Vec<String>,
}

#[get("/list")]
pub async fn list(data: web::Data<AppData>) -> impl Responder {
    let mut f0 = vec![];
    for s in data.config.services.iter() {
        f0.push(reflection::get_meta(&data.config, s));
    }
    let services = futures::future::join_all(f0).await;

    let list = services
        .into_iter()
        .zip(data.config.services.iter())
        .flat_map(|(v, name)| match v {
            Ok(v) => Some(SingleService {
                module_name: name.to_owned(),
                services_names: v.inner_services,
                description: v.description,
                instances: v.instances.into_iter().map(|v| v.0).collect(),
            }),
            Err(err) => {
                error!("get module {} fail. {}", name, err);
                None
            }
        })
        .collect();

    let rsp = ListResult { list };
    HttpResponse::Ok().json(&rsp)
}

#[get("/rpcs")]
pub async fn get_rpcs(data: web::Data<AppData>, server: web::Json<ServerPair>) -> impl Responder {
    let addr = match reflection::get_instance_address(
        &data.config,
        &server.module_name,
        &server.instance_name,
    )
    .await
    {
        Ok(v) => v,
        Err(err) => {
            let err = format!("try get instance address fail. {}", err);
            error!("{}", err);
            return HttpResponse::Ok().json(&json!({ "err_msg": err }));
        }
    };

    let rpcs = match reflection::get_rpcs(&server.service_name, addr).await {
        Ok(v) => v,
        Err(err) => {
            let err = format!("try get rpcs fail. {}", err);
            error!("{}", err);
            return HttpResponse::Ok().json(&json!({ "err_msg": err }));
        }
    };
    let rsp = ListRpcResult { list: rpcs };
    HttpResponse::Ok().json(&rsp)
}

#[get("/rpc")]
pub async fn get_rpc_info(data: web::Data<AppData>, _rpc: web::Json<RpcPair>) -> impl Responder {
    HttpResponse::Ok().json(&{})
}

#[post("/invoke")]
pub async fn invoke(_req: web::Json<Value>) -> impl Responder {
    HttpResponse::Ok().json(&{})
}
