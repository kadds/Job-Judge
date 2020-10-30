use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::sync::mpsc;
use tokio::time::{delay_for, Duration};

static mut QUEUE: Option<mpsc::Sender<String>> = None;

async fn logger_main(address: String, tx: mpsc::Sender<String>, rx: mpsc::Receiver<String>) {
    let mut rx = rx;
    let mut tx = tx;
    loop {
        let mut conn = match TcpStream::connect(address.clone()).await {
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
        }
    }
}

pub fn init_tcp_logger(address: String) {
    let (tx, rx) = mpsc::channel(10000);
    unsafe {
        QUEUE = Some(tx.clone());
    }

    tokio::spawn(logger_main(address, tx, rx));
}

async fn send_log_async(log: String) {
    unsafe {
        if let Some(q) = &mut QUEUE {
            let _ = q.send(log).await;
        }
    }
}

pub fn send_log(log: String) {
    tokio::spawn(send_log_async(log));
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
    pub static LOG_CONTEXT: Option<LogContext<'static>>;
}
#[macro_use]

#[macro_export]
macro_rules! log {
    ($level:expr, $($log:tt)+) => {
        let (vid, tid, server_name) = $crate::log::LOG_CONTEXT.get().map_or_else(|| (0, 0, ""), |ctx| (ctx.vid, ctx.tid, ctx.server_name));
            $crate::log::send_log(format!(
                "0 {} {} {} {} {} {}[{}/{}:{}:{}]",
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
    () => {};
}

#[macro_export]
macro_rules! rpc_log {
    () => {};
}
