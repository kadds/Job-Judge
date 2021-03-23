use log::*;
use std::boxed::Box;
use std::collections::HashMap;

use crate::builtin::reflection::reflection_svr_server::*;
use crate::builtin::reflection::*;
use prost::Message;
use prost_types::FileDescriptorSet;
use tokio::sync::Mutex;
use tonic::{
    body::BoxBody,
    codegen::{BoxFuture, Never, Service},
    transport::NamedService,
    Request, Response, Status,
};

mod proto {
    use std::collections::{BTreeMap, HashMap};
    struct Method {
        name: String,
        input_name: String,
        output_name: String,
        client_stream: bool,
        server_stream: bool,
    }

    struct Service {
        methods: BTreeMap<String, Method>,
    }

    struct Field {
        name: String,
        number: i32,
    }

    struct Message {
        name: String,
        fields: BTreeMap<String, Field>,
    }

    struct Enum {
        name: String,
        values: BTreeMap<i32, String>,
    }

    struct Reflection {
        service: Service,
        messages: HashMap<String, Message>,
        enums: HashMap<String, Enum>,
    }
}

struct ServicePair {
    fd: FileDescriptorSet,
    service: Box<
        dyn Service<
                http::Request<BoxBody>,
                Response = http::Response<BoxBody>,
                Error = Never,
                Future = BoxFuture<http::Response<BoxBody>, Never>,
            > + Send
            + Sync
            + 'static,
    >,
}

pub struct ReflectionSvrImpl {
    map: HashMap<String, Mutex<ServicePair>>,
    description: String,
    meta_string: String,
}

#[tonic::async_trait]
impl ReflectionSvr for ReflectionSvrImpl {
    async fn get_meta(&self, request: Request<GetMetaReq>) -> Result<Response<GetMetaRsp>, Status> {
        info!("reflection meta request");
        let _req = request.into_inner();
        let rsp = GetMetaRsp {
            services: self.map.keys().cloned().collect(),
            description: self.description.clone(),
            meta_string: self.meta_string.clone(),
        };
        Ok(Response::new(rsp))
    }

    async fn get_rpc(&self, request: Request<GetRpcReq>) -> Result<Response<GetRpcRsp>, Status> {
        let req = request.into_inner();
        if req.rpc_name.is_empty() {
            // get all rpcs
            todo!();
        } else {
            // get single rpc detail
            if let Some(pair) = self.map.get(&req.service_name) {
                todo!();
            } else {
                Err(Status::not_found("service not found"))
            }
        }
    }

    async fn invoke(&self, request: Request<InvokeReq>) -> Result<Response<InvokeRsp>, Status> {
        let req = request.into_inner();
        todo!();
    }
}

pub struct Builder {
    map: HashMap<String, Mutex<ServicePair>>,
    with_self: bool,
    description: String,
    meta_string: String,
}

impl Builder {
    pub fn new() -> Self {
        Builder {
            map: HashMap::new(),
            with_self: false,
            description: "".to_owned(),
            meta_string: "".to_owned(),
        }
    }

    pub fn description<T: Into<String>>(mut self, desc: T) -> Self {
        self.description = desc.into();
        self
    }
    pub fn meta<T: Into<String>>(mut self, meta: T) -> Self {
        self.meta_string = meta.into();
        self
    }

    pub fn register_self(mut self) -> Self {
        self.with_self = true;
        self
    }
    pub fn register<S>(mut self, fd_set: &'static [u8], service: S) -> Self
    where
        S: Service<
                http::Request<BoxBody>,
                Response = http::Response<BoxBody>,
                Error = Never,
                Future = BoxFuture<http::Response<BoxBody>, Never>,
            >
            + Send
            + Sync
            + 'static
            + NamedService,
    {
        let decoded: FileDescriptorSet = FileDescriptorSet::decode(fd_set).unwrap();
        let name = <S as NamedService>::NAME;
        self.map.insert(
            name.to_owned(),
            Mutex::new(ServicePair {
                fd: decoded,
                service: Box::new(service),
            }),
        );
        self
    }

    fn parse(fd_set: &'static [u8]) {
        let decoded: FileDescriptorSet = FileDescriptorSet::decode(fd_set).unwrap();
    }

    pub fn build(self) -> ReflectionSvrServer<ReflectionSvrImpl> {
        let ret = ReflectionSvrServer::new(ReflectionSvrImpl {
            map: self.map,
            description: self.description,
            meta_string: self.meta_string,
        });
        if self.with_self {
            todo!();
        }
        ret
    }
}
