use anyhow::{bail, Result};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

#[cfg(target_os = "linux")]
pub mod common;
#[cfg(target_os = "linux")]
mod tcp_client;
#[cfg(target_os = "linux")]
mod unix_client;

pub struct LinkerClient;
#[cfg(target_os = "linux")]
#[allow(clippy::type_complexity)]
impl LinkerClient {
    pub fn start(
        port: &str,
        db: u64,
        pw: String,
        exit_tx: mpsc::Sender<i32>,
        disable_cache: bool,
    ) -> Result<(UnboundedSender<Vec<u8>>, UnboundedReceiver<Vec<u8>>)> {
        let (to_linker, from_local) = mpsc::unbounded_channel();
        let (to_local, from_linker) = mpsc::unbounded_channel();
        if port.starts_with('/') {
            match unix_client::run(port, db, from_local, to_local, pw, exit_tx, disable_cache) {
                Ok(_) => {
                    return Ok((to_linker, from_linker));
                }
                Err(e) => {
                    log::warn!("{}", e);
                }
            }
        } else {
            match tcp_client::run(port, db, from_local, to_local, pw, exit_tx, disable_cache) {
                Ok(_) => {
                    return Ok((to_linker, from_linker));
                }
                Err(e) => {
                    log::warn!("{}", e);
                }
            }
        }
        bail!("linker connection failed");
    }
}

#[cfg(not(target_os = "linux"))]
#[allow(unused_variables)]
#[allow(clippy::type_complexity)]
impl LinkerClient {
    pub fn start(
        port: &str,
        db: u64,
        pw: String,
        exit_tx: mpsc::Sender<i32>,
    ) -> Result<(UnboundedSender<Vec<u8>>, UnboundedReceiver<Vec<u8>>)> {
        bail!("linker is not supported");
    }
}
