use anyhow::{Error, Result};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use log::{error, info};
use regex::Regex;
use sha2::{Digest, Sha512};
use std::thread;
use std::time::Duration;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio::sync::oneshot;
use tokio::time::sleep;
use tokio_uring::buf::IoBuf;
use tokio_uring::net::TcpStream;

use super::common::LINKER_VER;
use super::common::{IoBytesMut, RECEIVER, SENDER};

const LINKER_PORT: u16 = 25551;

pub(crate) fn run(
    tcp_port: &str,
    stream_id: u64,
    from_local: UnboundedReceiver<Vec<u8>>,
    to_local: UnboundedSender<Vec<u8>>,
    pw: String,
    exit_tx: mpsc::Sender<i32>,
    send_only: bool,
) -> Result<()> {
    let re = Regex::new(r":\d+$").unwrap();
    let tcp_port = if re.is_match(tcp_port) {
        tcp_port.to_owned()
    } else {
        format!("{}:{}", tcp_port, LINKER_PORT)
    };
    let _tcp_port = tcp_port.clone();
    let _pw = pw.clone();
    let (conn_no_sender, conn_no_receiver) = oneshot::channel::<u64>();
    let _exit_tx = exit_tx.clone();
    thread::Builder::new()
        .name("tcp adapter".to_string())
        .spawn(move || {
            tokio_uring::start(async move {
                info!("connecting to {_tcp_port}");
                let stream = match TcpStream::connect(_tcp_port.parse()?).await {
                    Ok(stream) => stream,
                    Err(_) => {
                        sleep(Duration::from_secs(1)).await;
                        TcpStream::connect(_tcp_port.parse()?).await?
                    }
                };
                handle_sender_stream(stream, stream_id, conn_no_sender, from_local, _pw).await?;
                Ok(())
            })
            .map_err(|e: Error| {
                error!("{}", &e);
                let _ = _exit_tx.try_send(1);
                e
            })
        })?;
    if !send_only {
        thread::Builder::new()
            .name("tcp adapter".to_string())
            .spawn(move || {
                tokio_uring::start(async move {
                    let conn_no = conn_no_receiver.await?;
                    info!("connecting to {tcp_port}");
                    let stream = match TcpStream::connect(tcp_port.parse()?).await {
                        Ok(stream) => stream,
                        Err(_) => {
                            sleep(Duration::from_secs(1)).await;
                            TcpStream::connect(tcp_port.parse()?).await?
                        }
                    };
                    handle_receiver_stream(stream, stream_id, conn_no, to_local, pw).await?;
                    Ok(())
                })
                .map_err(|e: Error| {
                    error!("{}", &e);
                    let _ = exit_tx.try_send(1);
                    e
                })
            })?;
    }
    Ok(())
}

async fn handle_sender_stream(
    stream: TcpStream,
    stream_id: u64,
    conn_no_sender: oneshot::Sender<u64>,
    mut from_local: UnboundedReceiver<Vec<u8>>,
    pw: String,
) -> Result<()> {
    let mut buf = BytesMut::with_capacity(2 + 2 + 64 + 8);
    buf.put_u16_le(LINKER_VER);
    buf.put_u16_le(SENDER);
    let mut hasher = Sha512::new();
    hasher.update(pw);
    buf.put(&*hasher.finalize());
    buf.put_u64_le(stream_id);
    write_all(buf.freeze(), &stream).await?;
    let buf = IoBytesMut::new(8);
    let conn_no = read_all(buf, &stream).await?.get_u64_le();
    let _ = conn_no_sender.send(conn_no);
    loop {
        tokio::select! {
            Some(data) = from_local.recv() => {
                let mut list = vec![serde_bytes::ByteBuf::from(data)];
                while let Ok(data) = from_local.try_recv() {
                    list.push(serde_bytes::ByteBuf::from(data));
                }
                let size = bincode::serialized_size(&list)?;
                let mut buf = Vec::with_capacity((size + (size >> 3) + 100).try_into()?);
                buf.put_u64_le(0u64);
                let mut compressor = lz4_flex::frame::FrameEncoder::new(buf);
                bincode::serialize_into(&mut compressor, &list)?;
                let mut buf = compressor.finish()?;
                let size = ((buf.len() - 8) as u64).to_le_bytes();
                unsafe {
                    std::ptr::copy(size.as_ptr(), buf.as_mut_ptr(), 8);
                }
                write_all(buf.into(), &stream).await?;
            }
            else => break,
        }
    }
    Ok(())
}

async fn handle_receiver_stream(
    stream: TcpStream,
    stream_id: u64,
    conn_no: u64,
    to_local: UnboundedSender<Vec<u8>>,
    pw: String,
) -> Result<()> {
    let mut buf = BytesMut::with_capacity(2 + 2 + 64 + 8 + 8);
    buf.put_u16_le(LINKER_VER);
    buf.put_u16_le(RECEIVER);
    let mut hasher = Sha512::new();
    hasher.update(pw);
    buf.put(&*hasher.finalize());
    buf.put_u64_le(stream_id);
    buf.put_u64_le(conn_no);
    write_all(buf.freeze(), &stream).await?;
    loop {
        let buf = IoBytesMut::new(8);
        tokio::select! {
            (res, mut buf) = stream.read(buf) => {
                let n = res?;
                if n == 0 { break }
                buf.advance(n);
                let buf = read_msg(buf, &stream).await?;
                if buf.is_empty() {
                    to_local.send(Vec::new())?;
                } else {
                    let decompressed_input = lz4_flex::frame::FrameDecoder::new(buf.freeze().reader());
                    let list: Vec<serde_bytes::ByteBuf> = bincode::deserialize_from(decompressed_input)?;
                    for data in list {
                        to_local.send(data.into_vec())?;
                    }
                }
            },
            else => break,
        }
    }
    Ok(())
}

async fn write_all(mut buf: Bytes, stream: &TcpStream) -> Result<()> {
    while !buf.is_empty() {
        let (res, _buf) = stream.write(buf).await;
        buf = _buf;
        buf.advance(res?);
    }
    Ok(())
}

async fn read_msg(buf: IoBytesMut, stream: &TcpStream) -> Result<BytesMut> {
    let len = read_all(buf, stream).await?.get_u64_le();
    let buf = IoBytesMut::new(len.try_into()?);
    read_all(buf, stream).await
}

async fn read_all(mut buf: IoBytesMut, stream: &TcpStream) -> Result<BytesMut> {
    while buf.bytes_total() > 0 {
        let (res, _buf) = stream.read(buf).await;
        buf = _buf;
        buf.advance(res?);
    }
    Ok(buf.get())
}
