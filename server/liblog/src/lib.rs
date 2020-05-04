#[macro_use]
use log::{self, Level, Metadata, Record, SetLoggerError, LevelFilter};
use tokio::fs::File;
use tokio::prelude::*;

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

        println!("{}:{} - {}", record.level(), record.target(), record.args());
    }
    fn flush(&self) {}
}

struct AsyncLogger {
    file: File,
}

impl log::Log for AsyncLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        let text = format!("{}:{} - {}", record.level(), record.target(), record.args());

        //self.file.write_all(text.as_bytes());
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
    // let file = File::create("log.log").await.unwrap();

    // log::set_boxed_logger(Box::new(AsyncLogger { file: file }))?;
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
