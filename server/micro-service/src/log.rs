use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::sync::mpsc;
use tokio::time::{delay_for, Duration};

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

#[derive(Clone, Copy)]
pub struct LogContext<'a> {
    pub vid: u64,
    pub tid: u64,
    pub nid: u64,
    pub pnid: u64,
    pub server_name: &'a str,
}

tokio::task_local! {
    pub static LOG_CONTEXT: LogContext<'static>;
}

#[macro_export]
macro_rules! log {
    ($level:expr, $($log:tt)+) => {
        let (vid, tid, server_name) = $crate::log::LOG_CONTEXT.try_with(|v| *v).map_or_else(|_| (0, 0, "UNKNOWN"), |ctx| (ctx.vid, ctx.tid, ctx.server_name));
            $crate::log::send_log(format!(
                "0 {} {} {} {} {} {} [{}:{}:{}:{}]",
                vid,
                $crate::tool::current_ts(),
                tid,
                server_name,
                $level,
                std::format_args!($($log)+),
                std::module_path!(),
                std::file!(),
                std::line!(),
                std::column!(),
            ))
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
        let (vid, tid, nid, server_name) = $crate::log::LOG_CONTEXT.try_with(|v| *v).map_or_else(|_| (0, 0, 0, "UNKNOWN"), |ctx| (ctx.vid, ctx.tid, ctx.nid, ctx.server_name));
        $crate::log::send_log(format!(
            "1 {} {} {} {} {} {} {} {} {} {} {}",
            vid,
            $ts,
            tid,
            nid,
            server_name,
            $cost,
            $method,
            $url,
            $host,
            $return_code,
            $return_length
        ))
    };
}

#[macro_export]
macro_rules! rpc_log {
    () => {};
}
