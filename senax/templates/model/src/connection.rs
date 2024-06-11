// This code is auto-generated and will always be overwritten.
// Senax v@{ ""|senax_version }@

use anyhow::{ensure, Context as _, Result};
#[allow(unused_imports)]
use crossbeam::queue::SegQueue;
use futures::future::BoxFuture;
use fxhash::FxHashMap;
use log::LevelFilter;
#[allow(unused_imports)]
use once_cell::sync::{Lazy, OnceCell};
use senax_common::ShardId;
use sqlx::migrate::MigrateDatabase;
use sqlx::mysql::{MySqlConnectOptions, MySqlPool, MySqlPoolOptions};
use sqlx::pool::PoolConnection;
use sqlx::{ConnectOptions, Executor, MySqlConnection, Row, Transaction};
use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, VecDeque};
#[allow(unused_imports)]
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
#[allow(unused_imports)]
use std::sync::Arc;
use std::time::SystemTime;
use std::{cmp, env};
#[allow(unused_imports)]
use tokio::sync::{Mutex, RwLock};

use crate::models::CacheOp;
use crate::*;

pub type DbType = sqlx::MySql;
pub type DbPool = MySqlPool;
pub type DbConnection = MySqlConnection;
pub type DbArguments = sqlx::mysql::MySqlArguments;
pub type DbRow = sqlx::mysql::MySqlRow;
pub const TX_ISOLATION: Option<&'static str> = @{ tx_isolation|disp_opt }@;
pub const READ_TX_ISOLATION: Option<&'static str> = @{ read_tx_isolation|disp_opt }@;
@%- if config.use_sequence %@
const ID_OF_SEQUENCE: u32 = 1;
@%- endif %@
@%- if !config.force_disable_cache %@
const ID_OF_CACHE_SYNC: u32 = 2;
@%- endif %@

static MAX_CONNECTIONS_FOR_WRITE: AtomicU32 = AtomicU32::new(0);
static MAX_CONNECTIONS_FOR_READ: AtomicU32 = AtomicU32::new(0);
static MAX_CONNECTIONS_FOR_CACHE: AtomicU32 = AtomicU32::new(0);
static SEQUENCE_FETCH_NUM: AtomicU32 = AtomicU32::new(0);

static SOURCE: RwLock<Vec<DbPool>> = RwLock::const_new(Vec::new());
static SOURCE_NUM: OnceCell<usize> = OnceCell::new();
static REPLICA: RwLock<Vec<Vec<DbPool>>> = RwLock::const_new(Vec::new());
static CACHE: RwLock<Vec<Vec<DbPool>>> = RwLock::const_new(Vec::new());
static NEXT_REPLICA: AtomicUsize = AtomicUsize::new(0);
@%- if !config.force_disable_cache %@
static NEXT_CACHE: AtomicUsize = AtomicUsize::new(0);
@%- endif %@
@%- if config.use_sequence %@

static SEQUENCE: Lazy<Vec<Mutex<(u64, u64)>>> = Lazy::new(|| {
    DbConn::shard_num_range()
        .map(|_| Mutex::new((0, 0)))
        .collect()
});
@%- endif %@
@%- if !config.force_disable_cache %@
static CACHE_SYNC: Lazy<Vec<Mutex<()>>> =
    Lazy::new(|| DbConn::shard_num_range().map(|_| Mutex::new(())).collect());
static CACHE_SYNC_QUEUE: Lazy<Vec<SegQueue<Arc<AtomicU64>>>> =
    Lazy::new(|| DbConn::shard_num_range().map(|_| SegQueue::new()).collect());
@%- endif %@
type NotifyFn = Box<dyn Fn(crate::models::TableName, crate::models::NotifyOp, &str) + Send + Sync>;
static NOTIFY_RECEIVER: RwLock<Vec<NotifyFn>> = RwLock::const_new(Vec::new());
static NOTIFY_RECEIVER_COUNT: AtomicUsize = AtomicUsize::new(0);
static NOTIFY_LIST: RwLock<
    Option<Vec<(crate::models::TableName, crate::models::NotifyOp, String)>>,
> = RwLock::const_new(None);

