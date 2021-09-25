use crate::error::InitConfigError;
use log::*;
use std::sync::Arc;

#[derive(Debug)]
pub struct Database {
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceLevel {
    Test,
    Pre,
    Prod,
}

#[derive(Debug)]
pub struct MicroServiceMetaConfig {
    pub module: String,
    pub name: String,
    pub level: ServiceLevel,
    pub ip: String,
}

#[derive(Debug)]
pub struct Discover {
    pub ttl: u32,
    pub file: Option<String>,
    pub suffix: String,
    pub name_server: String,
}

#[derive(Debug)]
pub struct MicroServiceConfig {
    pub comm_database: Database,
    pub bind_port: u16,
    pub discover: Discover,
    pub meta: MicroServiceMetaConfig,
    pub session_key: Option<String>,
    pub replica_id: Option<u32>,
}

pub fn init_from_env() -> Result<Arc<MicroServiceConfig>, InitConfigError> {
    let mut comm_database_url = None;
    let mut bind_port = 11100;
    let mut module = "UNKNOWN".to_owned();
    let mut name = "UNKNOWN".to_owned();
    let mut level = ServiceLevel::Prod;
    let mut ip = "localhost".to_owned();
    let mut suffix = "cluster.local".to_owned();
    let mut name_server = "".to_owned();
    let mut file = None;
    let mut ttl = 60;
    let mut session_key = None;
    let mut replica_id = None;

    for (k, v) in std::env::vars() {
        match k.as_str() {
            "JJ_DISCOVER_TTL" => match v.parse() {
                Ok(v) => ttl = v,
                Err(e) => {
                    error!("parse {}={} fail, error: {}", k, v, e);
                    return Err(InitConfigError::ParseParameterFail);
                }
            },
            "JJ_DISCOVER_FILE" => file = Some(v),
            "JJ_DISCOVER_NAME_SERVER" => name_server = v,
            "JJ_DISCOVER_SUFFIX" => suffix = v,
            "JJ_COMM_DATABASE_URL" => comm_database_url = Some(v),
            "JJ_BIND_PORT" => match v.parse() {
                Ok(v) => bind_port = v,
                Err(e) => {
                    error!("parse {}={} fail, error: {}", k, v, e);
                    return Err(InitConfigError::ParseParameterFail);
                }
            },
            "JJ_SERVICE_MODULE" => module = v,
            "JJ_SERVICE_NAME" => name = v,
            "JJ_SERVICE_IP" => ip = v,
            "JJ_SERVICE_LEVEL" => {
                level = match v.to_ascii_lowercase().as_str() {
                    "test" | "0" => ServiceLevel::Test,
                    "pre" | "1" => ServiceLevel::Pre,
                    "prod" | "2" => ServiceLevel::Prod,
                    _ => {
                        error!("parse {}={} fail, error: unknown service level type", k, v);
                        return Err(InitConfigError::ParseParameterFail);
                    }
                }
            }
            "JJ_SESSION_KEY" => session_key = Some(v),
            "JJ_REPLICA_ID" => match v.parse() {
                Ok(v) => replica_id = Some(v),
                Err(e) => {
                    error!("parse {}={} fail, error: {}", k, v, e);
                    return Err(InitConfigError::ParseParameterFail);
                }
            },
            _ => {}
        }
    }
    if replica_id.is_none() {
        if let Some(t) = name.split('-').last() {
            match t.parse() {
                Ok(v) => replica_id = Some(v),
                Err(e) => {
                    debug!("parse replica id from server name {} fail, error: {}", name, e);
                }
            }
        }
    }

    if module.is_empty() || name.is_empty() || ip.is_empty() {
        return Err(InitConfigError::EmptyConfigField);
    }

    Ok(Arc::new(MicroServiceConfig {
        comm_database: Database {
            url: comm_database_url,
        },
        bind_port,
        discover: Discover {
            ttl,
            file,
            suffix,
            name_server,
        },
        meta: MicroServiceMetaConfig {
            module,
            name,
            level,
            ip,
        },
        session_key,
        replica_id,
    }))
}
