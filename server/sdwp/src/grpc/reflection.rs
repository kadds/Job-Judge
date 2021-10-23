use futures_util::stream;
use prost::{DecodeError, Message};
use std::{
    collections::{HashMap, HashSet},
    convert::TryInto,
    iter::Iterator,
    mem::swap,
    net::SocketAddr,
    sync::Arc,
};
use tonic::transport::Channel;
tonic::include_proto!("grpc.reflection.v1alpha");
use super::any_message::{Field, Type, *};
use super::{get_channel, get_module_address, GrpcError, GrpcResult};
use rand::Rng;
use serde::{Deserialize, Serialize};
use server_reflection_client::ServerReflectionClient;
use server_reflection_request::MessageRequest;
use server_reflection_response::MessageResponse;

async fn query_symbols_inner<T: Iterator<Item = S>, S: ToString>(
    channel: Channel,
    symbols: T,
) -> GrpcResult<Vec<prost_types::FileDescriptorProto>> {
    let mut client = ServerReflectionClient::new(channel);

    let reqs: Vec<ServerReflectionRequest> = symbols
        .into_iter()
        .map(|v| ServerReflectionRequest {
            host: "".to_owned(),
            message_request: Some(MessageRequest::FileContainingSymbol(v.to_string())),
        })
        .collect();

    let mut symbols = String::with_capacity(512);
    for req in &reqs {
        symbols.push_str(match &req.message_request {
            Some(MessageRequest::FileContainingSymbol(v)) => v,
            _ => {
                panic!("");
            }
        });
        symbols.push(',');
    }
    log::info!("query symbols: {}", symbols);

    let mut rsp = client
        .server_reflection_info(tonic::Request::new(stream::iter(reqs)))
        .await?
        .into_inner();
    let mut result = Vec::new();
    while let Some(rsp) = rsp.message().await? {
        if let MessageResponse::FileDescriptorResponse(rsp) =
            rsp.message_response.ok_or(GrpcError::LogicError("empty response"))?
        {
            let fds: Result<Vec<prost_types::FileDescriptorProto>, DecodeError> = rsp
                .file_descriptor_proto
                .into_iter()
                .map(|v| prost_types::FileDescriptorProto::decode(v.as_slice()))
                .collect();
            let fds = fds.map_err(GrpcError::DecodeError)?;
            result.extend(fds);
        }
    }
    Ok(result)
}

pub struct SymbolQueryContext<'a> {
    channel: Channel,
    map: HashMap<String, CommonTypeProto>,
    path: Vec<&'a str>,
    require_types: HashSet<String>,
    relate: HashMap<String, CommonType>,
}

impl<'a> SymbolQueryContext<'a> {
    pub fn new(channel: Channel, proto_map: HashMap<String, CommonTypeProto>, base_path: Vec<&'a str>) -> Self {
        Self {
            channel,
            map: proto_map,
            path: base_path,
            require_types: HashSet::new(),
            relate: HashMap::new(),
        }
    }

    pub fn add_type<S: ToString>(&mut self, type_name: S) {
        self.require_types.insert(type_name.to_string());
    }

    fn make_types_r(prefix: &str, descs: &[prost_types::DescriptorProto], map: &mut HashMap<String, CommonTypeProto>) {
        for desc in descs {
            let full_name = format!("{}.{}", prefix, desc.name());
            if !desc.nested_type.is_empty() {
                Self::make_types_r(&full_name, &desc.nested_type, map);
            }
            if !desc.enum_type.is_empty() {
                Self::make_types_e_r(&full_name, &desc.enum_type, map);
            }
            map.insert(full_name, CommonTypeProto::Message(desc.clone()));
        }
    }

    fn make_types_e_r(
        prefix: &str,
        descs: &[prost_types::EnumDescriptorProto],
        map: &mut HashMap<String, CommonTypeProto>,
    ) {
        for desc in descs {
            let full_name = format!("{}.{}", prefix, desc.name());
            map.insert(full_name, CommonTypeProto::Enum(desc.clone()));
        }
    }

    pub fn make_types(symbols: &[prost_types::FileDescriptorProto]) -> HashMap<String, CommonTypeProto> {
        let mut map = HashMap::new();
        for file in symbols {
            let prefix = format!(".{}", file.package());
            Self::make_types_r(&prefix, &file.message_type, &mut map);
            Self::make_types_e_r(&prefix, &file.enum_type, &mut map);
        }
        // for x in &map {
        //     log::info!("found type {}", x.0);
        // }
        map
    }

