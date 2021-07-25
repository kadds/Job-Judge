use futures_util::stream;
use log::info;
use prost::{DecodeError, Message};
use tonic::client::Grpc;
use std::{ascii::AsciiExt, collections::{HashMap, HashSet}, hash::Hash, io::Read, mem::swap, net::SocketAddr};
tonic::include_proto!("grpc.reflection.v1alpha");

use bytes::{Buf, BufMut};
use prost_types::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use server_reflection_client::ServerReflectionClient;
use server_reflection_request::*;
use server_reflection_response::*;

use super::{get_channel, get_module_address, GrpcError, GrpcResult};
#[derive(Debug)]
pub struct RequestContext {
    addrs: Vec<(String, SocketAddr)>,
    addr: String,
    channel: tonic::transport::Channel,
}

#[derive(Debug)]
pub struct DynMessage {}
use prost::encoding::{
    decode_key, encode_varint, encoded_len_varint, message, DecodeContext, WireType,
};

pub struct QueryContext<'a> {
    map: HashMap<String, InnerTypeProto>,
    relate: HashMap<String, InnerType>,
    path: Vec<&'a str>,
    require_types: HashSet<String>,
}

impl Message for DynMessage {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        todo!()
    }

    fn merge_field<B>(
        &mut self,
        tag: u32,
        wire_type: WireType,
        buf: &mut B,
        ctx: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        B: Buf,
        Self: Sized,
    {
        todo!()
    }

    fn encoded_len(&self) -> usize {
        todo!()
    }

    fn clear(&mut self) {
        todo!()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Type {
    Double,
    Float,
    Int64,
    Uint64,
    Int32,
    Fixed64,
    Fixed32,
    Bool,
    String,
    Group,
    Message(String),
    Bytes,
    Uint32,
    Enum(String),
    Sfixed32,
    Sfixed64,
    Sint32,
    Sint64,
    Invalid,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Field {
    pub ktype: Type,
    pub name: String,
    pub pos: i32,
    pub deprecated: bool,
    pub packed: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EField {
    pub name: String,
    pub pos: i32,
    pub deprecated: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageType {
    pub name: String,
    pub full_name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnumType {
    pub name: String,
    pub full_name: String,
    pub enums: Vec<EField>,
}

#[derive(Debug, Clone)]
pub enum InnerTypeProto {
    Message(DescriptorProto),
    Enum(EnumDescriptorProto),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum InnerType {
    Message(MessageType),
    Enum(EnumType),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcInfo {
    pub name: String,
    pub full_name: String,
    pub request_typename: String,
    pub request_stream: bool,
    pub response_typename: String,
    pub response_stream: bool,
    pub relate_schema: HashMap<String, InnerType>,
}

impl RequestContext {
    pub async fn with_any(cfg: &crate::cfg::Config, module: &str) -> GrpcResult<Self> {
        let mut rng = rand::thread_rng();
        let addrs = get_module_address(cfg, module).await?;
        if addrs.is_empty() {
            return Err(GrpcError::EmptyInstance(module.to_owned()));
        }
        let addr = addrs.get(rng.gen_range(0..addrs.len())).unwrap();
        let channel = get_channel(addr.1).await?;
        Ok(RequestContext {
            addr: addr.0.to_owned(),
            channel,
            addrs,
        })
    }
    pub async fn with_name(
        cfg: &crate::cfg::Config,
        module: &str,
        instance: &str,
    ) -> GrpcResult<Self> {
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

    pub async fn new(cfg: &crate::cfg::Config, module: &str, instance: &str) -> GrpcResult<Self> {
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
            match rsp.message_response.unwrap() {
                MessageResponse::ListServicesResponse(rsp) => {
                    list.extend(rsp.service.into_iter().map(|v| v.name));
                }
                _ => (),
            }
        }

        Ok(list)
    }

    pub fn instance(&self) -> (String, Vec<String>) {
        (
            self.addr.to_owned(),
            self.addrs.iter().map(|v| v.0.clone()).collect(),
        )
    }

    pub async fn pick_services_or(&self, service: &str) -> GrpcResult<(String, Vec<String>)> {
        let mut res = self.list_services().await?;
        if res.is_empty() {
            return Err(GrpcError::ServiceNotFound(format!(
                "list services is empty"
            )));
        }
        res.sort();
        if service.is_empty() {
            res.iter()
                .filter(|v| !v.starts_with("grpc."))
                .next()
                .cloned()
                .ok_or_else(|| GrpcError::ServiceNotFound(service.to_owned()))
        } else {
            res.iter()
                .find(|v| **v == service)
                .cloned()
                .ok_or_else(|| GrpcError::ServiceNotFound(service.to_owned()))
        }
        .map(|v| (v.to_owned(), res))
    }

    async fn query_symbols<IT: std::iter::Iterator<Item = S>, S: ToString>(&self, symbols: IT) -> GrpcResult<Vec<FileDescriptorProto>> {
        let mut client = ServerReflectionClient::new(self.channel.clone());

        let reqs: Vec<ServerReflectionRequest> = symbols.into_iter().map(|v | ServerReflectionRequest {
            host: "".to_owned(),
            message_request: Some(MessageRequest::FileContainingSymbol(v.to_string())),
        }).collect();

        let mut symbols = String::with_capacity(512);
        for req in &reqs {
            symbols.push_str(match &req.message_request {
                Some(MessageRequest::FileContainingSymbol(v)) => v,
                _ => {panic!("");}
            });
            symbols.push(',');
        }
        log::info!("query symbols: {}", symbols);

        let mut rsp = client
            .server_reflection_info(tonic::Request::new(stream::iter(reqs ) ) )
            .await?
            .into_inner();
        let mut result = Vec::new();
        while let Some(rsp) = rsp.message().await? {
            match rsp.message_response.unwrap() {
                MessageResponse::FileDescriptorResponse(rsp) => {
                    let fds: Result<Vec<FileDescriptorProto>, DecodeError> = rsp
                        .file_descriptor_proto
                        .into_iter()
                        .map(|v| FileDescriptorProto::decode(v.as_slice()))
                        .collect();
                    let fds = fds.map_err(|v| GrpcError::DecodeError(v))?;
                    result.extend(fds);
                }
                _ => (),
            }
        }
        Ok(result)
    }

    pub async fn list_rpcs(&self, service: &str) -> GrpcResult<Vec<String>> {
        let symbols = self.query_symbols(std::iter::once(service.to_owned())).await?;
        let p = service.rfind('.').ok_or(GrpcError::InvalidParameters)?;
        let name = &service[p + 1..];

        for file in symbols {
            if service.starts_with(file.package()) {
                if let Some(service_proto) = file.service.into_iter().find(|v| v.name() == name) {
                    return Ok(service_proto
                        .method
                        .into_iter()
                        .map(|v| v.name().to_owned())
                        .collect());
                }
            }
        }
        Err(GrpcError::ServiceNotFound(service.to_owned()))
    }

    fn make_types_r(
        &self,
        prefix: &str,
        p: &Vec<DescriptorProto>,
        map: &mut HashMap<String, InnerTypeProto>,
    ) {
        for a in p {
            let full_name = format!("{}.{}", prefix, a.name());
            if !a.nested_type.is_empty() {
                self.make_types_r(&full_name, &a.nested_type, map);
            }
            if !a.enum_type.is_empty() {
                self.make_types_e_r(&full_name, &a.enum_type, map);
            }
            map.insert(full_name, InnerTypeProto::Message(a.clone()));
        }
    }

    fn make_types_e_r(
        &self,
        prefix: &str,
        p: &Vec<EnumDescriptorProto>,
        map: &mut HashMap<String, InnerTypeProto>,
    ) {
        for a in p {
            let full_name = format!("{}.{}", prefix, a.name());
            map.insert(full_name, InnerTypeProto::Enum(a.clone()));
        }
    }

    fn make_types(&self, symbols: &Vec<FileDescriptorProto>) -> HashMap<String, InnerTypeProto> {
        let mut map = HashMap::new();
        for file in symbols {
            let prefix = format!(".{}", file.package());
            self.make_types_r(&prefix, &file.message_type, &mut map);
            self.make_types_e_r(&prefix, &file.enum_type, &mut map);
        }
        for x in &map {
            log::info!("found type {}", x.0);
        }
        map
    }
    fn map_ktype(&self, t: &str, ext: std::option::Option<i32>) -> Type {
        use field::Kind;
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
            } else if ext == Kind::TypeGroup as i32 {
                Type::Group
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

    fn parse_fields(
        &self,
        fields: &Vec<FieldDescriptorProto>,
        context: &mut QueryContext<'_>,
    ) -> Vec<Field> {
        fields
            .iter()
            .map(|v| {
                let ktype = self.map_ktype(v.type_name(), v.r#type);
                if let Type::Message(m) = &ktype {
                    if let Some(t) = self.query_type(&m, context) {
                        context.relate.insert(m.to_owned(), t);
                    } else {
                        context.require_types.insert(m.to_owned());
                    }
                } else if let Type::Enum(m) = &ktype {
                    if let Some(t) = self.query_type(&m, context) {
                        context.relate.insert(m.to_owned(), t);
                    } else {
                        context.require_types.insert(m.to_owned());
                    }
                }
                Field {
                    name: v.name().to_owned(),
                    pos: v.number(),
                    ktype,
                    deprecated: v.options.as_ref().map_or(false, |v| v.deprecated()),
                    packed: v.options.as_ref().map_or(false, |v| v.packed()),
                }
            })
            .collect()
    }

    fn parse_enums(&self, enums: &Vec<EnumValueDescriptorProto>) -> Vec<EField> {
        enums
            .iter()
            .map(|v| EField {
                name: v.name().to_owned(),
                pos: v.number(),
                deprecated: v.options.as_ref().map_or(false, |v| v.deprecated()),
            })
            .collect()
    }

    fn inner_query_type(
        &self,
        full_name: &str,
        context: &mut QueryContext<'_>,
    ) -> std::option::Option<InnerType> {
        if let Some(t) = context.map.get(full_name).cloned() {
            return Some(match t {
                InnerTypeProto::Message(m) => {
                    let name = m.name().to_owned();
                    let fields = self.parse_fields(&m.field, context);
                    InnerType::Message(MessageType {
                        fields,
                        name,
                        full_name: full_name.to_owned(),
                    })
                }
                InnerTypeProto::Enum(e) => {
                    let enums = self.parse_enums(&e.value);
                    let name = e.name().to_owned();
                    InnerType::Enum(EnumType {
                        enums,
                        name,
                        full_name: full_name.to_owned(),
                    })
                }
            });
        }
        None
    }

    fn query_type(
        &self,
        name: &str,
        context: &mut QueryContext<'_>,
    ) -> std::option::Option<InnerType> {
        if name.starts_with(".") {
            return self.inner_query_type(name, context);
        }
        // name query
        while !context.path.is_empty() {
            let full_name = format!(".{}.{}", context.path.join("."), name);
            let res = self.inner_query_type(&full_name, context);
            if res.is_some() {
                return res;
            }
            context.path.pop().unwrap();
        }
        None
    }


    async fn query_and_parse_types(&self, context: &mut QueryContext<'_>) -> GrpcResult<()> {
        let symbols = self.query_symbols(context.require_types.iter().map(|v| &v[1..])).await?;
        let map = self.make_types(&symbols);
        context.map.extend(map);
        self.consume_types(context, true)?;
        Ok(())
    }

    fn consume_types(&self, context: &mut QueryContext, not_found_error: bool) -> GrpcResult<()> { 
        let mut new_require_types = HashSet::new();
        swap(&mut new_require_types, &mut context.require_types);
        
        for t in new_require_types.into_iter() {
            match self.query_type(&t, context) {
                Some(v) => {context.relate.insert(t, v);},
                None => {
                    if not_found_error {
                        return Err(GrpcError::TypeNotFound(t));
                    } else {
                        context.require_types.insert(t);
                    }
                }
            };
        }
        Ok(())
    }

    pub async fn rpc_info(&self, service: &str, method_name: &str) -> GrpcResult<RpcInfo> {
        let rpc_name = format!("{}.{}", service, method_name);
        let symbols = self.query_symbols(std::iter::once(rpc_name.to_owned())).await?;
        let map = self.make_types(&symbols);
        let relate = HashMap::new();
        let require_types = HashSet::new();

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
                    let mut path = Vec::new();
                    path.extend(file.package().split('.'));
                    let mut context = QueryContext {
                        map,
                        relate,
                        path: Vec::new(),
                        require_types,
                    };
                    log::info!("req {} rsp {}", method.input_type(), method.output_type());
                    context.require_types.insert(method.input_type().to_owned());
                    context.require_types.insert(method.output_type().to_owned());

                    self.consume_types(&mut context, false)?;

                    while !context.require_types.is_empty() {
                        self.query_and_parse_types(&mut context).await?;
                    }

                    return Ok(RpcInfo {
                        name: method_name.to_owned(),
                        full_name: rpc_name,
                        request_stream: method.client_streaming(),
                        response_stream: method.server_streaming(),
                        request_typename: method.input_type().to_owned(),
                        response_typename: method.output_type().to_owned(),
                        relate_schema: context.relate,
                    });
                }
            }
        }
        Err(GrpcError::RpcNotFound(rpc_name.to_owned()))
    }

    pub async fn invoke(&mut self, request: &[u8]) -> GrpcResult<Vec<u8>> {
        todo!()
    }
}
