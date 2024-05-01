pub mod logger;

#[cfg(all(feature = "uring", target_os = "linux"))]
#[path = "log_writer_uring.rs"]
pub mod log_writer;

#[cfg(not(all(feature = "uring", target_os = "linux")))]
#[path = "log_writer.rs"]
pub mod log_writer;

pub use crate::log_writer::Rotation;
pub use crate::logger::init;
