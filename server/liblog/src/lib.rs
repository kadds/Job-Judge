#[macro_use]
use log::{self, Level, Metadata, Record, SetLoggerError, LevelFilter};
use chrono::Local;
use tokio::fs::File;
use tokio::prelude::*;
extern crate ansi_term;
use ansi_term::Colour;
use ansi_term::Style;

fn color_text(record: &Record) -> String {
    let color = match record.level() {
        Level::Debug => Colour::White,
        Level::Trace => Colour::Fixed(252),
        Level::Info => Colour::Green,
        Level::Warn => Colour::Yellow,
        Level::Error => Colour::Red,
    };

    let fmt_text = format!(
        "[{}]<{}:{}> {} ({}:{})",
        Local::now().format("%Y-%m-%d %H:%M:%S"),
        record.level(),
        record.target(),
        record.args(),
        record.module_path().unwrap_or("<unnamed>"),
        record.line().unwrap_or(0)
    );
    color.paint(fmt_text).to_string()
}

struct TestLogger {
    level: Level,
}

impl log::Log for TestLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        println!("{}", color_text(record));
    }
    fn flush(&self) {}
}

struct AsyncLogger {}

impl log::Log for AsyncLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        println!("{}", color_text(record));
        // send to Kafka
    }
    fn flush(&self) {}
}

pub fn init_test_logger() -> Result<(), SetLoggerError> {
    log::set_boxed_logger(Box::new(TestLogger {
        level: Level::Trace,
    }))?;
    log::set_max_level(LevelFilter::Trace);
    Ok(())
}

pub fn init_async_logger() -> Result<(), SetLoggerError> {
    log::set_boxed_logger(Box::new(AsyncLogger {}))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use log::info;

    #[test]
    fn test_log() {
        super::init_test_logger().unwrap();
        info!("log info");
    }
}
