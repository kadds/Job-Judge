use bytes::{Buf, BufMut};
use prost::encoding::{DecodeContext, WireType};
use prost::{DecodeError, Message};
use prost_types::*;
use serde::{Deserialize, Serialize};
use std::option::Option;
use std::{collections::HashMap, sync::Arc};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "relate")]
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
pub enum Label {
    Optional,
    Required,
    Repeated,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Field {
    pub ktype: Type,
    pub name: String,
    pub pos: u32,
    pub deprecated: bool,
    pub packed: bool,
    pub label: Label,
    #[serde(skip_serializing_if = "std::option::Option::is_none")]
    pub oneof_index: Option<usize>,
    pub json_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnumField {
    pub name: String,
    pub pos: i32,
    pub deprecated: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageType {
    pub name: String,
    pub fields: Vec<Field>,

    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub oneofs: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EnumType {
    pub name: String,
    pub enums: Vec<EnumField>,
}

#[derive(Debug, Clone)]
pub enum CommonTypeProto {
    Message(DescriptorProto),
    Enum(EnumDescriptorProto),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum CommonType {
    Message(MessageType),
    Enum(EnumType),
}

/// ========== any message =============

#[derive(Debug, Clone, Default)]
pub struct AnyMessageContext {
    relate_message: Option<Arc<HashMap<String, CommonType>>>,
}

impl AnyMessageContext {
    pub fn new(relate: Arc<HashMap<String, CommonType>>) -> Self {
        Self {
            relate_message: Some(relate),
        }
    }
}

impl AnyMessageContext {
    pub fn relate(&self, name: &str) -> Option<&CommonType> {
        self.relate_message.as_ref().and_then(|v| v.get(name))
    }
    pub fn relate_message(&self, name: &str) -> Option<&MessageType> {
        self.relate(name).and_then(|v| match v {
            CommonType::Message(msg) => Some(msg),
            CommonType::Enum(_) => None,
        })
    }
    pub fn relate_enum(&self, name: &str) -> Option<&EnumType> {
        self.relate(name).and_then(|v| match v {
            CommonType::Message(_) => None,
            CommonType::Enum(e) => Some(e),
        })
    }
}

#[derive(Error, Debug)]
pub enum EncodeError {
    #[error("field '{0}' type expect {1}")]
    TypeMismatch(String, &'static str),
    #[error("field '{0}' is required")]
    Required(String),
    #[error("invalid type in field '{0}'")]
    InvalidType(String),
    #[error("base64 decode in field '{field}' errors: {inner}")]
    Base64DecodeFail {
        field: String,
        inner: base64::DecodeError,
    },
}

type EncodeResult = std::result::Result<(), EncodeError>;

#[derive(Debug)]
pub struct AnyMessage {
    value: serde_json::Value,
    ctx: AnyMessageContext,
    msg_name: Option<String>,
}

#[derive(Debug)]
pub struct SubMessage<'a> {
    value: &'a serde_json::Value,
    message: &'a MessageType,
    ctx: AnyMessageContext,
}

#[derive(Debug)]
pub struct SubMessageMut<'a> {
    value: &'a mut serde_json::Value,
    message: &'a MessageType,
    ctx: AnyMessageContext,
}

macro_rules! join_u8str {
    ($stack:ident) => {
        unsafe {
            $stack
                .iter()
                .map(|v| std::str::from_utf8_unchecked(std::slice::from_raw_parts(v.0, v.1)))
                .collect::<Vec<&str>>()
                .join("/")
        }
    };
}

macro_rules! ret_type_mismatch {
    ($t:tt, $stack:ident) => {
        return unsafe {
            Err(EncodeError::TypeMismatch(
                $stack
                    .iter()
                    .map(|v| std::str::from_utf8_unchecked(std::slice::from_raw_parts(v.0, v.1)))
                    .collect::<Vec<&str>>()
                    .join("/"),
                $t,
            ))
        }
    };
}
macro_rules! ret_invalid_type {
    ($stack:ident) => {
        return unsafe {
            Err(EncodeError::InvalidType(
                $stack
                    .iter()
                    .map(|v| std::str::from_utf8_unchecked(std::slice::from_raw_parts(v.0, v.1)))
                    .collect::<Vec<&str>>()
                    .join("/"),
            ))
        }
    };
}
macro_rules! ret_required {
    ($stack:ident) => {
        return unsafe {
            Err(EncodeError::Required(
                $stack
                    .iter()
                    .map(|v| std::str::from_utf8_unchecked(std::slice::from_raw_parts(v.0, v.1)))
                    .collect::<Vec<&str>>()
                    .join("/"),
            ))
        }
    };
}

impl<'a> SubMessage<'a> {
    fn child(&self, message: &'a MessageType, value: &'a serde_json::Value) -> SubMessage<'a> {
        Self {
            value,
            message,
            ctx: self.ctx.clone(),
        }
    }

    fn fetch_enum_value(&self, enum_name: &str, value: &serde_json::Value) -> Option<i32> {
        if let Some(e) = self.ctx.relate_enum(enum_name) {
            if let Some(val) = value.as_i64().and_then(num::cast::<i64, i32>) {
                e.enums.iter().find(|v| v.pos == val).map(|v| v.pos)
            } else {
                value
                    .as_str()
                    .and_then(|f| e.enums.iter().find(|v| v.name == f))
                    .map(|v| v.pos)
            }
        } else {
            None
        }
    }

    fn check_single_value(
        &self,
        t: &Type,
        value: &serde_json::Value,
        stack: &mut Vec<(*const u8, usize)>,
        ignore_msg: bool,
    ) -> EncodeResult {
        if value.is_null() {
            return Ok(());
        }
        match t {
            Type::Double => {
                if value.as_f64().is_none() {
                    ret_type_mismatch!("double", stack);
                }
            }
            Type::Float => {
                if value.as_f64().and_then(num::cast::<f64, f32>).is_none() {
                    ret_type_mismatch!("float", stack);
                }
            }
            Type::Fixed32 => {
                if value.as_u64().and_then(num::cast::<u64, u32>).is_none() {
                    ret_type_mismatch!("fixed32", stack);
                }
            }
            Type::Fixed64 => {
                if value.as_u64().is_none() {
                    ret_type_mismatch!("fixed64", stack);
                }
            }
            Type::Bool => {
                if value.as_bool().is_none() {
                    ret_type_mismatch!("bool", stack);
                }
            }
            Type::Bytes => {
                if let Some(v) = value.as_str() {
                    base64::decode(v).map_err(|e| EncodeError::Base64DecodeFail {
                        field: join_u8str!(stack),
                        inner: e,
                    })?;
                } else {
                    ret_type_mismatch!("bytes", stack);
                }
            }
            Type::String => {
                if value.as_str().is_none() {
                    ret_type_mismatch!("string", stack);
                }
            }
            Type::Sfixed32 => {
                if value.as_i64().and_then(num::cast::<i64, i32>).is_none() {
                    ret_type_mismatch!("sfixed32", stack);
                }
            }
            Type::Sfixed64 => {
                if value.as_i64().is_none() {
                    ret_type_mismatch!("sfixed64", stack);
                }
            }
            Type::Int32 => {
                if value.as_i64().and_then(num::cast::<i64, i32>).is_none() {
                    ret_type_mismatch!("int32", stack);
                }
            }
            Type::Int64 => {
                if value.as_i64().is_none() {
                    ret_type_mismatch!("int64", stack);
                }
            }
            Type::Uint32 => {
                if value.as_u64().and_then(num::cast::<u64, u32>).is_none() {
                    ret_type_mismatch!("uint32", stack);
                }
            }
            Type::Uint64 => {
                if value.as_u64().is_none() {
                    ret_type_mismatch!("uint64", stack);
                }
            }
            Type::Sint32 => {
                if value.as_i64().and_then(num::cast::<i64, i32>).is_none() {
                    ret_type_mismatch!("suint32", stack);
                }
            }
            Type::Sint64 => {
                if value.as_i64().is_none() {
                    ret_type_mismatch!("sint64", stack);
                }
            }
            Type::Enum(enum_name) => {
                // from enum int or string
                if self.fetch_enum_value(enum_name, value).is_none() {
                    ret_type_mismatch!("enum", stack);
                }
            }
            Type::Message(msg_type) => {
                if !ignore_msg {
                    let msg = match self.ctx.relate_message(msg_type) {
                        Some(t) => t,
                        _ => {
                            ret_invalid_type!(stack);
                        }
                    };
                    let msg = self.child(msg, value);
                    msg.encode_check(stack)?;
                }
            }
            Type::Invalid => ret_invalid_type!(stack),
        }
        Ok(())
    }
    fn repeated_type(&self, field: &Field) -> RepeatedType {
        match field.label {
            Label::Repeated => {
                if field.packed {
                    RepeatedType::Packed
                } else {
                    RepeatedType::Repeated
                }
            }
            _ => RepeatedType::None,
        }
    }

    fn encode_check(&self, stack: &mut Vec<(*const u8, usize)>) -> EncodeResult {
        for field in &self.message.fields {
            stack.push((field.json_name.as_ptr(), field.json_name.len()));

            let t = field.ktype.clone();
            let name = &field.json_name;
            let value = match field.label {
                Label::Optional => match self.value.get(name) {
                    Some(v) => v,
                    None => continue,
                },
                Label::Repeated => match self.value.get(name) {
                    Some(v) => {
                        if v.is_array() || v.is_null() {
                            v
                        } else {
                            ret_type_mismatch!("array", stack);
                        }
                    }
                    None => continue,
                },
                Label::Required => match self.value.get(name) {
                    Some(v) => v,
                    None => {
                        ret_required!(stack);
                    }
                },
            };

            match field.label {
                Label::Repeated => {
                    match t {
                        Type::String | Type::Bytes | Type::Message(_) => {
                            if field.packed {
                                ret_type_mismatch!("nopacked type", stack);
                            }
                        }
                        _ => {}
                    };
                    // get array
                    if value.is_array() {
                        for (idx, value_item) in value.as_array().unwrap().iter().enumerate() {
                            let str = format!("{}", idx);
                            stack.push((str.as_ptr(), str.len()));
                            self.check_single_value(&t, value_item, stack, idx != 0)?;
                            stack.pop();
                        }
                    }
                }
                _ => self.check_single_value(&t, value, stack, false)?,
            }
            stack.pop();
        }
        Ok(())
    }
}

impl<'a> SubMessageMut<'a> {
    fn child(
        &self,
        message: &'a MessageType,
        value: &'a mut serde_json::Value,
    ) -> SubMessageMut<'a> {
        Self {
            value,
            message,
            ctx: self.ctx.clone(),
        }
    }
    fn repeated_type(&self, field: &Field) -> RepeatedType {
        match field.label {
            Label::Repeated => {
                if field.packed {
                    RepeatedType::Packed
                } else {
                    RepeatedType::Repeated
                }
            }
            _ => RepeatedType::None,
        }
    }
}
macro_rules! wrap_encoded_len {
    ($rt: expr, $value: expr, $tag: expr, $s: ident, $f: ident) => {
        match $rt {
            RepeatedType::None | RepeatedType::Repeated => {
                wrap_encoded_len_nopack!($rt, $value, $tag, $s, $f)
            }
            RepeatedType::Packed => {
                if !$value.is_null() {
                    let arr = $value.as_array().unwrap();
                    let k = arr.iter().filter_map($f).collect::<Vec<_>>();
                    prost::encoding::$s::encoded_len_packed($tag, &k)
                } else {
                    0
                }
            }
        }
    };
}

macro_rules! wrap_encoded_len_nopack {
    ($rt: expr, $value: expr, $tag: expr, $s: ident, $f: ident) => {
        match $rt {
            RepeatedType::None => {
                if let Some(val) = $f($value) {
                    prost::encoding::$s::encoded_len($tag, &val)
                } else {
                    0
                }
            }
            RepeatedType::Repeated => {
                if !$value.is_null() {
                    let arr = $value.as_array().unwrap();
                    let k = arr.iter().filter_map($f).collect::<Vec<_>>();
                    prost::encoding::$s::encoded_len_repeated($tag, &k)
                } else {
                    0
                }
            }
            RepeatedType::Packed => 0,
        }
    };
}

macro_rules! wrap_encode {
    ($rt: expr, $value: expr, $tag: expr, $s: ident, $f: ident, $buf: expr) => {
        match $rt {
            RepeatedType::None | RepeatedType::Repeated => {
                wrap_encode_nopack!($rt, $value, $tag, $s, $f, $buf)
            }
            RepeatedType::Packed => {
                if !$value.is_null() {
                    let arr = $value.as_array().unwrap();
                    let k = arr.iter().filter_map($f).collect::<Vec<_>>();
                    prost::encoding::$s::encode_packed($tag, &k, $buf);
                }
            }
        }
    };
}

macro_rules! wrap_encode_nopack {
    ($rt: expr, $value: expr, $tag: expr, $s: ident, $f: ident, $buf: expr) => {
        match $rt {
            RepeatedType::None => {
                if let Some(val) = $f($value) {
                    prost::encoding::$s::encode($tag, &val, $buf)
                }
            }
            RepeatedType::Repeated => {
                if !$value.is_null() {
                    let arr = $value.as_array().unwrap();
                    let k = arr.iter().filter_map($f).collect::<Vec<_>>();
                    prost::encoding::$s::encode_repeated($tag, &k, $buf)
                }
            }
            RepeatedType::Packed => (),
        }
    };
}

macro_rules! wrap_merge {
    ($rt: expr, $s: ident, $f: ident, $wire_type: expr,$buf: expr, $ctx: expr) => {
        match $rt {
            RepeatedType::None => {
                let mut val = Default::default();
                if let Err(e) = prost::encoding::$s::merge($wire_type, &mut val, $buf, $ctx) {
                    Err(e)
                } else {
                    $f(val)
                }
            }
            _ => {
                let mut val = Vec::new();
                if let Err(e) =
                    prost::encoding::$s::merge_repeated($wire_type, &mut val, $buf, $ctx)
                {
                    Err(e)
                } else {
                    val.into_iter()
                        .map($f)
                        .collect::<std::result::Result<Vec<_>, DecodeError>>()
                        .map(serde_json::Value::Array)
                }
            }
        }
    };
}

enum RepeatedType {
    None,
    Repeated,
    Packed,
}

#[inline]
fn v_as_f64(value: &serde_json::Value) -> Option<f64> {
    value.as_f64()
}
#[inline]
fn v_as_f32(value: &serde_json::Value) -> Option<f32> {
    value.as_f64().and_then(num::cast)
}
#[inline]
fn v_as_u64(value: &serde_json::Value) -> Option<u64> {
    value.as_u64()
}
#[inline]
fn v_as_u32(value: &serde_json::Value) -> Option<u32> {
    value.as_u64().and_then(num::cast)
}
#[inline]
fn v_as_i64(value: &serde_json::Value) -> Option<i64> {
    value.as_i64()
}
#[inline]
fn v_as_i32(value: &serde_json::Value) -> Option<i32> {
    value.as_i64().and_then(num::cast)
}
#[inline]
fn v_as_bool(value: &serde_json::Value) -> Option<bool> {
    value.as_bool()
}
#[inline]
fn v_as_bytes(value: &serde_json::Value) -> Option<::bytes::Bytes> {
    value
        .as_str()
        .and_then(|v| base64::decode(v).ok())
        .map(::bytes::Bytes::from)
}
#[inline]
fn v_as_string(value: &serde_json::Value) -> Option<String> {
    value.as_str().map(|v| v.to_owned())
}

impl<'a> Message for SubMessage<'a> {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        for field in &self.message.fields {
            let value = match self.value.get(&field.json_name) {
                Some(v) => v,
                None => continue,
            };
            let tag = field.pos;
            let t = field.ktype.clone();
            let rt = self.repeated_type(field);
            if value.is_null() {
                continue;
            }

            match t {
                Type::Double => {
                    wrap_encode!(rt, value, tag, double, v_as_f64, buf)
                }
                Type::Float => {
                    wrap_encode!(rt, value, tag, float, v_as_f32, buf)
                }
                Type::Fixed32 => {
                    wrap_encode!(rt, value, tag, fixed32, v_as_u32, buf)
                }
                Type::Fixed64 => {
                    wrap_encode!(rt, value, tag, fixed64, v_as_u64, buf)
                }
                Type::Bool => {
                    wrap_encode!(rt, value, tag, bool, v_as_bool, buf)
                }
                Type::Bytes => wrap_encode_nopack!(rt, value, tag, bytes, v_as_bytes, buf),
                Type::String => {
                    wrap_encode_nopack!(rt, value, tag, string, v_as_string, buf)
                }
                Type::Sfixed32 => {
                    wrap_encode!(rt, value, tag, sfixed32, v_as_i32, buf)
                }
                Type::Sfixed64 => {
                    wrap_encode!(rt, value, tag, sfixed64, v_as_i64, buf)
                }
                Type::Int32 => {
                    wrap_encode!(rt, value, tag, int32, v_as_i32, buf)
                }
                Type::Int64 => {
                    wrap_encode!(rt, value, tag, int64, v_as_i64, buf)
                }
                Type::Uint32 => {
                    wrap_encode!(rt, value, tag, uint32, v_as_u32, buf)
                }
                Type::Uint64 => {
                    wrap_encode!(rt, value, tag, uint64, v_as_u64, buf)
                }
                Type::Sint32 => {
                    wrap_encode!(rt, value, tag, int32, v_as_i32, buf)
                }
                Type::Sint64 => {
                    wrap_encode!(rt, value, tag, int64, v_as_i64, buf)
                }
                Type::Enum(enum_name) => {
                    let v_as_enum = |v: &serde_json::Value| self.fetch_enum_value(&enum_name, v);
                    wrap_encode!(rt, value, tag, int32, v_as_enum, buf)
                }
                Type::Message(msg_type) => {
                    let msg = self.ctx.relate_message(&msg_type).unwrap();
                    let v_as_msg = |v: &'a serde_json::Value| {
                        let msg = self.child(msg, v);
                        Some(msg)
                    };
                    wrap_encode_nopack!(rt, value, tag, message, v_as_msg, buf)
                }
                Type::Invalid => {
                    panic!("invalid type");
                }
            };
        }
    }

