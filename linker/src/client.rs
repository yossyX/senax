use anyhow::{anyhow, bail, Result};
use bytes::{BufMut, BytesMut};
use quinn::Endpoint;
use sha2::{Digest, Sha512};
use std::{fs, net::ToSocketAddrs, path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use zstd::Encoder;

use crate::common::{
    Pack, ALPN_QUIC_HTTP, LINKER_VER, LINKER_VER_ERROR, PASSWORD_ERROR, ZSTD_LEVEL,
};

pub fn client_endpoint(ca_path: PathBuf) -> Result<Endpoint> {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(&rustls::Certificate(fs::read(&ca_path)?))?;
    let mut client_crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(roots)
        .with_no_client_auth();
    client_crypto.alpn_protocols = ALPN_QUIC_HTTP.iter().map(|&x| x.into()).collect();
    let mut endpoint = quinn::Endpoint::client("[::]:0".parse().unwrap())?;
    let mut config = quinn::ClientConfig::new(Arc::new(client_crypto));
    Arc::get_mut(&mut config.transport)
        .unwrap()
        .keep_alive_interval(Some(Duration::from_secs(3)))
        .max_concurrent_bidi_streams(1_u8.into())
        .max_idle_timeout(Some(Duration::from_secs(30).try_into()?));
    endpoint.set_default_client_config(config);
    Ok(endpoint)
}

pub fn connect_client(
    remote: String,
    host: &str,
    endpoint: Endpoint,
    pw: String,
    db: u64,
    command: u16,
) -> Result<UnboundedSender<Pack>> {
    let host = host.to_string();
    let (to_outer, from_local) = mpsc::unbounded_channel::<Pack>();
    tokio::spawn(async move {
        if let Err(e) = handle_connection(remote, host, endpoint, from_local, pw, db, command).await
        {
            error!("{}", e);
        }
    });
    Ok(to_outer)
}

async fn handle_connection(
    remote: String,
    host: String,
    endpoint: Endpoint,
    mut from_local: UnboundedReceiver<Pack>,
    pw: String,
    db: u64,
    command: u16,
) -> Result<(), anyhow::Error> {
    let remote = remote
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| anyhow!("couldn't resolve to an address"))?;
    let new_conn = endpoint
        .connect(remote, &host)?
        .await
        .map_err(|e| anyhow!("failed to connect: {}", e))?;
    let quinn::NewConnection {
        connection: conn, ..
    } = new_conn;
    send_password(&conn, pw, db, command).await?;
    while let Some(pack) = from_local.recv().await {
        let mut list1 = vec![pack];
        while let Ok(pack) = from_local.try_recv() {
            list1.push(pack);
        }
        let mut list2 = vec![];
        for pack in &list1 {
            list2.push(&pack.data[..]);
        }
        let size = bincode::serialized_size(&list2)?;
        let buf = BytesMut::with_capacity((size + (size >> 3) + 100).try_into()?).writer();
        let mut encoder = Encoder::new(buf, ZSTD_LEVEL)?;
        bincode::serialize_into(&mut encoder, &list2)?;
        let writer = encoder.finish()?;
        let mut send = conn
            .open_uni()
            .await
            .map_err(|e| anyhow!("failed to open stream: {}", e))?;
        let buf = writer.into_inner().to_vec();
        send.write_all(&buf).await?;
        send.finish()
            .await
            .map_err(|e| anyhow!("failed to shutdown stream: {}", e))?;
    }
    conn.close(0u32.into(), b"done");
    endpoint.wait_idle().await;
    Ok(())
}

async fn send_password(
    conn: &quinn::Connection,
    pw: String,
    db: u64,
    command: u16,
) -> Result<(), anyhow::Error> {
    let (mut send, recv) = conn
        .open_bi()
        .await
        .map_err(|e| anyhow!("failed to open stream: {}", e))?;
    let mut buf = BytesMut::with_capacity(2 + 64 + 8 + 2);
    buf.put_u16_le(LINKER_VER);
    let mut hasher = Sha512::new();
    hasher.update(pw);
    buf.put(&*hasher.finalize());
    buf.put_u64_le(db);
    buf.put_u16_le(command);
    send.write_all(&buf.freeze())
        .await
        .map_err(|e| anyhow!("failed to send request: {}", e))?;
    send.finish()
        .await
        .map_err(|e| anyhow!("failed to shutdown stream: {}", e))?;
    let resp = recv
        .read_to_end(1)
        .await
        .map_err(|e| anyhow!("failed to read response: {}", e))?;
    let result = u8::from_le_bytes(
        resp.try_into()
            .map_err(|_| anyhow!("failed to read response"))?,
    );
    if result == 0 {
    } else if result == LINKER_VER_ERROR {
        bail!("linker version error");
    } else if result == PASSWORD_ERROR {
        bail!("password error");
    } else {
        bail!("unknown error");
    }
    Ok(())
}
