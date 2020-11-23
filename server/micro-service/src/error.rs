#[derive(Debug)]
pub enum Error {
    ConnectionFailed,
    Timeout,
    CheckFailed,
    ResourceLimit,
    Unknown,
}

pub type Result<T> = std::result::Result<T, Error>;
use super::log;
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