    fn merge_field<B>(
        &mut self,
        _: u32,
        _: WireType,
        _: &mut B,
        _: DecodeContext,
    ) -> Result<(), DecodeError>
    where
        B: Buf,
        Self: Sized,
    {
        panic!("not available")
    }

    fn encoded_len(&self) -> usize {
        let mut len = 0;
        for field in &self.message.fields {
            let tag = field.pos;
            let t = field.ktype.clone();
            let name = &field.json_name;
            let value = match self.value.get(name) {
                Some(v) => v,
                None => continue,
            };
            let rt = self.repeated_type(field);
            if value.is_null() {
                continue;
            }

            let field_len = match t {
                Type::Double => {
                    wrap_encoded_len!(rt, value, tag, double, v_as_f64)
                }
                Type::Float => {
                    wrap_encoded_len!(rt, value, tag, float, v_as_f32)
                }
                Type::Fixed32 => {
                    wrap_encoded_len!(rt, value, tag, fixed32, v_as_u32)
                }
                Type::Fixed64 => {
                    wrap_encoded_len!(rt, value, tag, fixed64, v_as_u64)
                }
                Type::Bool => {
                    wrap_encoded_len!(rt, value, tag, bool, v_as_bool)
                }
                Type::Bytes => {
                    wrap_encoded_len_nopack!(rt, value, tag, bytes, v_as_bytes)
                }
                Type::String => {
                    wrap_encoded_len_nopack!(rt, value, tag, string, v_as_string)
                }
                Type::Sfixed32 => {
                    wrap_encoded_len!(rt, value, tag, sfixed32, v_as_i32)
                }
                Type::Sfixed64 => {
                    wrap_encoded_len!(rt, value, tag, sfixed64, v_as_i64)
                }
                Type::Int32 => {
                    wrap_encoded_len!(rt, value, tag, int32, v_as_i32)
                }
                Type::Int64 => {
                    wrap_encoded_len!(rt, value, tag, int64, v_as_i64)
                }
                Type::Uint32 => {
                    wrap_encoded_len!(rt, value, tag, uint32, v_as_u32)
                }
                Type::Uint64 => {
                    wrap_encoded_len!(rt, value, tag, uint64, v_as_u64)
                }
                Type::Sint32 => {
                    wrap_encoded_len!(rt, value, tag, sint32, v_as_i32)
                }
                Type::Sint64 => {
                    wrap_encoded_len!(rt, value, tag, sint64, v_as_i64)
                }
                Type::Enum(enum_name) => {
                    let v_as_enum = |v: &serde_json::Value| self.fetch_enum_value(&enum_name, v);
                    wrap_encoded_len!(rt, value, tag, int32, v_as_enum)
                }
                Type::Message(msg_type) => {
                    let msg = self.ctx.relate_message(&msg_type).unwrap();
                    let v_as_msg = |v: &'a serde_json::Value| {
                        let msg = self.child(msg, v);
                        Some(msg)
                    };
                    wrap_encoded_len_nopack!(rt, value, tag, message, v_as_msg)
                }
                Type::Invalid => {
                    panic!("invalid type");
                }
            };
            len += field_len;
        }
        len
    }

    fn clear(&mut self) {
        panic!("not available")
    }
}

