use log::*;
use std::boxed::Box;
use std::collections::HashMap;

use crate::builtin::reflection::reflection_svr_server::*;
use crate::builtin::reflection::*;
use tokio::sync::Mutex;
use tonic::{
    body::BoxBody,
    codegen::{BoxFuture, Never, Service},
    transport::NamedService,
    Request, Response, Status,
};

mod proto {
    use std::collections::{BTreeMap, HashMap};

    use log::*;
    use prost::Message as M;
    use prost_types::{DescriptorProto, FileDescriptorSet};


    pub(crate) struct Method {
        pub input_type: Message,
        pub output_type: Message,
        pub client_stream: bool,
        pub server_stream: bool,
    }

    pub(crate) struct Service {
        pub methods: BTreeMap<String, Method>,
    }
    pub(crate) enum TypeName {
        Message(Message),
        Enum(Enum),
    }

    pub(crate) struct Field {
        type_name: Option<Box<TypeName>>,
        number: i32,
    }

    pub(crate) struct Message {
        fields: BTreeMap<String, Field>,
    }

    pub(crate) struct Enum {
        values: BTreeMap<i32, String>,
    }

    pub(crate) struct ReflectionInstance {
        pub service: Service,
        pub messages: HashMap<String, Message>,
        pub enums: HashMap<String, Enum>,
    }

    fn find_message(name: &str, messages: BTreeMap<String, Message>, enums: BTreeMap<String, Enum>) -> Message {

    }

    fn parse_message(package: &str, m: DescriptorProto) -> Message {
        let name = format!("{}.{}", package, m.name.unwrap());
        let mut fields = BTreeMap::new();
        for f in m.field {
            let name = f.name.unwrap();
            fields.insert(
                name,
                Field {
                    number: f.number.unwrap(),
                    type_name: f.type_name.unwrap_or_default(),
                },
            );
        }
    }

    pub(crate) fn parse(fd_set: &'static [u8], name: &str) -> ReflectionInstance {
        let f: FileDescriptorSet = FileDescriptorSet::decode(fd_set).unwrap();
        let mut methods = BTreeMap::new();
        let mut messages = HashMap::new();
        let mut enums = HashMap::new();
        info!("{:?}", f.clone());

        for file in f.file {
            let file_package = file.package.unwrap();
            let file_name = file.name.unwrap();
            debug!("get proto file {} in {}", file_name, file_package);

            for m in file.message_type {
                let name = format!("{}.{}", file_package, m.name.unwrap());
                let mut fields = BTreeMap::new();
                for f in m.field {
                    let name = f.name.unwrap();
                    fields.insert(
                        name,
                        Field {
                            number: f.number.unwrap(),
                            type_name: f.type_name.unwrap_or_default(),
                        },
                    );
                }
                messages.insert(name, Message { fields });
            }
            for e in file.enum_type {
                let name = e.name.unwrap();
                let mut values = BTreeMap::new();
                for f in e.value {
                    values.insert(f.number.unwrap(), f.name.unwrap());
                }
                enums.insert(name, Enum { values });
            }
        }

        for file in f.file {
            let file_package = file.package.unwrap();
            let file_name = file.name.unwrap();
            for s in file.service {
                if format!("{}.{}", file_package, s.name.unwrap()) == name {
                    debug!("found service {} at {}", name, file_name);
                    for m in s.method {
                        let name = m.name.unwrap();
                        methods.insert(
                            name,
                            Method {
                                input_type_name: find_message(&m.input_type.unwrap(), messages, enums),
                                output_type_name: m.output_type.unwrap(),
                                client_stream: m.client_streaming.unwrap_or_default(),
                                server_stream: m.server_streaming.unwrap_or_default(),
                            },
                        );
                    }
                }
            }
        }

        let service = Service { methods };

        ReflectionInstance {
            service,
            messages,
            enums,
        }
    }
}

struct ServicePair {
    reflection: proto::ReflectionInstance,
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
        if let Some(pair) = self.map.get(&req.service_name) {
            if req.rpc_name.is_empty() {
                // get all rpcs
                let p = pair.lock().await;
                let rpcs = p
                    .reflection
                    .service
                    .methods
                    .iter()
                    .map(|v| v.0.to_owned())
                    .collect();
                Ok(Response::new(GetRpcRsp {
                    res: Some(get_rpc_rsp::Res::Rpcs(BasicRpcs { name: rpcs })),
                }))
            } else {
                // get single rpc detail
                todo!();
            }
        } else {
            let msg = format!("service name {} not found", req.service_name);
            error!("{}", msg);
            Err(Status::not_found(msg))
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
        let name = <S as NamedService>::NAME;
        self.map.insert(
            name.to_owned(),
            Mutex::new(ServicePair {
                reflection: proto::parse(fd_set, name),
                service: Box::new(service),
            }),
        );
        self
    }

    pub fn build(self) -> ReflectionSvrServer<ReflectionSvrImpl> {
        let name = <ReflectionSvrServer<ReflectionSvrImpl> as NamedService>::NAME;
        let inner = ReflectionSvrImpl {
            map: self.map,
            description: self.description,
            meta_string: self.meta_string,
        };
        let ret = ReflectionSvrServer::new(inner);

        if self.with_self {
            // map.insert(
            //     name.to_owned(),
            //     Mutex::new(ServicePair {
            //         reflection: proto::parse(FILE_DESCRIPTOR_SET, name),
            //         service: Box::new(ret.clone()),
            //     })
            // );
        }
        ret
    }
}