    fn map_ktype(&self, t: &str, ext: std::option::Option<i32>) -> Type {
        use prost_types::field::Kind;
        if let Some(ext) = ext {
            return if ext == Kind::TypeBool as i32 {
                Type::Bool
            } else if ext == Kind::TypeDouble as i32 {
                Type::Double
            } else if ext == Kind::TypeFloat as i32 {
                Type::Float
            } else if ext == Kind::TypeInt64 as i32 {
                Type::Int64
            } else if ext == Kind::TypeUint64 as i32 {
                Type::Uint64
            } else if ext == Kind::TypeInt32 as i32 {
                Type::Int32
            } else if ext == Kind::TypeFixed64 as i32 {
                Type::Fixed64
            } else if ext == Kind::TypeFixed32 as i32 {
                Type::Fixed32
            } else if ext == Kind::TypeString as i32 {
                Type::String
            } else if ext == Kind::TypeMessage as i32 {
                Type::Message(t.to_owned())
            } else if ext == Kind::TypeBytes as i32 {
                Type::Bytes
            } else if ext == Kind::TypeUint32 as i32 {
                Type::Uint32
            } else if ext == Kind::TypeEnum as i32 {
                Type::Enum(t.to_owned())
            } else if ext == Kind::TypeSfixed32 as i32 {
                Type::Sfixed32
            } else if ext == Kind::TypeSfixed64 as i32 {
                Type::Sfixed64
            } else if ext == Kind::TypeSint32 as i32 {
                Type::Sint32
            } else if ext == Kind::TypeSint64 as i32 {
                Type::Sint64
            } else {
                Type::Invalid
            };
        }
        Type::Invalid
    }

    fn to_label(&self, label: prost_types::field_descriptor_proto::Label) -> Label {
        match label {
            prost_types::field_descriptor_proto::Label::Optional => Label::Optional,
            prost_types::field_descriptor_proto::Label::Repeated => Label::Repeated,
            prost_types::field_descriptor_proto::Label::Required => Label::Required,
        }
    }

