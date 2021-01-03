use actix_web::{
    get, HttpResponse, Responder, web, post
};
// use anyhow::Result;
use super::super::AppData;
use etcd_rs::{
    Client, ClientConfig, KeyRange, RangeRequest
};
use serde_json::{json, Value};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FetchServiceError {
    #[error("connect etcd error")]
    Connection(#[from] etcd_rs::Error),
    #[error("string format error")]
    Format(#[from] std::string::FromUtf8Error),
    #[error("broken data")]
    DataUnException,
    #[error("not found")]
    NotFound,
    #[error("unknown data store error")]
    Unknown,
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
    fn from(pair: ServicePair)-> Self {
        RpcPair {
            module_name:  pair.module_name,
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

async fn get_service_meta(address: &str) -> anyhow::Result<ServiceMeta> {
    Err(anyhow::Error::msg("fail"))
}

async fn get_servers_info(etcd_config: &crate::cfg::EtcdConfig)
        -> FetchServiceResult<HashMap<String, Vec<ServiceDetail>>> {
    let client = Client::connect(ClientConfig {
        endpoints: etcd_config.endpoints.to_owned(),
        auth: Some((etcd_config.username.to_owned(), etcd_config.password.to_owned())),
        tls: None,
    }).await?;

    let req = RangeRequest::new(KeyRange::prefix(format!("{}/", etcd_config.prefix)));

    let mut rsp = client.kv().range(req).await?;

    let mut map: HashMap<String, Vec<ServiceDetail>> = HashMap::new();
    info!("take range {}(s)", rsp.count());
    let vec = rsp.take_kvs();
    for mut kv in vec {
        let mut key = String::from_utf8(kv.take_key())?;
        let value = String::from_utf8(kv.take_value())?;
        let val = serde_json::Value::from(value.clone());

        key.replace_range(0..etcd_config.prefix.len(), "");
        let key_split = key.split('/'); 
        let mut key_split = key_split.skip(1);
        let module = key_split.next().ok_or(FetchServiceError::DataUnException)?;
        let name = key_split.next().ok_or(FetchServiceError::DataUnException)?;
        // info!("module {} name {}", module, name);
        let service_detail = ServiceDetail {
            name: name.to_owned(),
            address: val["address"].as_str().unwrap_or_default().to_owned()
        };
        debug!("{} {} {}", key, value, service_detail.address);
        if let Some(v) = map.get_mut(module) {
            v.push(service_detail);
        }
        else {
            map.insert(module.to_owned(), vec!(service_detail));
        }
    }
    Ok(map)
}

#[get("/list")]
pub async fn list(data: web::Data<Arc<AppData>>) -> impl Responder {
    let etcd_config = &data.config.etcd;
    let services = match get_servers_info(etcd_config).await {
        Ok(v) => v,
        Err(err) => {
            let err = format!("{}", err);
            error!("{}", err);
            return HttpResponse::Ok().json(json!({"err_msg": err} ));
        }        
    };

    let mut rsp = ListResult {
        list: Vec::new(),
    };

    for module in services {
        rsp.list.push(ModuleServices {
            name: module.0,
            services: module.1.into_iter().map(|v| v.name).collect(), 
        }); 
    }
     
    HttpResponse::Ok().json(rsp)
}

fn Get<'a>(services: &'a HashMap<String, Vec<ServiceDetail>>, service: web::Json<ServicePair>) 
    -> FetchServiceResult<&'a ServiceDetail>{
    if let Some(module) =  services.get(&service.module_name) {
        if let Some(service) = module.iter().find(|v| v.name == service.server_name) {
            Ok(service)
        }
        else {
            Err(FetchServiceError::NotFound)
        }
    }
    else {
        Err(FetchServiceError::NotFound)
    }
}

#[get("/rpcs")]
pub async fn get_rpcs(data: web::Data<Arc<AppData>>, service: web::Json<ServicePair>) -> impl Responder {
    let etcd_config = &data.config.etcd;
    let services = match get_servers_info(etcd_config).await {
        Ok(v) => v,
        Err(err) => {
            let err = format!("{}", err);
            error!("{}", err);
            return HttpResponse::Ok().json(json!({"err_msg": err} ));
        }        
    };
    let service = match Get(&services, service) {
        Ok(v) => v,
        Err(err) => {
            let err = format!("{}", err);
            error!("{}", err);
            return HttpResponse::Ok().json(json!({"err_msg": err} ));
        }
    };

    let meta = match get_service_meta(&service.address).await {
        Ok(v) => v,
        Err(err) => {
            let err = format!("{}", err);
            error!("{}", err);
            return HttpResponse::Ok().json(json!({"err_msg": err} ));
        }
    };

    HttpResponse::Ok().json({})
}

#[get("/rpc")]
pub async fn get_rpc_info(data: web::Data<Arc<AppData>>, rpc: web::Json<RpcPair>) -> impl Responder {
    let etcd_config = &data.config.etcd;
    let services = match get_servers_info(etcd_config).await {
        Ok(v) => v,
        Err(err) => {
            let err = format!("{}", err);
            error!("{}", err);
            return HttpResponse::Ok().json(json!({"err_msg": err} ));
        }        
    };
    HttpResponse::Ok().json({})
}


#[post("/request")]
pub async fn request(req: web::Json<Value>) -> impl Responder {
    HttpResponse::Ok().json({})
}