use crate::error::InitConfigError;
use log::*;
use std::sync::Arc;

#[derive(Debug)]
pub struct Database {
    pub url: String,
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
    pub file: String,
    pub dns_template: String,
}

#[derive(Debug)]
pub struct MicroServiceConfig {
    pub comm_database: Database,
    pub bind_port: u16,
    pub discover: Discover,
    pub meta: MicroServiceMetaConfig,
    pub session_key: String,
}

pub fn init_from_env() -> Result<Arc<MicroServiceConfig>, InitConfigError> {
    let mut comm_database_url = "".to_owned();
    let mut bind_port = 11100;
    let mut module = "UNKNOWN".to_owned();
    let mut name = "UNKNOWN".to_owned();
    let mut level = ServiceLevel::Prod;
    let mut ip = "localhost".to_owned();
    let mut dns_template = "{}.local".to_owned();
    let mut file = "".to_owned();
    let mut ttl = 60;
    let mut session_key = "".to_owned();

    for (k, v) in std::env::vars() {
        match k.as_str() {
            "JJ_DISCOVER_TTL" => match v.parse() {
                Ok(v) => ttl = v,
                Err(e) => {
                    error!("parse {}={} fail, error: {}", k, v, e);
                    return Err(InitConfigError::ParseParameterFail);
                }
            },
            "JJ_DISCOVER_FILE" => file = v,
            "JJ_DISCOVER_DNS_TEMPLATE" => dns_template = v,
            "JJ_COMM_DATABASE_URL" => comm_database_url = v,
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
            "JJ_SESSION_KEY" => session_key = v,
            _ => {}
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
            dns_template,
        },
        meta: MicroServiceMetaConfig {
            module,
            name,
            level,
            ip,
        },
        session_key,
    }))
}