    fn parse_fields(&mut self, fields: &[prost_types::FieldDescriptorProto]) -> Vec<Field> {
        fields
            .iter()
            .map(|v| {
                let ktype = self.map_ktype(v.type_name(), v.r#type);
                if let Type::Message(m) = &ktype {
                    self.require_types.insert(m.to_string());
                } else if let Type::Enum(m) = &ktype {
                    self.require_types.insert(m.to_string());
                }
                let label = self.to_label(v.label());
                Field {
                    name: v.name().to_owned(),
                    pos: v.number() as u32,
                    ktype,
                    deprecated: v.options.as_ref().map_or(false, |v| v.deprecated()),
                    packed: v.options.as_ref().map_or(false, |v| v.packed()),
                    label,
                    json_name: v.json_name().to_owned(),
                    oneof_index: v.oneof_index.map(|v| v as usize),
                }
            })
            .collect()
    }

    fn parse_enums(&self, enums: &[prost_types::EnumValueDescriptorProto]) -> Vec<EnumField> {
        enums
            .iter()
            .map(|v| EnumField {
                name: v.name().to_owned(),
                pos: v.number(),
                deprecated: v.options.as_ref().map_or(false, |v| v.deprecated()),
            })
            .collect()
    }

    fn inner_query_type(&mut self, full_name: &str) -> std::option::Option<CommonType> {
        if let Some(t) = self.map.get(full_name).cloned() {
            return Some(match t {
                CommonTypeProto::Message(m) => {
                    let name = m.name().to_owned();
                    let oneofs = m.oneof_decl.iter().map(|v| v.name().to_owned()).collect();

                    let fields = self.parse_fields(&m.field);
                    CommonType::Message(MessageType {
                        fields,
                        name,
                        oneofs,
                    })
                }
                CommonTypeProto::Enum(e) => {
                    let enums = self.parse_enums(&e.value);
                    let name = e.name().to_owned();
                    CommonType::Enum(EnumType { enums, name })
                }
            });
        }
        None
    }

    fn query_type(&mut self, name: &str) -> std::option::Option<CommonType> {
        if name.starts_with('.') {
            return self.inner_query_type(name);
        }
        // name resolve
        while !self.path.is_empty() {
            let full_name = format!(".{}.{}", self.path.join("."), name);
            let res = self.inner_query_type(&full_name);
            if res.is_some() {
                return res;
            }
            self.path.pop().unwrap();
        }
        None
    }

    pub async fn parse(&mut self) -> GrpcResult<HashMap<String, CommonType>> {
        while !self.require_types.is_empty() {
            let symbols = query_symbols_inner(self.channel.clone(), self.require_types.iter().map(|v| &v[1..])).await?;
            self.map.extend(Self::make_types(&symbols));
            let mut success = true;

            while success {
                success = false;
                let mut t = HashSet::new();
                swap(&mut t, &mut self.require_types);

                for type_name in t {
                    #[allow(clippy::map_entry)]
                    if !self.relate.contains_key(&type_name) {
                        if let Some(t) = self.query_type(&type_name) {
                            self.relate.insert(type_name, t);
                            success = true;
                        } else {
                            self.require_types.insert(type_name);
                        }
                    }
                }
            }
        }

        let mut x: HashMap<String, CommonType> = HashMap::new();
        std::mem::swap(&mut x, &mut self.relate);
        Ok(x)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcInfo {
    pub name: String,
    pub full_name: String,
    pub request_typename: String,
    pub request_stream: bool,
    pub response_typename: String,
    pub response_stream: bool,
    pub relate_schema: Arc<HashMap<String, CommonType>>,
}

#[derive(Debug)]
pub struct RequestContext {
    addrs: Vec<(String, SocketAddr)>,
    addr: String,
    channel: tonic::transport::Channel,
}

impl RequestContext {
    pub async fn with_any(cfg: &micro_service::cfg::DiscoverConfig, module: &str) -> GrpcResult<Self> {
        let mut rng = rand::thread_rng();
        let addrs = get_module_address(cfg, module).await?;
        if addrs.is_empty() {
            return Err(GrpcError::EmptyInstance(module.to_owned()));
        }
        let addr = addrs
            .get(rng.gen_range(0..addrs.len()))
            .ok_or(GrpcError::LogicError("random pick fail"))?;
        let channel = get_channel(addr.1).await?;
        Ok(RequestContext {
            addr: addr.0.to_owned(),
            channel,
            addrs,
        })
    }

    pub async fn with_name(cfg: &micro_service::cfg::DiscoverConfig, module: &str, instance: &str) -> GrpcResult<Self> {
        let addrs = get_module_address(cfg, module).await?;
        let addr = addrs
            .iter()
            .find(|v| v.0 == instance)
            .ok_or_else(|| GrpcError::InstanceNotFound(instance.to_owned()))?;
        let channel = get_channel(addr.1).await?;
        Ok(RequestContext {
            addr: addr.0.to_owned(),
            channel,
            addrs,
        })
    }

    pub async fn new(cfg: &micro_service::cfg::DiscoverConfig, module: &str, instance: &str) -> GrpcResult<Self> {
        if instance.is_empty() {
            Self::with_any(cfg, module).await
        } else {
            Self::with_name(cfg, module, instance).await
        }
    }

    pub async fn list_services(&self) -> GrpcResult<Vec<String>> {
        let mut list = Vec::new();

        let mut client = ServerReflectionClient::new(self.channel.clone());
        let req = ServerReflectionRequest {
            host: "".into(),
            message_request: Some(MessageRequest::ListServices("".into())),
        };
        let mut rsp = client
            .server_reflection_info(tonic::Request::new(stream::iter([req])))
            .await?
            .into_inner();
        while let Some(rsp) = rsp.message().await? {
            if let MessageResponse::ListServicesResponse(rsp) =
                rsp.message_response.ok_or(GrpcError::LogicError("empty response"))?
            {
                list.extend(rsp.service.into_iter().map(|v| v.name));
            }
        }

        Ok(list)
    }

    pub fn instance(&self) -> (String, Vec<String>) {
        (self.addr.to_owned(), self.addrs.iter().map(|v| v.0.clone()).collect())
    }

    pub async fn pick_services_or(&self, service: &str) -> GrpcResult<(String, Vec<String>)> {
        let mut res = self.list_services().await?;
        if res.is_empty() {
            return Err(GrpcError::ServiceNotFound("list services is empty".to_string()));
        }
        res.sort();
        if service.is_empty() {
            res.iter()
                .find(|v| !v.starts_with("grpc."))
                .cloned()
                .ok_or_else(|| GrpcError::ServiceNotFound(service.to_owned()))
        } else {
            res.iter()
                .find(|v| **v == service)
                .cloned()
                .ok_or_else(|| GrpcError::ServiceNotFound(service.to_owned()))
        }
        .map(|v| (v, res))
    }

    async fn query_symbols<T: Iterator<Item = S>, S: ToString>(
        &self,
        symbols: T,
    ) -> GrpcResult<Vec<prost_types::FileDescriptorProto>> {
        query_symbols_inner(self.channel.clone(), symbols).await
    }

    pub async fn list_rpcs(&self, service: &str) -> GrpcResult<Vec<String>> {
        let symbols = self.query_symbols(std::iter::once(service.to_owned())).await?;
        let p = service.rfind('.').ok_or(GrpcError::InvalidParameters)?;
        let name = &service[p + 1..];

        for file in symbols {
            if service.starts_with(file.package()) {
                if let Some(service_proto) = file.service.into_iter().find(|v| v.name() == name) {
                    return Ok(service_proto.method.into_iter().map(|v| v.name().to_owned()).collect());
                }
            }
        }
        Err(GrpcError::ServiceNotFound(service.to_owned()))
    }

    pub async fn rpc_info(&self, service: &str, method_name: &str) -> GrpcResult<RpcInfo> {
        let rpc_name = format!("{}.{}", service, method_name);
        let symbols = self.query_symbols(std::iter::once(rpc_name.to_owned())).await?;
        let map = SymbolQueryContext::make_types(&symbols);

        let p = service.rfind('.').ok_or(GrpcError::InvalidParameters)?;
        let name = &service[p + 1..];
        for file in symbols {
            if service.starts_with(file.package()) {
                if let Some(service_proto) = file.service.iter().find(|v| v.name() == name) {
                    let method = service_proto
                        .method
                        .iter()
                        .find(|v| v.name() == method_name)
                        .ok_or_else(|| GrpcError::RpcNotFound(rpc_name.to_owned()))?;

                    log::info!("req {} rsp {}", method.input_type(), method.output_type());

                    let mut path = Vec::new();
                    path.extend(file.package().split('.'));

                    let mut context = SymbolQueryContext::new(self.channel.clone(), map, path);
                    context.add_type(method.input_type());
                    context.add_type(method.output_type());

                    let relate = context.parse().await?;

                    return Ok(RpcInfo {
                        name: method_name.to_owned(),
                        full_name: rpc_name,
                        request_stream: method.client_streaming(),
                        response_stream: method.server_streaming(),
                        request_typename: method.input_type().to_owned(),
                        response_typename: method.output_type().to_owned(),
                        relate_schema: Arc::new(relate),
                    });
                }
            }
        }
        Err(GrpcError::RpcNotFound(rpc_name.to_owned()))
    }

    pub async fn invoke(
        &self,
        service: &str,
        method_name: &str,
        request: serde_json::Value,
    ) -> GrpcResult<serde_json::Value> {
        let rpc_info = self.rpc_info(service, method_name).await?;
        let ctx = AnyMessageContext::new(rpc_info.relate_schema.clone());
        let mut message = AnyMessage::new_encode(request, ctx.clone());
        message.set_message_target(rpc_info.request_typename.clone());
        message.encode_check()?;

        let mut grpc = tonic::client::Grpc::new(self.channel.clone());
        let path = format!("/{}/{}", service, method_name)
            .try_into()
            .map_err(|_| GrpcError::InvalidUri)?;

        let codec =
            AnyProstCodec::new(rpc_info.request_typename.clone(), rpc_info.response_typename.clone(), ctx.clone());

        grpc.ready().await.map_err(|_| GrpcError::NetError)?;
        // let ret = if rpc_info.request_stream {
        //     if rpc_info.response_stream {
        //         grpc.unary(tonic::Request::new(message), path, codec)
        //     } else {
        //         grpc.unary(tonic::Request::new(message), path, codec)
        //     }
        // } else {
        //     if rpc_info.response_stream {
        //         grpc.unary(tonic::Request::new(message), path, codec)
        //     } else {
        //         grpc.unary(tonic::Request::new(message), path, codec)
        //     }
        // }
        // .await?;
        let ret = grpc.unary(tonic::Request::new(message), path, codec).await?;
        Ok(ret.into_inner().value())
    }
}
