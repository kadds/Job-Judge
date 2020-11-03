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
        if let Some(s) = info.payload().downcast_ref::<&str>() {
            error!("panic occurred: {:?}\n{:?}", s, bt);
        } else {
            error!("panic occurred.\n{:?}", bt);
        }
       std::thread::sleep(std::time::Duration::from_secs(2)); 
    }));
}