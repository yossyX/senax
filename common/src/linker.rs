use anyhow::{Context as _, Result, bail};
use serde::{Serialize, de::DeserializeOwned};
use std::marker::PhantomData;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

#[cfg(all(feature = "uring", target_os = "linux"))]
pub mod common;
pub mod stream;
#[cfg(all(feature = "uring", target_os = "linux"))]
mod tcp_client;
#[cfg(all(feature = "uring", target_os = "linux"))]
mod unix_client;

#[derive(Debug, Clone)]
pub struct Sender<T> {
    tx: UnboundedSender<Vec<u8>>,
    _phantom: PhantomData<T>,
}
impl<T> Sender<T>
where
    T: Serialize,
{
    pub fn send(&self, data: &T) -> Result<()> {
        let mut buf = Vec::new();
        ciborium::into_writer(data, &mut buf)?;
        self.tx.send(buf)?;
        Ok(())
    }
}
#[derive(Debug)]
pub struct Receiver<T> {
    rx: UnboundedReceiver<Vec<u8>>,
    _phantom: PhantomData<T>,
}
impl<T> Receiver<T>
where
    T: DeserializeOwned,
{
    /// Receive data from the Linker.
    /// If the connection with the Linker is disconnected, return None.
    /// If there is an abnormal disconnection between Linkers and it reconnects, return Some(None).
    pub async fn recv(&mut self) -> Option<Option<Result<T>>> {
        match self.rx.recv().await {
            Some(v) => {
                if v.is_empty() {
                    Some(None)
                } else {
                    Some(Some(
                        ciborium::from_reader::<T, _>(v.as_slice()).context("parse error"),
                    ))
                }
            }
            None => None,
        }
    }
}

pub fn link<T>(
    stream_id: u64,
    port: &str,
    pw: &str,
    exit_tx: mpsc::Sender<i32>,
    send_only: bool,
) -> Result<(Sender<T>, Receiver<T>)>
where
    T: Serialize + DeserializeOwned,
{
    let (to_linker, from_linker) = LinkerClient::start(port, stream_id, pw, exit_tx, send_only)?;
    Ok((
        Sender {
            tx: to_linker,
            _phantom: Default::default(),
        },
        Receiver {
            rx: from_linker,
            _phantom: Default::default(),
        },
    ))
}

pub struct LinkerClient;
#[cfg(all(feature = "uring", target_os = "linux"))]
#[allow(clippy::type_complexity)]
impl LinkerClient {
    pub fn start(
        port: &str,
        stream_id: u64,
        pw: &str,
        exit_tx: mpsc::Sender<i32>,
        send_only: bool,
    ) -> Result<(UnboundedSender<Vec<u8>>, UnboundedReceiver<Vec<u8>>)> {
        let (to_linker, from_local) = mpsc::unbounded_channel();
        let (to_local, from_linker) = mpsc::unbounded_channel();
        if port.starts_with('/') {
            match unix_client::run(
                port,
                stream_id,
                from_local,
                to_local,
                pw.to_string(),
                exit_tx,
                send_only,
            ) {
                Ok(_) => {
                    return Ok((to_linker, from_linker));
                }
                Err(e) => {
                    log::warn!("{}", e);
                }
            }
        } else {
            match tcp_client::run(
                port,
                stream_id,
                from_local,
                to_local,
                pw.to_string(),
                exit_tx,
                send_only,
            ) {
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

#[cfg(not(all(feature = "uring", target_os = "linux")))]
#[allow(unused_variables)]
#[allow(clippy::type_complexity)]
impl LinkerClient {
    pub fn start(
        _port: &str,
        _stream_id: u64,
        _pw: &str,
        _exit_tx: mpsc::Sender<i32>,
        _send_only: bool,
    ) -> Result<(UnboundedSender<Vec<u8>>, UnboundedReceiver<Vec<u8>>)> {
        bail!("linker is not supported");
    }
}
