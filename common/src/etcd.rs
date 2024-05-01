use anyhow::{Context, Result};
use etcd_client::{
    Client, ConnectOptions, EventType, GetOptions, TlsOptions, WatchOptions, WatchStream,
};
use fxhash::FxHashMap;
use std::{env, fs, sync::Arc};
use tokio::sync::{Mutex, RwLock, RwLockReadGuard};

static CLIENT: Mutex<Option<Client>> = Mutex::const_new(None);

pub async fn init() -> Result<()> {
    let etcd_port = env::var("ETCD_PORT").unwrap_or_else(|_| "localhost:2379".to_string());
    let etcd_user = env::var("ETCD_USER").ok();
    let etcd_pw = env::var("ETCD_PW").ok();
    let etcd_domain_name = env::var("ETCD_DOMAIN_NAME").ok();
    let etcd_ca_pem_file = env::var("ETCD_CA_PEM_FILE").ok();
    let etcd_cert_pem_file = env::var("ETCD_CERT_PEM_FILE").ok();
    let etcd_key_pem_file = env::var("ETCD_KEY_PEM_FILE").ok();

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
    let mut etcd_client = CLIENT.lock().await;
    *etcd_client = Some(Client::connect(etcd_port, Some(option)).await?);
    Ok(())
}

pub async fn get(key: &str) -> Result<Option<String>> {
    let key = format!("/senax/{key}");
    let mut etcd_client = CLIENT.lock().await;
    let res = etcd_client
        .as_mut()
        .expect("etcd_client has not been initialized.")
        .get(&*key, None)
        .await?;
    let list = res.kvs();
    if list.is_empty() {
        return Ok(None);
    }
    Ok(Some(list[0].value_str()?.to_string()))
}

/// Get all key values with matching prefixes.
pub async fn map(prefix: &str) -> Result<FxHashMap<String, String>> {
    let opt = GetOptions::new().with_prefix();
    let key = format!("/senax/{prefix}");
    let mut etcd_client = CLIENT.lock().await;
    let res = etcd_client
        .as_mut()
        .expect("etcd_client has not been initialized.")
        .get(&*key, Some(opt))
        .await?;
    let mut result = FxHashMap::default();
    for v in res.kvs() {
        result.insert(
            v.key_str()?.trim_start_matches(&key).to_string(),
            v.value_str()?.to_string(),
        );
    }
    Ok(result)
}

pub async fn put(key: &str, value: &str) -> Result<()> {
    let key = format!("/senax/{key}");
    let mut etcd_client = CLIENT.lock().await;
    etcd_client
        .as_mut()
        .expect("etcd_client has not been initialized.")
        .put(&*key, value, None)
        .await?;
    Ok(())
}

pub async fn delete(key: &str) -> Result<()> {
    let key = format!("/senax/{key}");
    let mut etcd_client = CLIENT.lock().await;
    etcd_client
        .as_mut()
        .expect("etcd_client has not been initialized.")
        .delete(&*key, None)
        .await?;
    Ok(())
}

pub async fn watch(key: &str, with_prefix: bool) -> Result<WatchStream> {
    let mut opt = WatchOptions::new();
    let mut key = format!("/senax/{key}");
    if with_prefix {
        opt = opt.with_prefix();
    }
    let mut etcd_client = CLIENT.lock().await;
    let (_, stream) = etcd_client
        .as_mut()
        .expect("etcd_client has not been initialized.")
        .watch(&*key, Some(opt))
        .await?;
    Ok(stream)
}

#[derive(Debug, Clone)]
pub struct SyncMap(Arc<RwLock<FxHashMap<String, String>>>);
impl SyncMap {
    pub async fn map(&self) -> RwLockReadGuard<FxHashMap<String, String>> {
        self.0.read().await
    }
}

/// Get a Map that is always synchronized with etcd.
pub async fn sync(prefix: &str) -> Result<SyncMap> {
    let opt = WatchOptions::new().with_prefix();
    let key = format!("/senax/{prefix}");
    let mut etcd_client = CLIENT.lock().await;
    let (_, mut stream) = etcd_client
        .as_mut()
        .expect("etcd_client has not been initialized.")
        .watch(&*key, Some(opt))
        .await?;
    let opt = GetOptions::new().with_prefix();
    let res = etcd_client
        .as_mut()
        .expect("etcd_client has not been initialized.")
        .get(&*key, Some(opt))
        .await?;
    let mut map = FxHashMap::default();
    for v in res.kvs() {
        map.insert(
            v.key_str()?.trim_start_matches(&key).to_string(),
            v.value_str()?.to_string(),
        );
    }
    let map = Arc::new(RwLock::new(map));
    let weak = Arc::downgrade(&map);
    let sync = SyncMap(map);
    tokio::spawn(async move {
        loop {
            match stream.message().await {
                Ok(res) => match res {
                    Some(res) => {
                        let map = match weak.upgrade() {
                            Some(map) => map,
                            None => {
                                break;
                            }
                        };
                        for event in res.events() {
                            let k = event.kv().map(|k| k.key_str());
                            let v = event.kv().map(|v| v.value_str());
                            match event.event_type() {
                                EventType::Delete => {
                                    if let Some(Ok(k)) = k {
                                        map.write().await.remove(k.trim_start_matches(&key));
                                    }
                                }
                                EventType::Put => {
                                    if let (Some(Ok(k)), Some(Ok(v))) = (k, v) {
                                        map.write().await.insert(
                                            k.trim_start_matches(&key).to_string(),
                                            v.to_string(),
                                        );
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        break;
                    }
                },
                Err(e) => {
                    log::error!("{}", e);
                    break;
                }
            }
        }
    });
    Ok(sync)
}
