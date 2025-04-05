#[macro_use]
extern crate log;

use anyhow::{Context, Result, bail};
use clap::Parser;
use client::client_endpoint;
use common::Pack;
use dotenvy::dotenv;
use etcd_client::{
    Client, ConnectOptions, EventType, GetOptions, PutOptions, TlsOptions, WatchOptions,
};
use mimalloc::MiMalloc;
use rcgen::generate_simple_self_signed;
use regex::Regex;
use std::collections::hash_map::Entry::Vacant;
use std::{
    collections::HashMap,
    env, fs,
    net::{SocketAddr, ToSocketAddrs},
    path::PathBuf,
    str,
    time::Duration,
};
use tokio::{sync::broadcast, time::sleep};
use tokio::{
    sync::mpsc::{self, UnboundedSender},
    time,
};

use crate::{client::connect_client, common::CMD_RESET};

mod client;
pub mod common;
mod server;
mod tcp_listener;
mod unix_listener;

const WATCHDOG_TTL: i64 = 10;
const WATCHDOG_KEEP_ALIVE: u64 = 1;
const HOST_NAME: &str = "senax_linker";

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct AppArg {
    #[clap(long)]
    cert: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let key_path: PathBuf = env::var_os("KEY")
        .unwrap_or_else(|| "certs/key.pem".into())
        .into();
    let cert_path: PathBuf = env::var_os("CERT")
        .unwrap_or_else(|| "certs/cert.pem".into())
        .into();
    let ca_path: PathBuf = env::var_os("CA")
        .unwrap_or_else(|| "certs/cert.der".into())
        .into();
    let host = env::var("HOST_NAME").unwrap_or_else(|_| HOST_NAME.to_string());

    let arg: AppArg = AppArg::parse();
    if arg.cert {
        let cert = generate_simple_self_signed(vec![host])?;
        let pem_serialized = cert.cert.pem();
        let pem = pem::parse(&pem_serialized).unwrap();
        std::fs::create_dir_all("certs/")?;
        fs::write("certs/cert.pem", pem_serialized.as_bytes())?;
        fs::write("certs/cert.der", pem.contents())?;
        fs::write("certs/key.pem", cert.key_pair.serialize_pem().as_bytes())?;
        // fs::write("certs/key.der", cert.key_pair.serialized_der())?;
        return Ok(());
    }

    let tcp_port: SocketAddr = env::var("TCP_PORT")
        .unwrap_or_else(|_| "127.0.0.1:25551".to_string())
        .parse()?;
    let unix_port = env::var("UNIX_PORT").ok();
    let pw = env::var("PASSWORD").with_context(|| "PASSWORD required")?;
    let etcd_port = env::var("ETCD_PORT").unwrap_or_else(|_| "localhost:2379".to_string());
    let etcd_user = env::var("ETCD_USER").ok();
    let etcd_pw = env::var("ETCD_PW").ok();
    let etcd_domain_name = env::var("ETCD_DOMAIN_NAME").ok();
    let etcd_ca_pem_file = env::var("ETCD_CA_PEM_FILE").ok();
    let etcd_cert_pem_file = env::var("ETCD_CERT_PEM_FILE").ok();
    let etcd_key_pem_file = env::var("ETCD_KEY_PEM_FILE").ok();

    let link_port = link_port()?;
    let endpoint = client_endpoint(ca_path)?;

    let (tx_end, mut rx_end) = broadcast::channel::<i32>(1);
    let (to_all, mut from_local) = mpsc::channel::<Pack>(1);
    let (tx_incoming_local, mut rx_incoming_local) =
        mpsc::channel::<(u64, UnboundedSender<Pack>)>(1);

    let (to_local, mut from_outer) = mpsc::unbounded_channel::<Pack>();
    tokio::spawn(server::run(
        key_path,
        cert_path,
        link_port,
        to_local,
        pw.clone(),
    ));

    tcp_listener::run(
        tx_end.clone(),
        to_all.clone(),
        tx_incoming_local.clone(),
        tcp_port,
        pw.clone(),
    )?;
    if let Some(unix_port) = unix_port {
        unix_listener::run(
            tx_end.clone(),
            to_all.clone(),
            tx_incoming_local.clone(),
            unix_port,
            pw.clone(),
        )?;
    }

    let mut option = ConnectOptions::new();
    if let (Some(etcd_user), Some(etcd_pw)) = (etcd_user, etcd_pw) {
        option = option.with_user(etcd_user, etcd_pw);
    }
    if let Some(domain_name) = etcd_domain_name {
        let mut tls = TlsOptions::new();
        tls = tls.domain_name(domain_name);
        if let Some(ca_path) = etcd_ca_pem_file {
            let pem = fs::read(ca_path).context("failed to read ETCD_CA_PEM_FILE")?;
            let ca = etcd_client::Certificate::from_pem(pem);
            tls = tls.ca_certificate(ca);
        }
        if let (Some(cert_file), Some(key_file)) = (etcd_cert_pem_file, etcd_key_pem_file) {
            let cert = fs::read(cert_file).context("failed to read ETCD_CERT_PEM_FILE")?;
            let key = fs::read(key_file).context("failed to read ETCD_KEY_PEM_FILE")?;
            let identity = etcd_client::Identity::from_pem(cert, key);
            tls = tls.identity(identity);
        }
        option = option.with_tls(tls);
    }
    let etcd_port: Vec<_> = etcd_port.split(',').collect();
    let mut etcd_client = Client::connect(etcd_port, Some(option)).await?;
    let lease = etcd_client.lease_grant(WATCHDOG_TTL, None).await?.id();
    let watchdog_key = set_watchdog(&mut etcd_client, link_port, lease).await?;

    let opt = WatchOptions::new().with_prefix();
    let key = "/senax_linker/";
    let (_watcher, mut etcd_stream) = etcd_client.watch(key, Some(opt)).await?;

    let etcd_stream_re = Regex::new(r"^/senax_linker/(\d+)/(.+)$").unwrap();

    let mut exit_code = 0;
    let mut to_locals: HashMap<u64, Vec<UnboundedSender<Pack>>> = HashMap::new();
    let mut to_outers: HashMap<u64, HashMap<String, UnboundedSender<Pack>>> = HashMap::new();
    let mut interval = time::interval(Duration::from_secs(30));
    interval.tick().await;
    loop {
        tokio::select! {
            Some((stream_id, sender)) = rx_incoming_local.recv() => {
                if let Some(vec) = to_locals.get_mut(&stream_id) {
                    vec.push(sender);
                } else {
                    to_locals.insert(stream_id, vec![sender]);

                    let map = to_outers.entry(stream_id).or_default();
                    for remote in fetch_node_list(&mut etcd_client, stream_id).await? {
                        if let Vacant(e) = map.entry(remote.clone()) {
                            let to_outer = connect_client(remote, &host, endpoint.clone(), pw.clone(), stream_id, 0)?;
                            e.insert(to_outer);
                        }
                    }
                    register_node(&mut etcd_client, stream_id, link_port, lease).await?;
                }
            },
            Some(pack) = from_local.recv() => {
                // from local to outer linker servers
                let stream_id = pack.stream_id;
                if let Some(vec) = to_locals.get_mut(&stream_id) {
                    vec.retain(|sender| sender.send(pack.clone()).is_ok());
                }
                if let Some(map) = to_outers.get_mut(&stream_id) {
                    map.retain(|_, sender| sender.send(pack.clone()).is_ok());
                }
            },
            Some(pack) = from_outer.recv() => {
                // from outer linker server to local
                let stream_id = pack.stream_id;
                if let Some(vec) = to_locals.get_mut(&stream_id) {
                    vec.retain(|sender| sender.send(pack.clone()).is_ok());
                    if vec.is_empty() {
                        // when all local connections are disconnected
                        log::warn!("stream removed {stream_id}");
                        to_locals.remove(&stream_id);
                        to_outers.remove(&stream_id);
                        unregister_node(&mut etcd_client, stream_id, link_port).await?;
                    }
                }
            },
            result = etcd_stream.message() => {
                let resp = match result {
                    Ok(v) => match v {
                        Some(resp) => resp,
                        None => {
                            error!("etcd watch disconnected");
                            exit_code = 1;
                            break;
                        },
                    },
                    Err(e) => {
                        error!("{}", e);
                        exit_code = 1;
                        break;
                    },
                };
                for event in resp.events() {
                    if let Some(kv) = event.kv() {
                        let key = kv.key_str()?;
                        if EventType::Delete == event.event_type() && key == watchdog_key {
                            let _ = tx_end.send(1);
                        } else if let Some(caps) = etcd_stream_re.captures(key) {
                            let stream_id: u64 = caps.get(1).unwrap().as_str().parse()?;
                            let remote = caps.get(2).unwrap().as_str().to_string();
                            if let Some(map) = to_outers.get_mut(&stream_id) {
                                if EventType::Put == event.event_type() {
                                    if remote != link_port.to_string() && !map.contains_key(&remote) {
                                        // connect with new linker server
                                        let to_outer = connect_client(remote.clone(), &host, endpoint.clone(), pw.clone(), stream_id, 0)?;
                                        map.insert(remote, to_outer);
                                    }
                                } else if EventType::Delete == event.event_type() {
                                    // disconnect from linker server
                                    map.remove(&remote);
                                }
                            }
                        }
                    }
                }
            },
            Ok(v) = rx_end.recv() => {
                exit_code = v;
                break;
            },
            _ = tokio::signal::ctrl_c() => {
                let _ = tx_end.send(0);
            }
            _ = interval.tick() => {
                // check connections
                for (&stream_id, map) in to_outers.iter_mut() {
                    for remote in fetch_node_list(&mut etcd_client, stream_id).await? {
                        if remote != link_port.to_string() && !map.contains_key(&remote) {
                            log::warn!("send reset {remote}");
                            let to_outer = connect_client(remote.clone(), &host, endpoint.clone(), pw.clone(), stream_id, CMD_RESET)?;
                            map.insert(remote, to_outer);
                        }
                    }
                }
            }
            else => break,
        }
    }
    let _ = tx_end.send(0);
    let _ = etcd_client.lease_revoke(lease).await;
    sleep(Duration::from_millis(100)).await;
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

fn link_port() -> Result<SocketAddr> {
    let port: u16 = env::var("OUTER_PORT")
        .unwrap_or_else(|_| "25552".to_string())
        .parse()
        .unwrap_or_default();
    if port == 0 {
        return Ok(env::var("OUTER_PORT").unwrap().parse()?);
    }
    let hostname = hostname::get()?;
    if let Ok(mut port) = format!("{}:{port}", hostname.to_str().unwrap()).to_socket_addrs() {
        if let Some(port) = port.next() {
            return Ok(port);
        }
    }
    for iface in if_addrs::get_if_addrs()? {
        if !iface.is_loopback() {
            return Ok(format!("{}:{port}", iface.ip()).parse()?);
        }
    }
    bail!("OUTER_PORT not found");
}

async fn set_watchdog(
    etcd_client: &mut Client,
    link_port: SocketAddr,
    lease: i64,
) -> Result<String> {
    let (mut keeper, mut _stream) = etcd_client.lease_keep_alive(lease).await?;
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(WATCHDOG_KEEP_ALIVE)).await;
            let _ = keeper.keep_alive().await;
        }
    });
    let opt = PutOptions::new().with_lease(lease);
    let key = format!("/senax_linker/watchdog/{link_port}");
    etcd_client.put(&*key, "1", Some(opt)).await?;
    Ok(key)
}

async fn register_node(
    etcd_client: &mut Client,
    stream_id: u64,
    link_port: SocketAddr,
    lease: i64,
) -> Result<()> {
    let opt = PutOptions::new().with_lease(lease);
    let key = format!("/senax_linker/{stream_id}/{link_port}");
    etcd_client.put(&*key, "1", Some(opt)).await?;
    Ok(())
}

async fn unregister_node(
    etcd_client: &mut Client,
    stream_id: u64,
    link_port: SocketAddr,
) -> Result<()> {
    let key = format!("/senax_linker/{stream_id}/{link_port}");
    etcd_client.delete(&*key, None).await?;
    Ok(())
}

async fn fetch_node_list(etcd_client: &mut Client, stream_id: u64) -> Result<Vec<String>> {
    let opt = GetOptions::new().with_prefix();
    let key = format!("/senax_linker/{stream_id}/");
    let resp = etcd_client.get(&*key, Some(opt)).await?;
    let mut ports = Vec::new();
    for kv in resp.kvs() {
        ports.push(kv.key_str()?.trim_start_matches(&key).to_string());
    }
    Ok(ports)
}
