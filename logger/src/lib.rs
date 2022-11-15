pub mod logger;

#[cfg(target_os = "linux")]
#[path = "log_writer.rs"]
pub mod log_writer;

#[cfg(not(target_os = "linux"))]
#[path = "log_writer_for_not_linux.rs"]
pub mod log_writer;

pub use crate::logger::init;
