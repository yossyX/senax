use actix::{Actor, Addr, ArbiterHandle, AsyncContext, Context, Handler, Message, WrapFuture};
use anyhow::{Context as _, Result};
use cache::Cache;
use futures::TryStreamExt;
use log::{error, warn};
use once_cell::sync::{Lazy, OnceCell};
use senax_common::{cache::msec::MSec, linker::LinkerClient, ShardId};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Weak,
    },
};
use tokio::{
    sync::{
        mpsc::{self, UnboundedSender},
        Semaphore,
    },
    task::LocalSet,
};
use tokio::{
    sync::{Mutex, MutexGuard},
    time::{sleep, Duration},
};

mod accessor;
pub mod cache;
pub mod connection;
pub mod misc;
pub mod seeder;
@% for (name, defs) in groups %@
#[allow(clippy::module_inception)]
pub mod @{ name|to_var_name }@;
@%- endfor %@

pub use connection::DbConn;
use connection::{DbConnection, DbType, REPLICA_MAX_CONNECTIONS};

#[allow(dead_code)]
const DB_NAME: &str = "@{ db }@";
const DB_UPPER_NAME: &str = "@{ db|upper }@";
const DB_NO: u64 = @{ config.db_no(db) }@;
const IN_CONDITION_LIMIT: usize = 1000;
const BULK_FETCH_RATE: usize = 20;
const CACHE_DELAY_SAFETY1: u64 = 1; // replica time lag
const CACHE_DELAY_SAFETY2: u64 = 5; // DB query time lag
const CACHE_DB_DIR: &str = "cache/@{ db }@";
const DELAYED_DB_DIR: &str = "delayed/@{ db }@";
const DEFAULT_DB_MAX_CONNECTIONS: &str = "10";
const DEFAULT_REPLICA_DB_MAX_CONNECTIONS: &str = "10";
const DEFAULT_CACHE_DB_MAX_CONNECTIONS: &str = "10";

static SHUTDOWN_GUARD: OnceCell<Weak<mpsc::Sender<u8>>> = OnceCell::new();
static SYS_STOP: AtomicBool = AtomicBool::new(false);
static BULK_FETCH_SEMAPHORE: OnceCell<Vec<Semaphore>> = OnceCell::new();
static BULK_INSERT_MAX_SIZE: OnceCell<usize> = OnceCell::new();
static LINKER_SENDER: OnceCell<UnboundedSender<Vec<u8>>> = OnceCell::new();

pub async fn start(
    handle: &ArbiterHandle,
    is_hot_deploy: bool,
    exit_tx: mpsc::Sender<i32>,
    guard: Weak<mpsc::Sender<u8>>,
    db_dir: &Path,
    linker_port: &Option<String>,
    pw: &Option<String>,
) -> Result<()> {
    SHUTDOWN_GUARD.set(guard).unwrap();
    connection::init().await?;

    set_bulk_fetch_lane();
    set_bulk_insert_max_size().await?;

    let actor = CacheActor {};
    let addr = CacheActor::start_in_arbiter(handle, |_| actor);
    CACHE_ADDR.set(addr).unwrap();

    let disable_cache = std::env::var("DISABLE_@{ db|upper }@_CACHE")
        .unwrap_or_else(|_| "false".to_owned())
        .parse::<bool>()
        .unwrap_or_else(|e| panic!("DISABLE_@{ db|upper }@_CACHE has an error:{:?}", e));

    if !disable_cache {
        Cache::start(is_hot_deploy, Some(&db_dir.join(CACHE_DB_DIR)), @{ config.use_fast_cache()|if_then_else("true", "false") }@, true)?;
    }

@% for (name, defs) in groups  %@    @{ name|to_var_name }@::start(Some(handle), Some(db_dir)).await?;
@% endfor %@
    if let Some(port) = linker_port {
        let pw = pw
            .as_ref()
            .with_context(|| "LINKER_PASSWORD required")?
            .to_owned();
        let (to_linker, mut from_linker) =
            LinkerClient::start(port, DB_NO, pw, exit_tx.clone(), disable_cache)?;
        LINKER_SENDER.set(to_linker).unwrap();
        handle.spawn(async move {
            while let Some(data) = from_linker.recv().await {
                if data.is_empty() {
                    warn!("cache clear received");
                    tokio::spawn(async move {
                        sleep(Duration::from_secs(CACHE_DELAY_SAFETY1)).await;
                        _clear_cache();
                    });
                } else {
                    match CacheMsg::decode(&data) {
                        Ok(msg) => msg.handle_cache_msg(true).await,
                        Err(e) => warn!("{}", e),
                    }
                }
            }
            let _ = exit_tx.try_send(1);
        });
    }
    Ok(())
}

