use anyhow::{Result, anyhow, bail};
use bytes::BufMut;
use futures_util::TryFutureExt;
use log::{error, info};
use quinn::{ServerConfig, crypto::rustls::QuicServerConfig};
use rustls_pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use sha2::{Digest, Sha512};
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::mpsc::UnboundedSender;
use zstd::Decoder;

use crate::common::{
    ALPN_QUIC_HTTP, CMD_RESET, CONNECTION_SUCCESS, LINKER_VER, LINKER_VER_ERROR, PASSWORD_ERROR,
    Pack,
};

pub async fn run(
    key_path: PathBuf,
    cert_path: PathBuf,
    addr: SocketAddr,
    to_local: UnboundedSender<Pack>,
    pw: String,
) -> Result<()> {
    let server_config = make_server(&key_path, &cert_path)?;
    let endpoint = quinn::Endpoint::server(server_config, addr)?;
    info!("listening on {}", endpoint.local_addr()?);
    while let Some(conn) = endpoint.accept().await {
        tokio::spawn(
            handle_connection(conn, to_local.clone(), pw.clone()).unwrap_or_else(|e| {
                error!("connection failed: {}", e);
            }),
        );
    }
    Ok(())
}

fn make_server(key_path: &PathBuf, cert_path: &PathBuf) -> Result<ServerConfig> {
    let key = PrivateKeyDer::from_pem_file(key_path)?;
    let certs: Result<Vec<_>, _> = CertificateDer::pem_file_iter(cert_path)?.collect();
    let mut server_crypto = rustls::ServerConfig::builder()
        // .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs?, key)?;
    server_crypto.alpn_protocols = ALPN_QUIC_HTTP.iter().map(|&x| x.into()).collect();
    let mut server_config =
        quinn::ServerConfig::with_crypto(Arc::new(QuicServerConfig::try_from(server_crypto)?));
    Arc::get_mut(&mut server_config.transport)
        .unwrap()
        .keep_alive_interval(Some(Duration::from_secs(3)))
        .max_concurrent_bidi_streams(1_u8.into())
        .max_idle_timeout(Some(Duration::from_secs(30).try_into()?));
    Ok(server_config)
}

async fn handle_connection(
    conn: quinn::Incoming,
    to_local: UnboundedSender<Pack>,
    pw: String,
) -> Result<()> {
    let connection = conn.await?;
    info!("connected from {}", connection.remote_address());
    let (stream_id, cmd) = if let Ok((send, mut recv)) = connection.accept_bi().await {
        let mut v = [0; 2];
        recv.read_exact(&mut v)
            .await
            .map_err(|e| anyhow!("failed reading request: {}", e))?;
        if u16::from_le_bytes(v) != LINKER_VER {
            send_response(send, LINKER_VER_ERROR).await?;
            bail!("linker version error");
        }
        let mut v = [0; 64];
        recv.read_exact(&mut v)
            .await
            .map_err(|e| anyhow!("failed reading request: {}", e))?;
        let mut hasher = Sha512::new();
        hasher.update(pw);
        if hasher.finalize().to_vec() != v {
            send_response(send, PASSWORD_ERROR).await?;
            bail!("password error");
        }
        let mut v = [0; 8];
        recv.read_exact(&mut v)
            .await
            .map_err(|e| anyhow!("failed reading request: {}", e))?;
        let stream_id = u64::from_le_bytes(v);
        let mut cmd = [0; 2];
        recv.read_exact(&mut cmd)
            .await
            .map_err(|e| anyhow!("failed reading request: {}", e))?;

        send_response(send, CONNECTION_SUCCESS).await?;

        (stream_id, u16::from_le_bytes(cmd))
    } else {
        bail!("bi_streams was not sent.");
    };
    if cmd == CMD_RESET {
        let data: Vec<u8> = 0u64.to_le_bytes().into();
        let _ = to_local.send(Pack {
            data: data.into(),
            conn_no: 0,
            stream_id,
        });
        warn!("reset command received");
    }

    loop {
        let stream = match connection.accept_uni().await {
            Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                info!("connection closed");
                return Ok(());
            }
            Err(e) => {
                return Err(anyhow!(e));
            }
            Ok(s) => s,
        };
        tokio::spawn(
            handle_request(stream, to_local.clone(), stream_id)
                .unwrap_or_else(move |e| error!("failed: {}", e)),
        );
    }
}

async fn send_response(mut send: quinn::SendStream, code: u8) -> Result<()> {
    let mut vec = Vec::new();
    vec.put_u8(code);
    send.write_all(&vec)
        .await
        .map_err(|e| anyhow!("failed to send response: {}", e))?;
    send.finish()
        .map_err(|e| anyhow!("failed to shutdown stream: {}", e))?;
    Ok(())
}

async fn handle_request(
    mut recv: quinn::RecvStream,
    to_local: UnboundedSender<Pack>,
    stream_id: u64,
) -> Result<()> {
    let buf = recv
        .read_to_end(usize::MAX)
        .await
        .map_err(|e| anyhow!("failed reading request: {}", e))?;
    let decoder = Decoder::new(&*buf)?;
    let list: Vec<serde_bytes::ByteBuf> = bincode::deserialize_from(decoder)?;
    for buf in list {
        let _ = to_local.send(Pack {
            data: buf.into_vec().into(),
            conn_no: 0,
            stream_id,
        });
    }
    Ok(())
}
