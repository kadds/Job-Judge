#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("connect etcd error")]
    ConnectionFailed,
    #[error("resource limit error")]
    ResourceLimit,
    #[error("unknown error")]
    Unknown,
}

#[derive(thiserror::Error, Debug)]
pub enum InitConfigError {
    #[error("parameter is empty")]
    EmptyConfigField,
    #[error("parse string parameter fail")]
    ParseParameterFail,
}

pub type Result<T> = std::result::Result<T, Error>;