type DecodeResult = std::result::Result<serde_json::Value, DecodeError>;
#[inline]
fn f64_to_v(f: f64) -> DecodeResult {
    serde_json::Number::from_f64(f)
        .ok_or_else(|| DecodeError::new(""))
        .map(serde_json::Value::Number)
}
#[inline]
fn f32_to_v(f: f32) -> DecodeResult {
    serde_json::Number::from_f64(f as f64)
        .ok_or_else(|| DecodeError::new(""))
        .map(serde_json::Value::Number)
}
#[inline]
fn i64_to_v(f: i64) -> DecodeResult {
    Ok(serde_json::Value::Number(serde_json::Number::from(f)))
}
#[inline]
fn i32_to_v(f: i32) -> DecodeResult {
    Ok(serde_json::Value::Number(serde_json::Number::from(f)))
}
#[inline]
fn u64_to_v(f: u64) -> DecodeResult {
    Ok(serde_json::Value::Number(serde_json::Number::from(f)))
}
#[inline]
fn u32_to_v(f: u32) -> DecodeResult {
    Ok(serde_json::Value::Number(serde_json::Number::from(f)))
}
#[inline]
fn bool_to_v(f: bool) -> DecodeResult {
    Ok(serde_json::Value::Bool(f))
}
#[inline]
fn bytes_to_v(bytes: ::bytes::Bytes) -> DecodeResult {
    Ok(serde_json::Value::String(base64::encode(bytes)))
}
#[inline]
fn string_to_v(str: String) -> DecodeResult {
    Ok(serde_json::Value::String(str))
}