fn env_u32(etcd: &FxHashMap<String, String>, name: &str, default: &str) -> Result<u32> {
    let v = etcd.get(name).cloned();
    let v = v.unwrap_or_else(|| {
        env::var(format!("@{ db|upper_snake }@_{}", name)).unwrap_or_else(|_| default.to_owned())
    });
    v.parse().with_context(|| format!("{} parse error", name))
}
fn env_opt_str(etcd: &FxHashMap<String, String>, name: &str) -> Option<String> {
    let v = etcd.get(name).cloned();
    v.or_else(|| env::var(format!("@{ db|upper_snake }@_{}", name)).ok())
}
fn env_urls(etcd: &FxHashMap<String, String>, name: &str) -> Result<Option<String>> {
    let mut urls = etcd.get(name).cloned();
    if urls.is_none() && !etcd.is_empty() {
        let prefix = format!("{}/", name);
        let mut values = BTreeMap::new();
        for (k, v) in etcd {
            if k.starts_with(&prefix) {
                let i: usize = k.trim_start_matches(&prefix).parse()?;
                values.insert(i, v);
            }
        }
        if !values.is_empty() {
            let vec: Vec<_> = values.values().map(|v| v.to_string()).collect();
            urls = Some(vec.join(" "));
        }
    }
    Ok(urls.or_else(|| env::var(format!("@{ db|upper_snake }@_{}", name)).ok()))
}
#[rustfmt::skip]
async fn config(etcd: &FxHashMap<String, String>) -> Result<()> {
    let v = env_u32(etcd, "DB_MAX_CONNECTIONS_FOR_WRITE", DEFAULT_DB_MAX_CONNECTIONS_FOR_WRITE)?;
    MAX_CONNECTIONS_FOR_WRITE.store(v, Ordering::SeqCst);
    let v = env_u32(etcd, "DB_MAX_CONNECTIONS_FOR_READ", DEFAULT_DB_MAX_CONNECTIONS_FOR_READ)?;
    MAX_CONNECTIONS_FOR_READ.store(v, Ordering::SeqCst);
    let v = env_u32(etcd, "DB_MAX_CONNECTIONS_FOR_CACHE", DEFAULT_DB_MAX_CONNECTIONS_FOR_CACHE)?;
    MAX_CONNECTIONS_FOR_CACHE.store(v, Ordering::SeqCst);
    let v = env_u32(etcd, "SEQUENCE_FETCH_NUM", DEFAULT_SEQUENCE_FETCH_NUM)?;
    SEQUENCE_FETCH_NUM.store(cmp::max(1, v), Ordering::SeqCst);
    Ok(())
}

pub async fn init() -> Result<()> {
    connect().await?;
    #[cfg(feature = "etcd")]
    tokio::spawn(async {
        let mut stream = match senax_common::etcd::watch("db/@{ db|upper_snake }@_", true).await {
            Ok(stream) => stream,
            Err(err) => {
                log::error!("{}", err);
                return;
            }
        };
        loop {
            match stream.message().await {
                Ok(Some(_)) => {}
                Ok(None) => {
                    log::error!("etcd connection has been lost.");
                    break;
                }
                Err(err) => {
                    log::error!("{}", err);
                    break;
                }
            }
            if let Err(err) = connect().await {
                log::error!("{}", err);
                exit(1);
            }
            match DbConn::inc_all_cache_sync().await {
                Ok(sync_map) => {
                    models::_clear_cache(&sync_map, false).await;
                }
                Err(e) => {
                    log::error!("{}", e);
                    exit(1);
                }
            }
        }
    });
    Ok(())
}

#[rustfmt::skip]
pub async fn connect() -> Result<()> {
    #[cfg(feature = "etcd")]
    let etcd = senax_common::etcd::map("db/@{ db|upper_snake }@_").await?;
    #[cfg(not(feature = "etcd"))]
    let etcd = FxHashMap::default();
    config(&etcd).await?;
    let database_url = env_urls(&etcd, "DB_URL")?.with_context(|| "@{ db|upper_snake }@_DB_URL is required in the .env file.")?;
    let user = env_opt_str(&etcd, "DB_USER");
    let pw = env_opt_str(&etcd, "DB_PASSWORD");
    let mut source = SOURCE.write().await;
    source.clear();
    source.append(&mut get_source(&database_url, &user, &pw, DbConn::max_connections_for_write()).await?);
    ensure!(SOURCE_NUM.get_or_init(|| source.len()) == &source.len());

    let replica_url = env_urls(&etcd, "REPLICA_DB_URL")?.unwrap_or_else(|| database_url.clone());
    let user = env_opt_str(&etcd, "REPLICA_DB_USER").or(user);
    let pw = env_opt_str(&etcd, "REPLICA_DB_PASSWORD").or(pw);
    let mut replica = REPLICA.write().await;
    replica.clear();
    replica.append(&mut get_replica(&replica_url, &user, &pw, DbConn::max_connections_for_read()).await?);

    let cache_url = env_urls(&etcd, "CACHE_DB_URL")?.unwrap_or_else(|| replica_url.clone());
    let user = env_opt_str(&etcd, "CACHE_DB_USER").or(user);
    let pw = env_opt_str(&etcd, "CACHE_DB_PASSWORD").or(pw);
    let mut cache = CACHE.write().await;
    cache.clear();
    cache.append(&mut get_replica(&cache_url, &user, &pw, DbConn::max_connections_for_cache()).await?);

    ensure!(
        source.len() == replica.len(),
        "Source and replica shards have different lengths."
    );
    ensure!(
        source.len() == cache.len(),
        "Source and cache shards have different lengths."
    );
    ensure!(
        DbConn::shard_num() <= ShardId::MAX as usize + 1,
        "Number of shards exceeds limit."
    );
    Ok(())
}

