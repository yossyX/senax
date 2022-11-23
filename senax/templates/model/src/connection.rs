use anyhow::{ensure, Context as _, Result};
use futures::future::LocalBoxFuture;
use fxhash::FxHashMap;
use log::LevelFilter;
use once_cell::sync::{Lazy, OnceCell};
use senax_common::cache::msec::MSec;
use senax_common::ShardId;
use sqlx::mysql::{MySqlConnectOptions, MySqlPool, MySqlPoolOptions};
use sqlx::pool::PoolConnection;
use sqlx::{ConnectOptions, Connection, Executor, MySqlConnection, Row, Transaction};
use std::collections::hash_map::Entry;
use std::collections::VecDeque;
use std::env;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::SystemTime;
use url::Url;

use crate::{
    CacheMsg, CacheOp, DEFAULT_CACHE_DB_MAX_CONNECTIONS, DEFAULT_DB_MAX_CONNECTIONS,
    DEFAULT_REPLICA_DB_MAX_CONNECTIONS,
};

pub type DbType = sqlx::MySql;
pub type DbPool = MySqlPool;
pub type DbConnection = MySqlConnection;
pub type DbArguments = sqlx::mysql::MySqlArguments;
pub const TX_ISOLATION: Option<&'static str> = @{ tx_isolation|disp_opt }@;
pub const READ_TX_ISOLATION: Option<&'static str> = @{ read_tx_isolation|disp_opt }@;

static SOURCE: OnceCell<Vec<DbPool>> = OnceCell::new();
static REPLICA: OnceCell<Vec<Vec<DbPool>>> = OnceCell::new();
static CACHE: OnceCell<Vec<Vec<DbPool>>> = OnceCell::new();
static NEXT_REPLICA: AtomicUsize = AtomicUsize::new(0);
static NEXT_CACHE: AtomicUsize = AtomicUsize::new(0);
pub static SOURCE_MAX_CONNECTIONS: Lazy<u32> = Lazy::new(|| {
    env::var("@{ db|upper }@_DB_MAX_CONNECTIONS")
        .unwrap_or_else(|_| DEFAULT_DB_MAX_CONNECTIONS.to_owned())
        .parse()
        .unwrap_or_else(|err| panic!("{}: @{ db|upper }@_DB_MAX_CONNECTIONS", err))
});
pub static REPLICA_MAX_CONNECTIONS: Lazy<u32> = Lazy::new(|| {
    env::var("@{ db|upper }@_REPLICA_DB_MAX_CONNECTIONS")
        .unwrap_or_else(|_| DEFAULT_REPLICA_DB_MAX_CONNECTIONS.to_owned())
        .parse()
        .unwrap_or_else(|err| panic!("{}: @{ db|upper }@_REPLICA_DB_MAX_CONNECTIONS", err))
});
pub static CACHE_MAX_CONNECTIONS: Lazy<u32> = Lazy::new(|| {
    env::var("@{ db|upper }@_CACHE_DB_MAX_CONNECTIONS")
        .unwrap_or_else(|_| DEFAULT_CACHE_DB_MAX_CONNECTIONS.to_owned())
        .parse()
        .unwrap_or_else(|err| panic!("{}: @{ db|upper }@_CACHE_DB_MAX_CONNECTIONS", err))
});

pub async fn init() -> Result<()> {
    if SOURCE.get().is_some() {
        return Ok(());
    }
    let database_url =
        env::var("@{ db|upper }@_DB_URL").with_context(|| "@{ db|upper }@_DB_URL is not set in .env file")?;
    SOURCE.set(get_source(&database_url).await?).unwrap();

    let replica_url = env::var("@{ db|upper }@_REPLICA_DB_URL").unwrap_or_else(|_| database_url.clone());
    REPLICA
        .set(get_replica(&replica_url, *REPLICA_MAX_CONNECTIONS).await?)
        .unwrap();

    let cache_url = env::var("@{ db|upper }@_CACHE_DB_URL").unwrap_or_else(|_| database_url.clone());
    CACHE
        .set(get_replica(&cache_url, *CACHE_MAX_CONNECTIONS).await?)
        .unwrap();

    ensure!(
        SOURCE.get().unwrap().len() == REPLICA.get().unwrap().len(),
        "Source and replica shards have different lengths."
    );
    ensure!(
        SOURCE.get().unwrap().len() == CACHE.get().unwrap().len(),
        "Source and cache shards have different lengths."
    );
    ensure!(
        SOURCE.get().unwrap().len() <= ShardId::MAX as usize + 1,
        "Number of shards exceeds limit."
    );
    Ok(())
}