fn set_bulk_fetch_lane() {
    if BULK_FETCH_SEMAPHORE.get().is_none() {
        let bulk_fetch_lane =
            std::cmp::max(1, *REPLICA_MAX_CONNECTIONS as usize * BULK_FETCH_RATE / 100);
        BULK_FETCH_SEMAPHORE
            .set(
                DbConn::shard_num_range()
                    .map(|_| Semaphore::new(bulk_fetch_lane))
                    .collect(),
            )
            .unwrap();
    }
}

async fn set_bulk_insert_max_size() -> Result<(), anyhow::Error> {
    if BULK_INSERT_MAX_SIZE.get().is_none() {
        let conn = DbConn::_new(0);
        let mut conn = conn.acquire_source().await?;
        let row = sqlx::query("SHOW VARIABLES LIKE 'max_allowed_packet';")
            .fetch_one(&mut conn)
            .await?;
        let max_allowed_packet: String = row.get(1);
        BULK_INSERT_MAX_SIZE
            .set(max_allowed_packet.parse::<usize>()? / 8)
            .unwrap();
    }
    Ok(())
}

static TEST_LOCK: Lazy<Mutex<u8>> = Lazy::new(|| Mutex::new(0));

pub async fn start_test() -> Result<MutexGuard<'static, u8>> {
    let guard = TEST_LOCK.lock().await;
    migrate(true, true).await?;

    set_bulk_fetch_lane();
    set_bulk_insert_max_size().await?;

@% for (name, defs) in groups  %@    @{ name|to_var_name }@::start(None, None).await?;
@% endfor %@
    _clear_cache();
    Ok(guard)
}

pub async fn migrate(use_test: bool, clean: bool) -> Result<()> {
    if clean {
        let shards = connection::get_migrate_connections(use_test).await?;
        let local = LocalSet::new();
        for shard in shards {
            local.spawn_local(async move {
                if let Err(err) = exec_clean(shard.0, shard.1).await {
                    eprintln!("{}", err);
                }
            });
        }
        local.await;
        _clear_cache();
    }

    if use_test {
        connection::init_test().await?;
    } else {
        connection::init().await?;
    }
    let local = LocalSet::new();
    for shard_id in DbConn::shard_num_range() {
        local.spawn_local(async move {
            if let Err(err) = exec_migrate(shard_id).await {
                eprintln!("{}", err);
            }
        });
    }
    local.await;
    Ok(())
}

#[rustfmt::skip]
async fn exec_clean(mut conn: DbConnection, db_name: String) -> Result<()> {
    exec_ddl(&format!("DROP DATABASE IF EXISTS `{db_name}`;"), &mut conn).await?;
    exec_ddl(&format!("CREATE DATABASE `{db_name}`@{ config.get_sql_character_set() }@@{ config.get_sql_collate() }@;"), &mut conn).await
}

async fn exec_ddl<'c, E>(sql: &str, conn: E) -> Result<()>
where
    E: sqlx::Executor<'c, Database = DbType>,
{
    let mut s = conn.execute_many(sql);
    while s.try_next().await?.is_some() {}
    Ok(())
}