#[rustfmt::skip]
pub async fn init_test() -> Result<()> {
    #[cfg(feature = "etcd")]
    let etcd = senax_common::etcd::map("db/@{ db|upper_snake }@_").await?;
    #[cfg(not(feature = "etcd"))]
    let etcd = FxHashMap::default();
    config(&etcd).await?;
    let database_url = env_urls(&etcd, "TEST_DB_URL")?.with_context(|| "@{ db|upper_snake }@_TEST_DB_URL is required in the .env file.")?;
    let user = env_opt_str(&etcd, "TEST_DB_USER");
    let pw = env_opt_str(&etcd, "TEST_DB_PASSWORD");
    let mut source = SOURCE.write().await;
    source.clear();
    source.append(&mut get_source(&database_url, &user, &pw, DbConn::max_connections_for_write()).await?);
    SOURCE_NUM.get_or_init(|| source.len());

    let mut replica = REPLICA.write().await;
    replica.clear();
    replica.append(&mut get_replica(&database_url, &user, &pw, DbConn::max_connections_for_read()).await?);

    let mut cache = CACHE.write().await;
    cache.clear();
    cache.append(&mut get_replica(&database_url, &user, &pw, DbConn::max_connections_for_cache()).await?);

    ensure!(
        DbConn::shard_num() <= ShardId::MAX as usize + 1,
        "Number of shards exceeds limit."
    );
    Ok(())
}

#[rustfmt::skip]
pub(crate) async fn reset_database(is_test: bool, clean: bool) -> Result<()> {
    #[cfg(feature = "etcd")]
    let etcd = senax_common::etcd::map("db/@{ db|upper_snake }@_").await?;
    #[cfg(not(feature = "etcd"))]
    let etcd = FxHashMap::default();
    let (database_url, user, pw) = if is_test {
        (env_urls(&etcd, "TEST_DB_URL")?.with_context(|| "@{ db|upper_snake }@_TEST_DB_URL is required in the .env file.")?,
        env_opt_str(&etcd, "TEST_DB_USER"),
        env_opt_str(&etcd, "TEST_DB_PASSWORD"))
    } else {
        (env_urls(&etcd, "DB_URL")?.with_context(|| "@{ db|upper_snake }@_DB_URL is required in the .env file.")?,
        env_opt_str(&etcd, "DB_USER"),
        env_opt_str(&etcd, "DB_PASSWORD"))
    };

    for url in database_url.split('\n') {
        let url = url.trim();
        if url.is_empty() {
            continue;
        }
        let mut url: url::Url = url.parse()?;
        if let Some(user) = &user {
            url.set_username(user).expect("DB_URL ERROR");
        }
        if pw.is_some() {
            url.set_password(pw.as_deref()).expect("DB_URL ERROR");
        }
        let url = url.as_str();
        if clean {
            DbType::drop_database(url).await?;
            DbType::create_database(url).await?;
        } else if !DbType::database_exists(url).await? {
            DbType::create_database(url).await?;
        }
    }
    Ok(())
}

struct SavePoint {
    cache_internal_op_pos: usize,
    cache_op_pos: usize,
    callback_post: usize,
}
pub struct DbConn {
    ctx_no: u64,
    time: SystemTime,
    shard_id: ShardId,
    @%- if !config.force_disable_cache %@
    cache_sync: u64,
    @%- endif %@
    tx: FxHashMap<ShardId, sqlx::Transaction<'static, DbType>>,
    save_point: Vec<SavePoint>,
    read_tx: FxHashMap<ShardId, sqlx::Transaction<'static, DbType>>,
    @%- if !config.force_disable_cache %@
    cache_tx: FxHashMap<ShardId, sqlx::Transaction<'static, DbType>>,
    @%- endif %@
    conn: FxHashMap<ShardId, PoolConnection<DbType>>,
    cache_internal_op_list: Vec<CacheOp>,
    cache_op_list: Vec<CacheOp>,
    callback_list: VecDeque<Box<dyn FnOnce() -> BoxFuture<'static, ()> + Send + Sync>>,
    pub(crate) clear_whole_cache: bool,
    has_tx: bool,
    wo_tx: usize,
    has_read_tx: usize,
    lock_list: Vec<DbLock>,
}

impl Clone for DbConn {
    fn clone(&self) -> Self {
        DbConn {
            ctx_no: self.ctx_no,
            time: self.time,
            shard_id: self.shard_id,
            @%- if !config.force_disable_cache %@
            cache_sync: 0,
            @%- endif %@
            tx: FxHashMap::default(),
            save_point: Vec::new(),
            read_tx: FxHashMap::default(),
            @%- if !config.force_disable_cache %@
            cache_tx: FxHashMap::default(),
            @%- endif %@
            conn: FxHashMap::default(),
            cache_internal_op_list: Vec::new(),
            cache_op_list: Vec::new(),
            callback_list: VecDeque::new(),
            clear_whole_cache: false,
            has_tx: false,
            wo_tx: 0,
            has_read_tx: 0,
            lock_list: Vec::new(),
        }
    }
}