pub async fn init_test() -> Result<()> {
    if SOURCE.get().is_some() {
        return Ok(());
    }
    let database_url =
        env::var("@{ db|upper }@_TEST_DB_URL").with_context(|| "@{ db|upper }@_TEST_DB_URL is not set in .env file")?;
    SOURCE.set(get_source(&database_url).await?).unwrap();
    REPLICA
        .set(get_replica(&database_url, *REPLICA_MAX_CONNECTIONS).await?)
        .unwrap();
    CACHE
        .set(get_replica(&database_url, *CACHE_MAX_CONNECTIONS).await?)
        .unwrap();

    ensure!(
        SOURCE.get().unwrap().len() <= ShardId::MAX as usize,
        "Number of shards exceeds limit."
    );
    Ok(())
}

struct SavePoint {
    cache_op_pos: usize,
    callback_post: usize,
}
pub struct DbConn {
    time: SystemTime,
    shard_id: ShardId,
    begin_time: Option<MSec>,
    tx: FxHashMap<ShardId, sqlx::Transaction<'static, DbType>>,
    save_point: Vec<SavePoint>,
    read_tx: FxHashMap<ShardId, sqlx::Transaction<'static, DbType>>,
    cache_tx: FxHashMap<ShardId, sqlx::Transaction<'static, DbType>>,
    conn: FxHashMap<ShardId, PoolConnection<DbType>>,
    cache_op_list: Vec<CacheOp>,
    callback_list: VecDeque<Box<dyn FnOnce() -> LocalBoxFuture<'static, ()>>>,
    pub(crate) clear_all_cache: bool,
    wo_tx: bool,
}

impl DbConn {
    #[allow(clippy::new_without_default)]
    pub fn new() -> DbConn {
        DbConn {
            time: SystemTime::now(),
            shard_id: 0,
            begin_time: None,
            tx: FxHashMap::default(),
            save_point: Vec::new(),
            read_tx: FxHashMap::default(),
            cache_tx: FxHashMap::default(),
            conn: FxHashMap::default(),
            cache_op_list: Vec::new(),
            callback_list: VecDeque::new(),
            clear_all_cache: false,
            wo_tx: false,
        }
    }

    pub(crate) fn _new(shard_id: ShardId) -> DbConn {
        DbConn {
            time: SystemTime::now(),
            shard_id,
            begin_time: None,
            tx: FxHashMap::default(),
            save_point: Vec::new(),
            read_tx: FxHashMap::default(),
            cache_tx: FxHashMap::default(),
            conn: FxHashMap::default(),
            cache_op_list: Vec::new(),
            callback_list: VecDeque::new(),
            clear_all_cache: false,
            wo_tx: false,
        }
    }

    pub fn set_time(&mut self, time: SystemTime) {
        self.time = time;
    }

    pub fn time(&self) -> SystemTime {
        self.time
    }

    pub fn shard_num() -> usize {
        SOURCE.get().unwrap().len()
    }

    pub fn shard_num_range() -> std::ops::RangeInclusive<ShardId> {
        0..=(SOURCE.get().unwrap().len() - 1) as ShardId
    }

    pub fn shard_id(&self) -> ShardId {
        self.shard_id
    }

    pub fn set_shard_id(&mut self, shard_id: usize) {
        self.shard_id = (shard_id % SOURCE.get().unwrap().len()) as ShardId;
    }

    /// Cache transaction start time
    pub(crate) fn begin_time(&self) -> MSec {
        self.begin_time.unwrap()
    }

    pub fn set_clear_all_cache(&mut self) {
        self.clear_all_cache = true;
    }

    pub async fn acquire_source(&self) -> Result<PoolConnection<DbType>> {
        Ok(SOURCE.get().unwrap()[self.shard_id() as usize]
            .acquire()
            .await?)
    }

