use ahash::AHashMap;
use anyhow::Result;
use chrono::{Local, Utc};
use colored::Colorize;
use env_logger::Builder;
use log::kv::{Key, Value, source::Visitor};
use log::{Level, Metadata, Record};
use once_cell::sync::OnceCell;
use std::env;
use time::UtcOffset;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

use crate::Rotation;
use crate::log_writer::LogWriter;

const FILTER_ENV: &str = "RUST_LOG";

static WRITER: OnceCell<LogWriter> = OnceCell::new();
static NAMED_WRITER: OnceCell<AHashMap<String, LogWriter>> = OnceCell::new();
static ERROR_TX: OnceCell<UnboundedSender<String>> = OnceCell::new();
static WARN_TX: OnceCell<UnboundedSender<String>> = OnceCell::new();

pub fn init(
    rotation: Rotation,
    offset: Option<UtcOffset>,
    compress: bool,
) -> Result<(UnboundedReceiver<String>, UnboundedReceiver<String>)> {
    let use_local = offset.is_some();
    let offset = offset.unwrap_or(UtcOffset::UTC);
    let (error_tx, error_rx) = mpsc::unbounded_channel::<String>();
    let (warn_tx, warn_rx) = mpsc::unbounded_channel::<String>();
    ERROR_TX.set(error_tx).unwrap();
    WARN_TX.set(warn_tx).unwrap();

    let log_dir = env::var("LOG_DIR").ok();
    if let Some(log_dir) = log_dir {
        std::fs::create_dir_all(&log_dir)?;
        let log_writer = LogWriter::new(rotation.clone(), &log_dir, "log", offset, compress);
        WRITER.set(log_writer).unwrap();
        let log_file = env::var("LOG_FILE").unwrap_or_else(|_| "".to_owned());
        let files: Vec<&str> = log_file.split(',').collect();
        let mut named_writer = AHashMap::<String, LogWriter>::new();
        for file in files {
            let log_writer = LogWriter::new(rotation.clone(), &log_dir, file, offset, compress);
            named_writer.insert(file.to_owned(), log_writer);
        }
        if !named_writer.is_empty() {
            NAMED_WRITER.set(named_writer).unwrap();
        }
    }

    let logger = Logger::new(use_local);
    log::set_max_level(logger.inner.filter());
    log::set_boxed_logger(Box::new(logger))?;
    Ok((error_rx, warn_rx))
}

struct Logger {
    inner: env_logger::Logger,
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

#[derive(Default)]
struct KvBuf(String);

impl Visitor<'_> for KvBuf {
    fn visit_pair(&mut self, key: Key, value: Value) -> Result<(), log::kv::value::Error> {
        #[cfg(not(feature = "jsonl"))]
        {
            let str = format!(
                "\t{}:{}",
                key,
                serde_json::to_string(&value)
                    .map_err(|v| log::warn!("{}", v))
                    .unwrap_or_default()
            );
            self.0.push_str(&str);
        }
        #[cfg(feature = "jsonl")]
        {
            let str = format!(
                ", {:?}:{}",
                key.as_str(),
                serde_json::to_string(&value)
                    .map_err(|v| log::warn!("{}", v))
                    .unwrap_or_default()
            );
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
            let time = if self.use_local {
                Local::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, false)
            } else {
                Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            };
            let mut visitor = KvBuf::default();
            let _ = record.key_values().visit(&mut visitor);
            #[cfg(not(feature = "jsonl"))]
            let log = match record.args().as_str() {
                Some("") => format!(
                    "time:{}\tlevel:{}\ttarget:{}{}\n",
                    time,
                    record.level(),
                    record.target(),
                    visitor.0,
                ),
                _ => format!(
                    "time:{}\tlevel:{}\ttarget:{}{}\tmsg:{}\n",
                    time,
                    record.level(),
                    record.target(),
                    visitor.0,
                    record.args(),
                ),
            };
            #[cfg(feature = "jsonl")]
            let log = match record.args().as_str() {
                Some("") => format!(
                    "{{\"time\":{:?}, \"level\":{:?}, \"target\":{:?}{}}}\n",
                    time,
                    record.level().as_str(),
                    record.target(),
                    visitor.0,
                ),
                _ => format!(
                    "{{\"time\":{:?}, \"level\":{:?}, \"target\":{:?}{}, \"msg\":{:?}}}\n",
                    time,
                    record.level().as_str(),
                    record.target(),
                    visitor.0,
                    record.args().to_string(),
                ),
            };
            if let Some(writer) = NAMED_WRITER.get().and_then(|w| w.get(record.target())) {
                writer.write(log);
            } else {
                if WRITER.get().is_none() || cfg!(debug_assertions) {
                    if record.metadata().level() == Level::Error {
                        print!("{}", log.red());
                    } else if record.metadata().level() == Level::Warn {
                        print!("{}", log.yellow());
                    } else {
                        print!("{}", log);
                    }
                }
                if record.level() == Level::Error {
                    ERROR_TX.get().map(|v| v.send(log.clone()));
                }
                if record.level() == Level::Warn {
                    WARN_TX.get().map(|v| v.send(log.clone()));
                }
                if let Some(w) = WRITER.get() {
                    w.write(log)
                }
            }
        }
    }

    fn flush(&self) {}
}