impl<'a> Message for SubMessageMut<'a> {
    fn encode_raw<B>(&self, _: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        panic!("not available")
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
        use prost::encoding::*;
        use serde_json::*;
        if let Some(field) = self.message.fields.iter().find(|v| v.pos == tag) {
            if !self.value.is_object() {
                *self.value = serde_json::Value::Object(serde_json::Map::new());
            }
            let t = field.ktype.clone();
            let rt = self.repeated_type(field);
            let mut value = match t {
                Type::Double => wrap_merge!(rt, double, f64_to_v, wire_type, buf, ctx)?,
                Type::Float => wrap_merge!(rt, float, f32_to_v, wire_type, buf, ctx)?,
                Type::Fixed32 => wrap_merge!(rt, fixed32, u32_to_v, wire_type, buf, ctx)?,
                Type::Fixed64 => wrap_merge!(rt, fixed64, u64_to_v, wire_type, buf, ctx)?,
                Type::Bool => wrap_merge!(rt, bool, bool_to_v, wire_type, buf, ctx)?,
                Type::Bytes => wrap_merge!(rt, bytes, bytes_to_v, wire_type, buf, ctx)?,
                Type::String => wrap_merge!(rt, string, string_to_v, wire_type, buf, ctx)?,
                Type::Sfixed32 => wrap_merge!(rt, sfixed32, i32_to_v, wire_type, buf, ctx)?,
                Type::Sfixed64 => wrap_merge!(rt, sfixed64, i64_to_v, wire_type, buf, ctx)?,
                Type::Int32 => wrap_merge!(rt, int32, i32_to_v, wire_type, buf, ctx)?,
                Type::Int64 => wrap_merge!(rt, int64, i64_to_v, wire_type, buf, ctx)?,
                Type::Uint32 => wrap_merge!(rt, uint32, u32_to_v, wire_type, buf, ctx)?,
                Type::Uint64 => wrap_merge!(rt, uint64, u64_to_v, wire_type, buf, ctx)?,
                Type::Sint32 => wrap_merge!(rt, sint32, i32_to_v, wire_type, buf, ctx)?,
                Type::Sint64 => wrap_merge!(rt, sint64, i64_to_v, wire_type, buf, ctx)?,
                Type::Enum(enum_name) => {
                    let enum_to_v = |pos: i32| {
                        let s = if let Some(e) = self.ctx.relate_enum(&enum_name) {
                            e.enums
                                .iter()
                                .find(|v| v.pos == pos)
                                .map_or_else(|| pos.to_string(), |v| v.name.clone())
                        } else {
                            pos.to_string()
                        };
                        Ok(serde_json::Value::String(s))
                    };
                    wrap_merge!(rt, int32, enum_to_v, wire_type, buf, ctx)?
                }
                Type::Message(msg_type) => {
                    let msg = match self.ctx.relate_message(&msg_type) {
                        Some(t) => t,
                        _ => {
                            return Err(DecodeError::new(format!(
                                "not found message {}",
                                msg_type
                            )));
                        }
                    };
                    let mut value = Value::Object(Map::new());
                    let mut msg = self.child(msg, &mut value);
                    let _ = message::merge(wire_type, &mut msg, buf, ctx)?;
                    value
                }
                Type::Invalid => {
                    panic!("invalid type");
                }
            };
            let obj = self.value.as_object_mut().unwrap();
            let mut has_set = false;
            if value.is_array() {
                if let Some(v) = obj.get_mut(&field.json_name) {
                    if let Some(v) = v.as_array_mut() {
                        v.append(value.as_array_mut().unwrap());
                        has_set = true;
                    }
                }
            }
            if !has_set {
                obj.insert(field.json_name.to_owned(), value);
            }
        }
        Ok(())
    }

