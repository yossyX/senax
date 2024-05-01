use anyhow::{bail, Result};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use sha2::{Digest, Sha512};
use std::fs;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::thread::{self, JoinHandle};
use tokio::sync::broadcast;
use tokio::sync::mpsc::{self, Sender, UnboundedReceiver, UnboundedSender};
use tokio_uring::buf::IoBuf;
use tokio_uring::net::{UnixListener, UnixStream};

use crate::common::{IoBytesMut, Pack, CONN_NO, LINKER_VER, RECEIVER, SENDER};

pub fn run(
    tx_end: broadcast::Sender<i32>,
    to_all: Sender<Pack>,
    tx_incoming_local: Sender<(u64, UnboundedSender<Pack>)>,
    unix_port: String,
    pw: String,
) -> std::io::Result<JoinHandle<Result<()>>> {
    thread::Builder::new()
        .name("unix listener".to_string())
        .spawn(move || {
            let mut rx_end = tx_end.subscribe();
            let tx_end2 = tx_end.clone();
            tokio_uring::start(async move {
                let sock_file = Path::new(&unix_port);
                if sock_file.exists() {
                    fs::remove_file(sock_file)?;
                }
                let listener = UnixListener::bind(sock_file)?;
                loop {
                    tokio::select! {
                        result = listener.accept() => {
                            let stream = result?;
                            let (stream_id, conn_no, mode) = match check_stream(&stream, &pw).await {
                                Ok((stream_id, conn_no, mode)) => (stream_id, conn_no, mode),
                                Err(e) => {
                                    warn!("unix incoming connection: {}", e);
                                    continue;
                                }
                            };
                            let tx_end3 = tx_end2.clone();
                            if mode == SENDER {
                                info!("sender connected");
                                let to_all = to_all.clone();
                                tokio_uring::spawn(async move {
                                    if let Err(e) = handle_sender_stream(conn_no, stream, stream_id, tx_end3, to_all).await {
                                        error!("unix sender {}", &e);
                                    }
                                    info!("sender disconnected");
                                });
                            } else if mode == RECEIVER {
                                info!("receiver connected");
                                let (to_hub, from_hub) = mpsc::unbounded_channel::<Pack>();
                                tx_incoming_local.send((stream_id, to_hub)).await?;
                                tokio_uring::spawn(async move {
                                    if let Err(e) = handle_receiver_stream(conn_no, stream, tx_end3, from_hub).await {
                                        error!("unix receiver {}", &e);
                                    }
                                    info!("receiver disconnected");
                                });
                            }
                        },
                        _stop = rx_end.recv() => {
                            break
                        },
                        else => break,
                    }
                }
                let _ = std::fs::remove_file(sock_file);
                Ok(())
            })
            .map_err(|e| {
                log::error!("{}", &e);
                let _ = tx_end.send(1);
                e
            })
        })
}

async fn check_stream(stream: &UnixStream, pw: &str) -> Result<(u64, u64, u16)> {
    let buf = IoBytesMut::new(2);
    let version = read_all(buf, stream).await?.get_u16_le();
    if version != LINKER_VER {
        bail!("version error");
    }

    let buf = IoBytesMut::new(2);
    let mode = read_all(buf, stream).await?.get_u16_le();

    let buf = IoBytesMut::new(64);
    let pw_hash = read_all(buf, stream).await?;
    let mut hasher = Sha512::new();
    hasher.update(pw);
    if hasher.finalize().to_vec() != pw_hash {
        bail!("password error");
    }

    let buf = IoBytesMut::new(8);
    let stream_id = read_all(buf, stream).await?.get_u64_le();

    let conn_no = if mode == SENDER {
        let conn_no = CONN_NO.fetch_add(1, Ordering::SeqCst);
        let mut buf = BytesMut::with_capacity(8);
        buf.put_u64_le(conn_no);
        write_all(buf.freeze(), stream).await?;
        conn_no
    } else {
        let buf = IoBytesMut::new(8);
        read_all(buf, stream).await?.get_u64_le()
    };
    Ok((stream_id, conn_no, mode))
}

async fn handle_sender_stream(
    conn_no: u64,
    stream: UnixStream,
    stream_id: u64,
    tx_end: broadcast::Sender<i32>,
    to_all: Sender<Pack>,
) -> Result<()> {
    let mut rx_end = tx_end.subscribe();
    loop {
        let buf = IoBytesMut::new(8);
        tokio::select! {
            (res, mut buf) = stream.read(buf) => {
                let n = res?;
                if n == 0 { break }
                buf.advance(n);
                let data =  read_msg(buf, &stream).await?.freeze();
                let _ = to_all.send(Pack{data, conn_no, stream_id}).await;
            },
            _stop = rx_end.recv() => break,
            else => break,
        }
    }
    Ok(())
}

async fn handle_receiver_stream(
    conn_no: u64,
    stream: UnixStream,
    tx_end: broadcast::Sender<i32>,
    mut from_hub: UnboundedReceiver<Pack>,
) -> Result<()> {
    let mut rx_end = tx_end.subscribe();
    loop {
        let buf = IoBytesMut::new(1);
        tokio::select! {
            (res, mut _buf) = stream.read(buf) => {
                let n = res?;
                if n == 0 { break }
            },
            result = from_hub.recv() => {
                let recv = if let Some(recv) = result {
                    recv
                } else {
                    break;
                };
                if conn_no != recv.conn_no {
                    write_all(recv.data, &stream).await?;
                }
            },
            _stop = rx_end.recv() => break,
            else => break,
        }
    }
    Ok(())
}

async fn write_all(mut buf: Bytes, stream: &UnixStream) -> Result<()> {
    while !buf.is_empty() {
        let (res, _buf) = stream.write(buf).await;
        buf = _buf;
        buf.advance(res?);
    }
    Ok(())
}

async fn read_msg(buf: IoBytesMut, stream: &UnixStream) -> Result<BytesMut> {
    let len = read_all(buf, stream).await?.get_u64_le();
    let mut buf = IoBytesMut::new((len + 8).try_into()?);
    buf.put_length(len);
    read_all(buf, stream).await
}

async fn read_all(mut buf: IoBytesMut, stream: &UnixStream) -> Result<BytesMut> {
    while buf.bytes_total() > 0 {
        let (res, _buf) = stream.read(buf).await;
        buf = _buf;
        buf.advance(res?);
    }
    Ok(buf.get())
}