    pub async fn acquire_replica(&self) -> Result<PoolConnection<DbType>> {
        let replica = &REPLICA.get().unwrap()[self.shard_id() as usize];
        let len = replica.len();
        let index = NEXT_REPLICA.fetch_add(1, Ordering::SeqCst) % len;
        for i in 0..len {
            let idx = (i + index) % len;
            if *REPLICA_MAX_CONNECTIONS - replica[idx].size() + replica[idx].num_idle() as u32 > 0 {
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
        let replica = &REPLICA.get().unwrap()[shard_id as usize];
        let len = replica.len();
        let index = NEXT_REPLICA.fetch_add(1, Ordering::SeqCst) % len;
        for i in 0..len {
            let idx = (i + index) % len;
            if *REPLICA_MAX_CONNECTIONS - replica[idx].size() + replica[idx].num_idle() as u32 > 0 {
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

    pub(crate) async fn acquire_cache_tx(
        shard_id: ShardId,
    ) -> Result<sqlx::Transaction<'static, DbType>> {
        let cache = &CACHE.get().unwrap()[shard_id as usize];
        let len = cache.len();
        let index = NEXT_CACHE.fetch_add(1, Ordering::SeqCst) % len;
        for i in 0..len {
            let idx = (i + index) % len;
            if *CACHE_MAX_CONNECTIONS - cache[idx].size() + cache[idx].num_idle() as u32 > 0 {
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
        ensure!(!self.has_tx(), "Transaction is active.");
        self.wo_tx = true;
        Ok(())
    }

    pub async fn begin(&mut self) -> Result<()> {
        self.wo_tx = false;
        if self.tx.is_empty() {
            let mut tx = SOURCE.get().unwrap()[self.shard_id as usize]
                .begin()
                .await?;
            set_tx_isolation(&mut tx).await?;
            self.tx.insert(self.shard_id, tx);
        } else {
            for (_shard_id, tx) in self.tx.iter_mut() {
                let sql = format!("SAVEPOINT s{}", self.save_point.len());
                tx.execute(&*sql).await?;
            }
            self.save_point.push(SavePoint {
                cache_op_pos: self.cache_op_list.len(),
                callback_post: self.callback_list.len(),
            });
        }
        Ok(())
    }

    pub fn has_tx(&self) -> bool {
        !self.tx.is_empty()
    }

    pub(crate) fn wo_tx(&self) -> bool {
        self.wo_tx
    }

    pub async fn get_tx(&mut self) -> Result<&mut sqlx::Transaction<'static, DbType>> {
        ensure!(self.has_tx(), "No transaction is active.");
        match self.tx.entry(self.shard_id) {
            Entry::Occupied(tx) => Ok(tx.into_mut()),
            Entry::Vacant(v) => {
                let mut tx = SOURCE.get().unwrap()[self.shard_id as usize]
                    .begin()
                    .await?;
                set_tx_isolation(&mut tx).await?;
                for s in 0..self.save_point.len() {
                    let sql = format!("SAVEPOINT s{}", s);
                    tx.execute(&*sql).await?;
                }
                Ok(v.insert(tx))
            }
        }
    }

    pub(crate) async fn push_cache_op(&mut self, op: CacheOp) {
        if self.has_tx() {
            self.cache_op_list.push(op);
        } else {
            CacheMsg(vec![op], MSec::now()).do_send().await;
        }
    }

    pub(crate) async fn push_callback(
        &mut self,
        cb: Box<dyn FnOnce() -> LocalBoxFuture<'static, ()>>,
    ) {
        if self.has_tx() {
            self.callback_list.push_back(cb);
        } else {
            cb().await;
        }
    }

    pub async fn rows_affected(&mut self) -> Result<i64> {
        let query = sqlx::query("select row_count()");
        let row = query.fetch_one(self.get_tx().await?).await?;
        Ok(row.try_get(0)?)
    }

    pub async fn commit(&mut self) -> Result<()> {
        if let Some(_save_point) = self.save_point.pop() {
            for (_shard_id, tx) in self.tx.iter_mut() {
                let sql = format!("RELEASE SAVEPOINT s{}", self.save_point.len());
                tx.execute(&*sql).await?;
            }
            return Ok(());
        }
        for (_shard_id, tx) in self.tx.drain() {
            tx.commit().await?;
        }
        let mut cache_op_list = Vec::new();
        cache_op_list.append(&mut self.cache_op_list);
        if self.clear_all_cache {
            crate::clear_cache().await;
        } else {
            CacheMsg(cache_op_list, MSec::now()).do_send().await;
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
        if let Some(save_point) = self.save_point.pop() {
            self.cache_op_list.truncate(save_point.cache_op_pos);
            self.callback_list.truncate(save_point.callback_post);
            for (_shard_id, tx) in self.tx.iter_mut() {
                let sql = format!("ROLLBACK TO SAVEPOINT s{}", self.save_point.len());
                tx.execute(&*sql).await?;
            }
            return Ok(());
        }
        for (_shard_id, tx) in self.tx.drain() {
            tx.rollback().await?;
        }
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

    pub fn release_read_tx(&mut self) {
        self.read_tx.clear();
    }

    pub(crate) async fn begin_cache_tx(&mut self) -> Result<()> {
        if self.cache_tx.is_empty() {
            self.begin_time = Some(MSec::now());
            let mut tx = Self::acquire_cache_tx(self.shard_id).await?;
            set_read_tx_isolation(&mut tx).await?;
            self.cache_tx.insert(self.shard_id, tx);
        }
        Ok(())
    }

    pub(crate) fn has_cache_tx(&self) -> bool {
        !self.cache_tx.is_empty()
    }

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

    pub(crate) fn release_cache_tx(&mut self) {
        self.cache_tx.clear();
    }
}

impl Drop for DbConn {
    fn drop(&mut self) {
        if cfg!(debug_assertions)
            && (self.has_tx() || !self.callback_list.is_empty() || !self.cache_op_list.is_empty())
        {
            log::warn!("implicit rollback");
        }
    }
}

pub(crate) async fn get_migrate_connections(is_test: bool) -> Result<Vec<(DbConnection, String)>> {
    let database_url = if is_test {
        env::var("@{ db|upper }@_TEST_DB_URL").with_context(|| "@{ db|upper }@_TEST_DB_URL is not set in .env file")
    } else {
        env::var("@{ db|upper }@_DB_URL").with_context(|| "@{ db|upper }@_DB_URL is not set in .env file")
    }?;

    let mut shards = Vec::new();
    for url in database_url.split('\n') {
        let url = url.trim();
        if url.is_empty() {
            continue;
        }
        let parsed_url = Url::parse(url)?;
        let name = parsed_url.path().trim_start_matches('/').to_string();
        let url = url.trim_end_matches(&name);
        shards.push((DbConnection::connect(url).await?, name));
    }
    Ok(shards)
}

async fn get_source(database_url: &str) -> Result<Vec<sqlx::Pool<DbType>>> {
    let mut shards = Vec::new();
    for url in database_url.split('\n') {
        let url = url.trim();
        if url.is_empty() {
            continue;
        }
        let mut options: MySqlConnectOptions = url.parse().unwrap();
        options.log_statements(LevelFilter::Debug);
        shards.push(
            MySqlPoolOptions::new()
                .max_connections(*SOURCE_MAX_CONNECTIONS)
                .after_connect(|conn, _meta| {
                    Box::pin(async move {
                        if let Some(iso) = TX_ISOLATION {
                            conn.execute(&*format!(
                                "SET SESSION TRANSACTION ISOLATION LEVEL {};",
                                iso
                            ))
                            .await?;
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
    max_connections: u32,
) -> Result<Vec<Vec<sqlx::Pool<DbType>>>> {
    let mut shards = Vec::new();
    for urls in database_url.split('\n') {
        let urls = urls.trim();
        if urls.is_empty() {
            continue;
        }
        let mut pools = Vec::new();
        for url in urls.split(',') {
            let mut options: MySqlConnectOptions = url.parse().unwrap();
            options.log_statements(LevelFilter::Debug);
            pools.push(
                MySqlPoolOptions::new()
                    .max_connections(max_connections)
                    .after_connect(|conn, _meta| {
                        Box::pin(async move {
                            if let Some(iso) = READ_TX_ISOLATION {
                                conn.execute(&*format!(
                                    "SET SESSION TRANSACTION ISOLATION LEVEL {}, READ ONLY;",
                                    iso
                                ))
                                .await?;
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

async fn set_tx_isolation(_tx: &mut Transaction<'_, DbType>) -> Result<()> {
    // postgresqlはBEGINのあとにSET TRANSACTION
    Ok(())
}

async fn set_read_tx_isolation(_tx: &mut Transaction<'_, DbType>) -> Result<()> {
    Ok(())
}
@{-"\n"}@