    fn encoded_len(&self) -> usize {
        panic!("not available")
    }

    fn clear(&mut self) {}
}

impl AnyMessage {
    pub fn new_decode(ctx: AnyMessageContext) -> Self {
        Self {
            value: serde_json::Value::Object(serde_json::Map::new()),
            msg_name: None,
            ctx,
        }
    }

    pub fn value(self) -> serde_json::Value {
        self.value
    }

    pub fn new_encode(value: serde_json::Value, ctx: AnyMessageContext) -> Self {
        Self {
            value,
            msg_name: None,
            ctx,
        }
    }

    pub fn set_message_target(&mut self, name: String) {
        self.msg_name = Some(name);
    }

    fn root(&self) -> SubMessage {
        SubMessage {
            message: self
                .ctx
                .relate_message(self.msg_name.as_ref().unwrap())
                .unwrap(),
            value: &self.value,
            ctx: self.ctx.clone(),
        }
    }
    fn root_mut(&mut self) -> SubMessageMut {
        SubMessageMut {
            message: self
                .ctx
                .relate_message(self.msg_name.as_ref().unwrap())
                .unwrap(),
            value: &mut self.value,
            ctx: self.ctx.clone(),
        }
    }
    pub fn encode_check(&self) -> EncodeResult {
        let mut stack = vec![("".as_ptr(), 0)];
        self.root().encode_check(&mut stack)
    }
}

