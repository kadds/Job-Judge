#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("connect etcd error")]
    ConnectionFailed,
    #[error("resource limit error")]
    ResourceLimit,
    #[error("op error")]
    OperationError(#[from] etcd_rs::Error),
    #[error("unknown error")]
    Unknown,
}

pub type Result<T> = std::result::Result<T, Error>;

use crate::log;
use backtrace::Backtrace;

pub fn panic_hook (){
    std::panic::set_hook(Box::new(|info| {
        std::thread::sleep(std::time::Duration::from_secs(1)); 
        let bt = Backtrace::new();
        let payload = info.payload();
        loop {
            if let Some(s) = payload.downcast_ref::<&str>() {
                error!("panic occurred: {:?}\n{:?}", s, bt);
                break;
            }
            if let Some(s) = info.payload().downcast_ref::<String>() {
                error!("panic occurred: {:?}\n{:?}", s, bt);
                break;
            } 
            error!("panic occurred.\n{:?}", bt);
            break;
        }
       std::thread::sleep(std::time::Duration::from_secs(3)); 
    }));
}