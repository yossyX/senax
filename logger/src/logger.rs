use ahash::AHashMap;
use anyhow::Result;
use chrono::{Local, Utc};
use colored::Colorize;
use env_logger::filter::{Builder, Filter};
use log::kv::{source::Visitor, Key, Value};
use log::{Level, Metadata, Record};
use once_cell::sync::OnceCell;
use std::env;
use time::UtcOffset;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::log_writer::LogWriter;

const FILTER_ENV: &str = "RUST_LOG";

static WRITER: OnceCell<LogWriter> = OnceCell::new();
static NAMED_WRITER: OnceCell<AHashMap<String, LogWriter>> = OnceCell::new();
static ERROR_TX: OnceCell<UnboundedSender<String>> = OnceCell::new();
static WARN_TX: OnceCell<UnboundedSender<String>> = OnceCell::new();

pub fn init(
    offset: Option<UtcOffset>,
) -> Result<(UnboundedReceiver<String>, UnboundedReceiver<String>)> {
    let use_local = offset.is_some();
    let offset = offset.unwrap_or(UtcOffset::UTC);
    let compress = !cfg!(debug_assertions);
    let log_dir = env::var("LOG_DIR").unwrap_or_else(|_| "log".to_owned());
    std::fs::create_dir_all(&log_dir)?;
    let (error_tx, error_rx) = mpsc::unbounded_channel::<String>();
    let (warn_tx, warn_rx) = mpsc::unbounded_channel::<String>();
    ERROR_TX.set(error_tx).unwrap();
    WARN_TX.set(warn_tx).unwrap();

    let log_writer = LogWriter::daily(&log_dir, "log", offset, compress);
    WRITER.set(log_writer).unwrap();

    let log_file = env::var("LOG_FILE").unwrap_or_else(|_| "".to_owned());
    let files: Vec<&str> = log_file.split(',').collect();
    let mut named_writer = AHashMap::<String, LogWriter>::new();
    for file in files {
        let log_writer = LogWriter::daily(&log_dir, file, offset, compress);
        named_writer.insert(file.to_owned(), log_writer);
    }
    NAMED_WRITER.set(named_writer).unwrap();

    let logger = Logger::new(use_local);
    log::set_max_level(logger.inner.filter());
    log::set_boxed_logger(Box::new(logger))?;
    Ok((error_rx, warn_rx))
}

struct Logger {
    inner: Filter,
    use_local: bool,
}

impl Logger {
    fn new(use_local: bool) -> Logger {
        let mut builder = Builder::from_env(FILTER_ENV);

        Logger {
            inner: builder.build(),
            use_local,
        }
    }
}

fn remove_ctrl(str: &mut String, end: char) {
    let len = str.len();
    let buf = unsafe { str.as_mut_vec() };
    for c in buf.iter_mut() {
        if *c <= 0x1F || *c == 0x7F {
            *c = b' ';
        }
    }
    buf[len - 1] = end as u8;
}

fn remove_ctrl_except_tab(str: &mut String, end: char) {
    let len = str.len();
    let buf = unsafe { str.as_mut_vec() };
    for c in buf.iter_mut() {
        if *c != b'\t' && (*c <= 0x1F || *c == 0x7F) {
            *c = b' ';
        }
    }
    buf[len - 1] = end as u8;
}
struct KvBuf(String);
impl Visitor<'_> for KvBuf {
    fn visit_pair(&mut self, key: Key, value: Value) -> Result<(), log::kv::value::Error> {
        let value = format!("{}", value);
        if !value.is_empty() {
            let mut str = format!("{}:{}\t", key, value);
            remove_ctrl(&mut str, '\t');
            self.0.push_str(&str);
        }
        Ok(())
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            if let Some(writer) = NAMED_WRITER.wait().get(record.target()) {
                let mut log = format!("{}\n", record.args());
                remove_ctrl_except_tab(&mut log, '\n');
                writer.write(log);
            } else {
                let time = if self.use_local {
                    Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, false)
                } else {
                    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, false)
                };
                let mut visitor = KvBuf(String::new());
                let _ = record.key_values().visit(&mut visitor);
                let mut log = match record.args().as_str() {
                    Some("") => format!(
                        "time:{}\tlevel:{}\ttarget:{}\t{}\n",
                        time,
                        record.level(),
                        record.target(),
                        visitor.0,
                    ),
                    _ => format!(
                        "time:{}\tlevel:{}\ttarget:{}\t{}msg:{}\n",
                        time,
                        record.level(),
                        record.target(),
                        visitor.0,
                        record.args(),
                    ),
                };
                if cfg!(debug_assertions) {
                    if record.metadata().level() == Level::Error {
                        print!("{}", log.red());
                    } else if record.metadata().level() == Level::Warn {
                        print!("{}", log.yellow());
                    } else {
                        print!("{}", log);
                    }
                }
                remove_ctrl_except_tab(&mut log, '\n');
                if record.level() == Level::Error {
                    ERROR_TX.get().map(|v| v.send(log.clone()));
                }
                if record.level() == Level::Warn {
                    WARN_TX.get().map(|v| v.send(log.clone()));
                }
                WRITER.wait().write(log);
            }
        }
    }

    fn flush(&self) {}
}