async fn exec_migrate(shard_id: ShardId) -> Result<()> {
    let conn = DbConn::_new(shard_id);
    let mut source = conn.acquire_source().await?;
    exec_ddl(
        r#"
            CREATE TABLE IF NOT EXISTS _sqlx_migrations (
                version BIGINT PRIMARY KEY,
                description TEXT NOT NULL,
                installed_on DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                success BOOLEAN NOT NULL,
                checksum BLOB NOT NULL,
                execution_time BIGINT NOT NULL
            );
        "#,
        &mut source,
    )
    .await?;
    sqlx::migrate!().run(&mut source).await?;
    Ok(())
}

#[rustfmt::skip]
pub async fn check(use_test: bool) -> Result<()> {
    if use_test {
        connection::init_test().await?;
    } else {
        connection::init().await?;
    }
    for shard_id in DbConn::shard_num_range() {
        tokio::try_join!(
            @%- for (name, defs) in groups  %@
            @{ name|to_var_name }@::check(shard_id),
            @%- endfor %@
        )?;
    }
    Ok(())
}

pub fn stop() {
    SYS_STOP.store(true, Ordering::SeqCst);
    Cache::stop();
}

pub(crate) fn is_stopped() -> bool {
    SYS_STOP.load(Ordering::SeqCst)
}

pub(crate) fn get_shutdown_guard() -> Option<Arc<mpsc::Sender<u8>>> {
    if let Some(guard) = SHUTDOWN_GUARD.get() {
        guard.upgrade()
    } else {
        None
    }
}

static CACHE_ADDR: OnceCell<Addr<CacheActor>> = OnceCell::new();

pub(crate) struct CacheActor;
impl Actor for CacheActor {
    type Context = Context<Self>;
}

#[derive(Message, Deserialize, Serialize, Clone, Debug)]
#[rtype(result = "()")]
pub(crate) struct CacheMsg(Vec<CacheOp>, MSec);

#[allow(clippy::large_enum_variant)]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) enum CacheOp {
@%- for (name, defs) in groups %@
    @{ name|to_pascal_name }@(@{ name|to_var_name }@::CacheOp),
@%- endfor %@
    _AllClear,
}

impl CacheMsg {
    pub(crate) async fn handle_cache_msg(self, propagated: bool) {
        for op in self.0.into_iter() {
            match op {
@%- for (name, defs) in groups %@
                CacheOp::@{ name|to_pascal_name }@(op) => op.handle_cache_msg(self.1, propagated).await,
@%- endfor %@
                CacheOp::_AllClear => _clear_cache(),
            };
        }
    }

    pub(crate) async fn do_send(self) {
        if let Some(addr) = CACHE_ADDR.get() {
            addr.do_send(self);
        } else {
            self.handle_cache_msg(false).await;
        }
    }

    fn encode(&self) -> Result<Vec<u8>> {
        Ok(serde_cbor::to_vec(self)?)
    }
    fn decode(v: &[u8]) -> Result<Self> {
        Ok(serde_cbor::from_slice::<Self>(v)?)
    }
}

#[rustfmt::skip]
impl Handler<CacheMsg> for CacheActor {
    type Result = ();

    fn handle(&mut self, msg: CacheMsg, ctx: &mut Self::Context) -> Self::Result {
        ctx.spawn(
            async move {
                let _guard = get_shutdown_guard();
                if let Some(linker) = LINKER_SENDER.get() {
                    match msg.encode() {
                        Ok(msg) => {
                            if let Err(e) = linker.send(msg) {
                                error!("{}", e);
                            }
                        }
                        Err(e) => error!("{}", e),
                    }
                }
                msg.handle_cache_msg(false).await;
            }
            .into_actor(self),
        );
    }
}

pub async fn clear_cache() {
    CacheMsg(vec![CacheOp::_AllClear], MSec::now())
        .do_send()
        .await;
}

pub(crate) fn _clear_cache() {
    Cache::invalidate_all();
@%- for (name, defs) in groups %@
    @{ name|to_var_name }@::clear_cache_all();
@%- endfor %@
}
@{-"\n"}@