impl DbConn {
    pub fn new(ctx_no: u64) -> DbConn {
        DbConn {
            ctx_no,
            time: SystemTime::now(),
            shard_id: 0,
            @%- if !config.force_disable_cache %@
            cache_sync: 0,
            @%- endif %@
            tx: FxHashMap::default(),
            save_point: Vec::new(),
            read_tx: FxHashMap::default(),
            @%- if !config.force_disable_cache %@
            cache_tx: FxHashMap::default(),
            @%- endif %@
            conn: FxHashMap::default(),
            cache_internal_op_list: Vec::new(),
            cache_op_list: Vec::new(),
            callback_list: VecDeque::new(),
            clear_whole_cache: false,
            has_tx: false,
            wo_tx: 0,
            has_read_tx: 0,
            lock_list: Vec::new(),
        }
    }

    pub fn new_with_time(ctx_no: u64, time: SystemTime) -> DbConn {
        DbConn {
            ctx_no,
            time,
            shard_id: 0,
            @%- if !config.force_disable_cache %@
            cache_sync: 0,
            @%- endif %@
            tx: FxHashMap::default(),
            save_point: Vec::new(),
            read_tx: FxHashMap::default(),
            @%- if !config.force_disable_cache %@
            cache_tx: FxHashMap::default(),
            @%- endif %@
            conn: FxHashMap::default(),
            cache_internal_op_list: Vec::new(),
            cache_op_list: Vec::new(),
            callback_list: VecDeque::new(),
            clear_whole_cache: false,
            has_tx: false,
            wo_tx: 0,
            has_read_tx: 0,
            lock_list: Vec::new(),
        }
    }

    pub(crate) fn _new(shard_id: ShardId) -> DbConn {
        DbConn {
            ctx_no: 0,
            time: SystemTime::now(),
            shard_id,
            @%- if !config.force_disable_cache %@
            cache_sync: 0,
            @%- endif %@
            tx: FxHashMap::default(),
            save_point: Vec::new(),
            read_tx: FxHashMap::default(),
            @%- if !config.force_disable_cache %@
            cache_tx: FxHashMap::default(),
            @%- endif %@
            conn: FxHashMap::default(),
            cache_internal_op_list: Vec::new(),
            cache_op_list: Vec::new(),
            callback_list: VecDeque::new(),
            clear_whole_cache: false,
            has_tx: false,
            wo_tx: 0,
            has_read_tx: 0,
            lock_list: Vec::new(),
        }
    }

    pub fn ctx_no(&self) -> u64 {
        self.ctx_no
    }

    pub fn set_time(&mut self, time: SystemTime) {
        self.time = time;
    }

    pub fn time(&self) -> SystemTime {
        self.time
    }

    pub fn shard_num() -> usize {
        *SOURCE_NUM.get().unwrap()
    }

    pub fn shard_num_range() -> std::ops::RangeInclusive<ShardId> {
        0..=(Self::shard_num() - 1) as ShardId
    }

    pub fn shard_id(&self) -> ShardId {
        self.shard_id
    }

    pub fn set_shard_id(&mut self, shard_id: usize) {
        self.shard_id = shard_id as ShardId;
    }
    @%- if !config.force_disable_cache %@

    /// Cache transaction synchronization
    #[allow(dead_code)]
    pub(crate) fn cache_sync(&self) -> u64 {
        self.cache_sync
    }

    pub fn set_clear_all_cache(&mut self) {
        self.clear_whole_cache = true;
    }
    @%- endif %@

    pub async fn acquire_source(&self) -> Result<PoolConnection<DbType>> {
        Ok(SOURCE.read().await[self.shard_id as usize]
            .acquire()
            .await?)
    }

    pub async fn acquire_replica(&self) -> Result<PoolConnection<DbType>> {
        let replica = &REPLICA.read().await[self.shard_id as usize];
        let len = replica.len();
        if len == 1 {
            return Ok(replica[0].acquire().await?);
        }
        let index = NEXT_REPLICA.fetch_add(1, Ordering::Relaxed) % len;
        for i in 0..len {
            let idx = (i + index) % len;
            if DbConn::max_connections_for_read() - replica[idx].size()
                + replica[idx].num_idle() as u32
                > 0
            {
                if let Ok(conn) = replica[idx].acquire().await {
                    return Ok(conn);
                }
            }
        }
        for i in 0..len {
            let idx = (i + index) % len;
            if let Ok(conn) = replica[idx].acquire().await {
                return Ok(conn);
            }
        }
        Ok(replica[index].acquire().await?)
    }

