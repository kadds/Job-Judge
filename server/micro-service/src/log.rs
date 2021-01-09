use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::sync::mpsc;
use tokio::time::{delay_for, Duration};
use std::future::Future;
use std::sync::Arc;

static mut QUEUE: Option<mpsc::Sender<String>> = None;

use async_trait::async_trait;

#[async_trait]
trait LogSender {
    async fn do_send(&self, tx: mpsc::Sender<String>, rx: mpsc::Receiver<String>);
}

struct TcpLogger {
    address: String,
}

#[async_trait]
impl LogSender for TcpLogger {
    async fn do_send(&self, tx: mpsc::Sender<String>, rx: mpsc::Receiver<String>) {
        let mut rx = rx;
        let mut tx = tx;
        loop {
            let mut conn = match TcpStream::connect(self.address.clone()).await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("{}", e);
                    delay_for(Duration::from_secs(5)).await;
                    continue;
                }
            };
            while let Some(log) = rx.recv().await {
                if let Err(_) = conn.write_all(log.as_bytes()).await {
                    let _ = tx.send(log).await;
                    break;
                }
                let _ = conn.write_all(b"\x04").await;
            }
        }
    }
}

struct ConsoleLogger {}

#[async_trait]
impl LogSender for ConsoleLogger {
    async fn do_send(&self, tx: mpsc::Sender<String>, rx: mpsc::Receiver<String>) {
        let mut rx = rx;
        let _ = tx;
        while let Some(log) = rx.recv().await {
            print!("{}\n", log);
        }
    }
}

#[tokio::main(core_threads = 1, max_threads = 1)]
async fn tcp_logger_main(address: String, tx: mpsc::Sender<String>, rx: mpsc::Receiver<String>) {
    let logger = TcpLogger { address };
    logger.do_send(tx, rx).await;
}

pub fn init_tcp_logger(address: String) {
    let (tx, rx) = mpsc::channel(10000);
    unsafe {
        assert!(QUEUE.is_none());
        QUEUE = Some(tx.clone());
    }
    std::thread::spawn(|| tcp_logger_main(address, tx, rx));
}

#[tokio::main(core_threads = 1, max_threads = 1)]
async fn console_logger_main(tx: mpsc::Sender<String>, rx: mpsc::Receiver<String>) {
    let logger = ConsoleLogger {};
    logger.do_send(tx, rx).await;
}

pub fn init_console_logger() {
    let (tx, rx) = mpsc::channel(10000);
    unsafe {
        assert!(QUEUE.is_none());
        QUEUE = Some(tx.clone());
    }

    std::thread::spawn(|| console_logger_main(tx, rx));
}

pub fn send_log(log: String) {
    unsafe {
        if let Some(q) = &mut QUEUE {
            let _ = q.try_send(log);
        }
    }
}

#[derive(Clone)]
pub struct LogContext{
    pub vid: u64,
    pub tid: u64,
    pub nid: u64,
    pub pnid: u64,
    pub server_name: Arc<String>,
}

tokio::task_local! {
    pub static LOG_CONTEXT: LogContext;
}

pub async fn make_context<T, F>(vid: u64, tid: u64, nid: u64, pnid: u64, server_name: Arc<String>, future: F) -> T
    where
        F: Future<Output = T>
{
    LOG_CONTEXT.scope(LogContext{
        vid,
        tid,
        nid,
        pnid,
        server_name
    }, future).await
}

pub async fn make_empty_context<T, F>(server_name: Arc<String>, future: F) -> T 
    where
        F: Future<Output = T>
{
    LOG_CONTEXT.scope(LogContext{
        vid: 0,
        tid: 0,
        nid: 0,
        pnid: 0,
        server_name
    }, future).await
}

#[macro_export]
macro_rules! early_log {
    ($level: tt, $server: tt, $($log: tt)+)=> {
        $crate::log::send_log(format!(
            "0 0 {} 0 {} {} {} [{}:{}:{}:{}]",
            $crate::util::current_ts(),
            $server,
            $level,
            std::format_args!($($log)+),
            std::module_path!(),
            std::file!(),
            std::line!(),
            std::column!(),
        ))
    }
}

#[macro_export]
macro_rules! early_log_error {
    ($($log: tt)+) => {
        early_log!("error", $($log)+);
    };
}

#[macro_export]
macro_rules! early_log_warn {
    ($($log: tt)+) => {
        early_log!("warn", $($log)+);
    };
}

#[macro_export]
macro_rules! early_log_info {
    ($($log: tt)+) => {
        early_log!("info", $($log)+);
    };
}

#[macro_export]
macro_rules! early_log_debug {
    ($($log: tt)+) => {
        early_log!("debug", $($log)+);
    };
}


#[macro_export]
macro_rules! log {
    ($level:expr, $($log:tt)+) => {
        match $crate::log::LOG_CONTEXT.try_with(|v| v.clone()) {
            Ok(v) => {
                $crate::log::send_log(format!(
                    "0 {} {} {} {} {} {} [{}:{}:{}:{}]",
                    v.vid,
                    $crate::util::current_ts(),
                    v.tid,
                    &v.server_name,
                    $level,
                    std::format_args!($($log)+),
                    std::module_path!(),
                    std::file!(),
                    std::line!(),
                    std::column!(),
                ))
            },
            Err(_) => {
                $crate::log::send_log(format!(
                    "0 0 {} 0 UNKNOWN {} {} [{}:{}:{}:{}]",
                    $crate::util::current_ts(),
                    $level,
                    std::format_args!($($log)+),
                    std::module_path!(),
                    std::file!(),
                    std::line!(),
                    std::column!(),
                ))
            }
        }
    };
}

#[macro_export]
macro_rules! error {
    ($($log: tt)+) => {
        log!("error", $($log)+);
    };
}

#[macro_export]
macro_rules! warn {
    ($($log: tt)+) => {
        log!("warn", $($log)+);
    };
}

#[macro_export]
macro_rules! info {
    ($($log: tt)+) => {
        log!("info", $($log)+);
    };
}

#[macro_export]
macro_rules! debug {
    ($($log: tt)+) => {
        log!("debug", $($log)+);
    };
}

#[macro_export]
macro_rules! click_log {
    ($ts: tt, $cost: tt, $method: tt, $url: tt, $host: tt, $return_code: tt, $return_length: tt) => {
        match $crate::log::LOG_CONTEXT.try_with(|v| v.clone()) {
            Ok(v) => {
                $crate::log::send_log(format!(
                    "1 {} {} {} {} {} {} {} {} {} {} {}",
                    v.vid,
                    $ts,
                    v.tid,
                    v.nid,
                    &v.server_name,
                    $cost,
                    $method,
                    $url,
                    $host,
                    $return_code,
                    $return_length
                ))
            },
            Err(_) => {
                $crate::log::send_log(format!(
                    "1 0 {} 0 0 UNKNOWN {} {} {} {} {} {}",
                    $ts,
                    $cost,
                    $method,
                    $url,
                    $host,
                    $return_code,
                    $return_length
                ))
            }
        }
    };
}

#[macro_export]
macro_rules! rpc_log {
    () => {};
}