impl Message for AnyMessage {
    fn encode_raw<B>(&self, buf: &mut B)
    where
        B: BufMut,
        Self: Sized,
    {
        let message = self.root();
        message.encode_raw(buf);
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
        let mut message = self.root_mut();
        message.merge_field(tag, wire_type, buf, ctx)
    }

    fn encoded_len(&self) -> usize {
        let message = self.root();
        message.encoded_len()
    }

    fn clear(&mut self) {
        self.value = serde_json::Value::Object(serde_json::Map::new());
    }
}

#[derive(Debug, Clone, Default)]
pub struct AnyProstCodec {
    ctx: AnyMessageContext,
    encode_msg_name: String,
    decode_msg_name: String,
}

impl AnyProstCodec {
    pub fn new(encode_msg_name: String, decode_msg_name: String, ctx: AnyMessageContext) -> Self {
        Self {
            encode_msg_name,
            decode_msg_name,
            ctx,
        }
    }
}

impl tonic::codec::Codec for AnyProstCodec {
    type Encode = AnyMessage;

    type Decode = AnyMessage;

    type Encoder = AnyProstEncoder;

    type Decoder = AnyProstDecoder;

    fn encoder(&mut self) -> Self::Encoder {
        AnyProstEncoder {
            _ctx: self.ctx.clone(),
            msg_name: self.encode_msg_name.clone(),
        }
    }