    pub(crate) async fn acquire_replica_tx(
        shard_id: ShardId,
    ) -> Result<sqlx::Transaction<'static, DbType>> {
        let replica = &REPLICA.read().await[shard_id as usize];
        let len = replica.len();
        if len == 1 {
            return Ok(replica[0].begin().await?);
        }
        let index = NEXT_REPLICA.fetch_add(1, Ordering::Relaxed) % len;
        for i in 0..len {
            let idx = (i + index) % len;
            if DbConn::max_connections_for_read() - replica[idx].size()
                + replica[idx].num_idle() as u32
                > 0
            {
                if let Ok(conn) = replica[idx].begin().await {
                    return Ok(conn);
                }
            }
        }
        for i in 0..len {
            let idx = (i + index) % len;
            if let Ok(conn) = replica[idx].begin().await {
                return Ok(conn);
            }
        }
        Ok(replica[index].begin().await?)
    }
    @%- if !config.force_disable_cache %@

    pub(crate) async fn acquire_cache_tx(
        shard_id: ShardId,
    ) -> Result<sqlx::Transaction<'static, DbType>> {
        let cache = &CACHE.read().await[shard_id as usize];
        let len = cache.len();
        if len == 1 {
            return Ok(cache[0].begin().await?);
        }
        let index = NEXT_CACHE.fetch_add(1, Ordering::Relaxed) % len;
        for i in 0..len {
            let idx = (i + index) % len;
            if DbConn::max_connections_for_cache() - cache[idx].size()
                + cache[idx].num_idle() as u32
                > 0
            {
                if let Ok(conn) = cache[idx].begin().await {
                    return Ok(conn);
                }
            }
        }
        for i in 0..len {
            let idx = (i + index) % len;
            if let Ok(conn) = cache[idx].begin().await {
                return Ok(conn);
            }
        }
        Ok(cache[index].begin().await?)
    }
    @%- endif %@

    pub async fn get_replica_conn(&mut self) -> Result<&mut PoolConnection<DbType>> {
        let conn = if self.conn.contains_key(&self.shard_id) {
            None
        } else {
            Some(self.acquire_replica().await?)
        };
        Ok(self
            .conn
            .entry(self.shard_id)
            .or_insert_with(|| conn.unwrap()))
    }

    pub fn release_conn(&mut self) {
        self.conn.clear();
    }

    pub async fn begin_without_transaction(&mut self) -> Result<()> {
        self.wo_tx += 1;
        Ok(())
    }

    pub async fn end_of_without_transaction(&mut self) -> Result<()> {
        ensure!(self.wo_tx > 0, "No without transaction is active.");
        self.wo_tx -= 1;
        Ok(())
    }

    pub async fn begin(&mut self) -> Result<()> {
        if !self.has_tx {
            self.has_tx = true;
        } else {
            for (_shard_id, tx) in self.tx.iter_mut() {
                let sql = format!("SAVEPOINT s{}", self.save_point.len());
                tx.execute(&*sql).await?;
            }
            self.save_point.push(SavePoint {
                cache_internal_op_pos: self.cache_internal_op_list.len(),
                cache_op_pos: self.cache_op_list.len(),
                callback_post: self.callback_list.len(),
            });
        }
        Ok(())
    }

    pub async fn begin_immediately(&mut self) -> Result<()> {
        self.begin().await?;
        self.get_tx().await?;
        Ok(())
    }

    pub fn has_tx(&self) -> bool {
        self.has_tx
    }

    pub(crate) fn wo_tx(&self) -> bool {
        !self.has_tx && self.wo_tx > 0
    }

    pub async fn get_tx(&mut self) -> Result<&mut sqlx::Transaction<'static, DbType>> {
        ensure!(self.has_tx(), "No transaction is active.");
        match self.tx.entry(self.shard_id) {
            Entry::Occupied(tx) => Ok(tx.into_mut()),
            Entry::Vacant(v) => {
                let mut tx = SOURCE.read().await[self.shard_id as usize].begin().await?;
                set_tx_isolation(&mut tx).await?;
                for s in 0..self.save_point.len() {
                    let sql = format!("SAVEPOINT s{}", s);
                    tx.execute(&*sql).await?;
                }
                Ok(v.insert(tx))
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) async fn push_cache_op(&mut self, op: CacheOp) -> Result<()> {
        if self.has_tx() {
            self.cache_op_list.push(op);
        } else {
            let sync = Self::inc_cache_sync(self.shard_id()).await?;
            let mut sync_map = FxHashMap::default();
            sync_map.insert(self.shard_id(), sync);
            CacheMsg(vec![op], sync_map).do_send().await;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) async fn push_cache_op_to(&mut self, op: CacheOp, internal: bool) -> Result<()> {
        if internal {
            if self.has_tx() {
                self.cache_internal_op_list.push(op);
            } else {
                let sync = Self::inc_cache_sync(self.shard_id()).await?;
                let mut sync_map = FxHashMap::default();
                sync_map.insert(self.shard_id(), sync);
                CacheMsg(vec![op], sync_map).do_send_to_internal().await;
            }
        } else {
            self.push_cache_op(op).await?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) async fn push_callback(
        &mut self,
        cb: Box<dyn FnOnce() -> BoxFuture<'static, ()> + Send + Sync>,
    ) {
        if self.has_tx() {
            self.callback_list.push_back(cb);
        } else {
            cb().await;
        }
    }

    pub async fn rows_affected(&mut self) -> Result<i64> {
        let query = sqlx::query("select row_count()");
        let row = query.fetch_one(self.get_tx().await?.as_mut()).await?;
        Ok(row.try_get(0)?)
    }

    pub async fn commit(&mut self) -> Result<()> {
        ensure!(self.has_tx(), "No transaction is active.");
        if let Some(_save_point) = self.save_point.pop() {
            for (_shard_id, tx) in self.tx.iter_mut() {
                let sql = format!("RELEASE SAVEPOINT s{}", self.save_point.len());
                tx.execute(&*sql).await?;
            }
            return Ok(());
        }
        self.has_tx = false;
        let mut sync_map = FxHashMap::default();
        let use_sync_map = !self.clear_whole_cache
            && (!self.cache_internal_op_list.is_empty() || !self.cache_op_list.is_empty());
        for (shard_id, tx) in self.tx.drain() {
            tx.commit().await?;
            if use_sync_map {
                sync_map.insert(shard_id, Self::inc_cache_sync(shard_id).await?);
            }
        }
        self.lock_list.clear();
        let mut cache_internal_op_list = Vec::new();
        cache_internal_op_list.append(&mut self.cache_internal_op_list);
        let mut cache_op_list = Vec::new();
        cache_op_list.append(&mut self.cache_op_list);
        if self.clear_whole_cache {
            crate::clear_whole_cache().await?;
        } else {
            CacheMsg(cache_internal_op_list, sync_map.clone())
                .do_send_to_internal()
                .await;
            CacheMsg(cache_op_list, sync_map).do_send().await;
        }
        let mut fut = Vec::new();
        while let Some(cb) = self.callback_list.pop_front() {
            fut.push(cb());
        }
        if !fut.is_empty() {
            futures::future::join_all(fut).await;
        }
        Ok(())
    }

    pub async fn rollback(&mut self) -> Result<()> {
        ensure!(self.has_tx(), "No transaction is active.");
        if let Some(save_point) = self.save_point.pop() {
            self.cache_internal_op_list
                .truncate(save_point.cache_internal_op_pos);
            self.cache_op_list.truncate(save_point.cache_op_pos);
            self.callback_list.truncate(save_point.callback_post);
            for (_shard_id, tx) in self.tx.iter_mut() {
                let sql = format!("ROLLBACK TO SAVEPOINT s{}", self.save_point.len());
                tx.execute(&*sql).await?;
            }
            return Ok(());
        }
        self.has_tx = false;
        for (_shard_id, tx) in self.tx.drain() {
            tx.rollback().await?;
        }
        self.lock_list.clear();
        self.cache_internal_op_list.clear();
        self.cache_op_list.clear();
        self.callback_list.clear();
        Ok(())
    }

    pub async fn begin_read_tx(&mut self) -> Result<()> {
        if self.read_tx.is_empty() {
            let mut tx = Self::acquire_replica_tx(self.shard_id).await?;
            set_read_tx_isolation(&mut tx).await?;
            self.read_tx.insert(self.shard_id, tx);
        }
        self.has_read_tx += 1;
        Ok(())
    }

    pub fn has_read_tx(&self) -> bool {
        !self.read_tx.is_empty()
    }

    pub async fn get_read_tx(&mut self) -> Result<&mut sqlx::Transaction<'static, DbType>> {
        ensure!(!self.read_tx.is_empty(), "No transaction is active");
        match self.read_tx.entry(self.shard_id) {
            Entry::Occupied(tx) => Ok(tx.into_mut()),
            Entry::Vacant(v) => {
                let mut tx = Self::acquire_replica_tx(self.shard_id).await?;
                set_read_tx_isolation(&mut tx).await?;
                Ok(v.insert(tx))
            }
        }
    }

    pub fn release_read_tx(&mut self) -> Result<()> {
        ensure!(self.has_read_tx > 0, "No read transaction is active.");
        self.has_read_tx -= 1;
        if self.has_read_tx == 0 {
            self.read_tx.clear();
        }
        Ok(())
    }
    @%- if !config.force_disable_cache %@

    #[allow(dead_code)]
    pub(crate) async fn begin_cache_tx(&mut self) -> Result<()> {
        if self.cache_tx.is_empty() {
            let mut tx = Self::acquire_cache_tx(self.shard_id).await?;
            set_read_tx_isolation(&mut tx).await?;
            let sql = "SELECT seq FROM _sequence where id = 2";
            let sync: (u64,) = sqlx::query_as(sql).fetch_one(tx.as_mut()).await?;
            self.cache_sync = sync.0;
            self.cache_tx.insert(self.shard_id, tx);
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn has_cache_tx(&self) -> bool {
        !self.cache_tx.is_empty()
    }

    #[allow(dead_code)]
    pub(crate) async fn get_cache_tx(&mut self) -> Result<&mut sqlx::Transaction<'static, DbType>> {
        ensure!(!self.cache_tx.is_empty(), "No transaction is active");
        match self.cache_tx.entry(self.shard_id) {
            Entry::Occupied(tx) => Ok(tx.into_mut()),
            Entry::Vacant(v) => {
                let mut tx = Self::acquire_cache_tx(self.shard_id).await?;
                set_read_tx_isolation(&mut tx).await?;
                Ok(v.insert(tx))
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn release_cache_tx(&mut self) {
        self.cache_tx.clear();
    }
    @%- endif %@
    @%- if config.use_sequence %@

    pub async fn sequence(&mut self, num: u64) -> Result<u64> {
        let mut cur = SEQUENCE[self.shard_id() as usize].lock().await;
        let (seq, ceiling) = *cur;
        if seq + num > ceiling {
            let base = SEQUENCE_FETCH_NUM.load(Ordering::Relaxed) as u64;
            let fetch_num = (num / base + 1) * base;
            let sql = "UPDATE _sequence SET seq = LAST_INSERT_ID(seq + ?) where id = ?;";
            let query = sqlx::query(sql).bind(fetch_num).bind(ID_OF_SEQUENCE);
            let result = query.execute(self.acquire_source().await?.as_mut()).await?;
            let ceiling = result.last_insert_id();
            let ret = ceiling - fetch_num + 1;
            *cur = (ret + num - 1, ceiling);
            Ok(ret)
        } else {
            let ret = seq + 1;
            *cur = (seq + num, ceiling);
            Ok(ret)
        }
    }
    @%- endif %@

    pub fn empty_all_cache_sync() -> FxHashMap<ShardId, u64> {
        let mut sync_map = FxHashMap::default();
        for shard_id in DbConn::shard_num_range() {
            sync_map.insert(shard_id, 0);
        }
        sync_map
    }

    pub async fn inc_all_cache_sync() -> Result<FxHashMap<ShardId, u64>> {
        let mut sync_map = FxHashMap::default();
        for shard_id in DbConn::shard_num_range() {
            sync_map.insert(shard_id, Self::inc_cache_sync(shard_id).await?);
        }
        Ok(sync_map)
    }
    @%- if !config.force_disable_cache %@

    pub async fn inc_cache_sync(shard_id: ShardId) -> Result<u64> {
        let buf = Arc::new(AtomicU64::default());
        CACHE_SYNC_QUEUE[shard_id as usize].push(Arc::clone(&buf));
        let _lock = CACHE_SYNC[shard_id as usize].lock().await;
        let sync = buf.load(Ordering::SeqCst);
        if sync > 0 {
            return Ok(sync);
        }
        let mut list = Vec::new();
        while let Some(b) = CACHE_SYNC_QUEUE[shard_id as usize].pop() {
            list.push(b);
        }
        let sql = "UPDATE _sequence SET seq = LAST_INSERT_ID(seq + ?) where id = ?;";
        let query = sqlx::query(sql).bind(1).bind(ID_OF_CACHE_SYNC);
        let mut source = SOURCE.read().await[shard_id as usize].acquire().await?;
        let result = query.execute(source.as_mut()).await?;
        let sync = result.last_insert_id();
        for b in list {
            b.store(sync, Ordering::Release);
        }
        Ok(sync)
    }
    @%- else %@

    pub async fn inc_cache_sync(_shard_id: ShardId) -> Result<u64> {
        Ok(0)
    }
    @%- endif %@

    pub fn max_connections_for_write() -> u32 {
        MAX_CONNECTIONS_FOR_WRITE.load(Ordering::Relaxed)
    }
    pub fn max_connections_for_read() -> u32 {
        MAX_CONNECTIONS_FOR_READ.load(Ordering::Relaxed)
    }
    pub fn max_connections_for_cache() -> u32 {
        MAX_CONNECTIONS_FOR_CACHE.load(Ordering::Relaxed)
    }

    pub(crate) fn _has_update_notice() -> bool {
        NOTIFY_RECEIVER_COUNT.load(Ordering::Relaxed) > 0
    }
    pub(crate) async fn _push_update_notice(
        table: crate::models::TableName,
        op: crate::models::NotifyOp,
        id: &impl serde::Serialize,
    ) {
        let id = serde_json::to_string(id).unwrap();
        let mut notify_list = NOTIFY_LIST.write().await;
        let mut list = notify_list.take().unwrap_or_default();
        list.push((table, op, id));
        notify_list.replace(list);
    }
    pub(crate) async fn _publish_update_notice() {
        let list = NOTIFY_LIST.write().await.take();
        if let Some(list) = list {
            for f in NOTIFY_RECEIVER.read().await.iter() {
                for row in &list {
                    f(row.0, row.1, &row.2);
                }
            }
        }
    }
    @%- if config.use_update_notice %@
    pub async fn subscribe_update_notice(f: NotifyFn) {
        let mut receivers = NOTIFY_RECEIVER.write().await;
        receivers.push(f);
        NOTIFY_RECEIVER_COUNT.store(receivers.len(), Ordering::SeqCst);
    }
    @%- endif %@

    /// Obtain a lock during a transaction
    pub async fn lock(&mut self, key: &str, time: i32) -> Result<()> {
        let hash = senax_common::hash64(key);
        let mut conn = self.acquire_source().await?;
        let result: (Option<i64>,) = sqlx::query_as("SELECT GET_LOCK(?, ?)")
            .bind(hash)
            .bind(time)
            .fetch_one(conn.as_mut())
            .await?;
        if result.0 == Some(1) {
            self.lock_list.push(DbLock { conn: Some(conn) });
            Ok(())
        } else {
            Err(senax_common::err::LockFailed::new(key.to_string()).into())
        }
    }

    pub fn take_lock(&mut self) -> Option<DbLock> {
        self.lock_list.pop()
    }
}

pub struct DbLock {
    conn: Option<PoolConnection<DbType>>,
}
impl Drop for DbLock {
    fn drop(&mut self) {
        let mut conn = self.conn.take().unwrap();
        tokio::spawn(async move {
            if let Err(e) = sqlx::query("DO RELEASE_ALL_LOCKS()")
                .fetch_all(conn.as_mut())
                .await
            {
                log::error!("RELEASE_LOCK ERROR: {}", e);
            }
        });
    }
}

impl Drop for DbConn {
    fn drop(&mut self) {
        if cfg!(debug_assertions)
            && (self.has_tx() || !self.callback_list.is_empty() || !self.cache_op_list.is_empty())
        {
            log::debug!("implicit rollback");
        }
    }
}

async fn get_source(
    database_url: &str,
    user: &Option<String>,
    pw: &Option<String>,
    max_connections: u32,
) -> Result<Vec<sqlx::Pool<DbType>>> {
    let mut shards = Vec::new();
    for url in database_url.split('\n') {
        let url = url.trim();
        if url.is_empty() {
            continue;
        }
        let options: MySqlConnectOptions = url.parse()?;
        let options = if let Some(user) = user {
            options.username(user)
        } else {
            options
        };
        let options = if let Some(pw) = pw {
            options.password(pw)
        } else {
            options
        };
        let options = options.log_statements(LevelFilter::Debug);
        shards.push(
            MySqlPoolOptions::new()
                .max_connections(max_connections)
                .after_connect(|conn, _meta| {
                    Box::pin(async move {
                        if let Some(iso) = TX_ISOLATION {
                            conn.execute(&*format!(
                                "SET sql_mode=(SELECT CONCAT(@@sql_mode,',ANSI_QUOTES'));SET SESSION TRANSACTION ISOLATION LEVEL {};",
                                iso
                            ))
                            .await?;
                        } else {
                            conn.execute("SET sql_mode=(SELECT CONCAT(@@sql_mode,',ANSI_QUOTES'));").await?;
                        }
                        Ok(())
                    })
                })
                .connect_with(options)
                .await?,
        );
    }
    Ok(shards)
}

async fn get_replica(
    database_url: &str,
    user: &Option<String>,
    pw: &Option<String>,
    max_connections: u32,
) -> Result<Vec<Vec<sqlx::Pool<DbType>>>> {
    let mut shards = Vec::new();
    let re = regex::Regex::new(r"[ \t]+").unwrap();
    for urls in database_url.split('\n') {
        let urls = urls.trim();
        if urls.is_empty() {
            continue;
        }
        let mut pools = Vec::new();
        for url in re.split(urls) {
            let options: MySqlConnectOptions = url.parse()?;
            let options = if let Some(user) = user {
                options.username(user)
            } else {
                options
            };
            let options = if let Some(pw) = pw {
                options.password(pw)
            } else {
                options
            };
            let options = options.log_statements(LevelFilter::Debug);
            pools.push(
                MySqlPoolOptions::new()
                    .max_connections(max_connections)
                    .after_connect(|conn, _meta| {
                        Box::pin(async move {
                            if let Some(iso) = READ_TX_ISOLATION {
                                conn.execute(&*format!(
                                    "SET sql_mode=(SELECT CONCAT(@@sql_mode,',ANSI_QUOTES'));SET SESSION TRANSACTION ISOLATION LEVEL {}, READ ONLY;",
                                    iso
                                ))
                                .await?;
                            } else {
                                conn.execute("SET sql_mode=(SELECT CONCAT(@@sql_mode,',ANSI_QUOTES'));").await?;
                            }
                            Ok(())
                        })
                    })
                    .connect_with(options)
                    .await?,
            )
        }
        shards.push(pools);
    }
    Ok(shards)
}

#[allow(dead_code)]
pub(crate) fn is_retryable_error<T>(result: Result<T>, table: &str) -> bool {
    if let Err(err) = result {
        if let Some(err) = err.downcast_ref::<sqlx::Error>() {
            match err {
                sqlx::Error::Io(..) => {
                    // retry all
                    log::error!(table = table; "{}", err);
                    return true;
                }
                sqlx::Error::WorkerCrashed => {
                    // retry all
                    log::error!(table = table; "{}", err);
                    return true;
                }
                _ => {
                    log::error!(table = table; "{}", err);
                }
            }
        } else {
            log::error!(table = table; "{}", err);
        }
    }
    false
}

async fn set_tx_isolation(_tx: &mut Transaction<'_, DbType>) -> Result<()> {
    // postgresqlはBEGINのあとにSET TRANSACTION
    Ok(())
}

async fn set_read_tx_isolation(_tx: &mut Transaction<'_, DbType>) -> Result<()> {
    Ok(())
}
@{-"\n"}@