    fn decoder(&mut self) -> Self::Decoder {
        AnyProstDecoder {
            ctx: self.ctx.clone(),
            msg_name: self.decode_msg_name.clone(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AnyProstEncoder {
    _ctx: AnyMessageContext,
    msg_name: String,
}

impl tonic::codec::Encoder for AnyProstEncoder {
    type Item = AnyMessage;

    type Error = tonic::Status;

    fn encode(
        &mut self,
        mut item: Self::Item,
        buf: &mut tonic::codec::EncodeBuf<'_>,
    ) -> Result<(), Self::Error> {
        item.set_message_target(self.msg_name.clone());
        item.encode(buf)
            .expect("Message only errors if not enough space");

        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct AnyProstDecoder {
    ctx: AnyMessageContext,
    msg_name: String,
}

impl tonic::codec::Decoder for AnyProstDecoder {
    type Item = AnyMessage;
    type Error = tonic::Status;

    fn decode(
        &mut self,
        buf: &mut tonic::codec::DecodeBuf<'_>,
    ) -> Result<Option<Self::Item>, Self::Error> {
        let mut message = AnyMessage::new_decode(self.ctx.clone());
        message.set_message_target(self.msg_name.clone());
        let item = message
            .merge(buf)
            .map(|_| message)
            .map(Option::Some)
            .map_err(from_decode_error)?;

        Ok(item)
    }
}

fn from_decode_error(error: prost::DecodeError) -> tonic::Status {
    tonic::Status::new(tonic::Code::Internal, error.to_string())
}
