@% if non_snake_case -%@
#![allow(non_snake_case)]
@% endif -%@
#[allow(unused_imports)]
use anyhow::{Context as _, Result};
use log::warn;
use once_cell::sync::{Lazy, OnceCell};
use senax_common::linker;
use sqlx::Row;
use std::{
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Weak,
    },
};
#[allow(unused_imports)]
use tokio::{
    sync::{mpsc, Mutex, MutexGuard, Semaphore},
    task::LocalSet,
    time::{sleep, Duration},
};

#[rustfmt::skip]
mod accessor;
@%- if !config.force_disable_cache %@
#[rustfmt::skip]
pub mod cache;
@%- endif %@
#[rustfmt::skip]
pub mod connection;
@%- if !config.excluded_from_domain %@
#[allow(clippy::module_inception)]
pub mod impl_domain;
@%- endif %@
#[rustfmt::skip]
pub mod misc;
#[rustfmt::skip]
#[allow(clippy::module_inception)]
pub mod models;
#[rustfmt::skip]
pub mod seeder;

pub(crate) use models::{CacheMsg, CacheOp};

@% if !config.force_disable_cache -%@
use cache::Cache;
@%- endif %@
pub use connection::DbConn;

#[allow(dead_code)]
const DB_NAME: &str = "@{ db }@";
#[allow(dead_code)]
const DB_UPPER_NAME: &str = "@{ db|upper_snake }@";
#[allow(dead_code)]
const DB_ID: u64 = @{ config.db_id() }@;
const IN_CONDITION_LIMIT: usize = 500;
const UNION_LIMIT: usize = 100;
@%- if !config.force_disable_cache %@
#[allow(dead_code)]
const CACHE_DB_DIR: &str = "cache/@{ db|snake }@";
@%- endif %@
const DELAYED_DB_DIR: &str = "delayed/@{ db|snake }@";
const DEFAULT_DB_MAX_CONNECTIONS_FOR_WRITE: &str = "50";
const DEFAULT_DB_MAX_CONNECTIONS_FOR_READ: &str = "100";
const DEFAULT_DB_MAX_CONNECTIONS_FOR_CACHE: &str = "50";
#[allow(dead_code)]
const DEFAULT_SEQUENCE_FETCH_NUM: &str = "1000";
const CONNECT_CHECK_INTERVAL: u64 = 10;
const CHECK_CONNECTION_TIMEOUT: u64 = 8;
const ACQUIRE_CONNECTION_WAIT_TIME: u64 = 10;

static SHUTDOWN_GUARD: OnceCell<Weak<mpsc::Sender<u8>>> = OnceCell::new();
static EXIT: OnceCell<mpsc::Sender<i32>> = OnceCell::new();
static SYS_STOP: AtomicBool = AtomicBool::new(false);
static TEST_MODE: AtomicBool = AtomicBool::new(false);
static BULK_INSERT_MAX_SIZE: OnceCell<usize> = OnceCell::new();
static LINKER_SENDER: OnceCell<linker::Sender<CacheMsg>> = OnceCell::new();
static UUID_NODE: OnceCell<[u8; 6]> = OnceCell::new();

#[allow(unused_variables)]
pub async fn start(
    is_hot_deploy: bool,
    exit_tx: mpsc::Sender<i32>,
    guard: Weak<mpsc::Sender<u8>>,
    db_dir: &Path,
    linker_port: &Option<String>,
    pw: &Option<String>,
    uuid_node: &Option<[u8; 6]>,
) -> Result<()> {
    SHUTDOWN_GUARD
        .set(guard)
        .expect("SHUTDOWN_GUARD duplicate setting error");
    EXIT.set(exit_tx.clone())
        .expect("EXIT duplicate setting error");
    TEST_MODE.store(false, Ordering::SeqCst);
    connection::init().await?;

    if let Some(uuid_node) = uuid_node {
        UUID_NODE.set(*uuid_node).unwrap();
    }

    set_bulk_insert_max_size().await?;
    @%- if !config.force_disable_cache %@

    #[cfg(not(feature = "cache_update_only"))]
    Cache::start(
        is_hot_deploy,
        Some(&db_dir.join(CACHE_DB_DIR)),
        models::USE_FAST_CACHE,
        models::USE_STORAGE_CACHE,
    )?;
    @%- endif %@

    models::start(db_dir).await?;
    @%- if !config.force_disable_cache %@
    let sync_map = DbConn::inc_all_cache_sync().await;
    models::_clear_cache(&sync_map, false).await;

    if let Some(port) = linker_port {
        let pw = pw.as_ref().with_context(|| "LINKER_PASSWORD required")?;
        let send_only = cfg!(feature = "cache_update_only");
        let (sender, mut receiver) = linker::link(DB_ID, port, pw, exit_tx.clone(), send_only)?;
        LINKER_SENDER.set(sender).unwrap();
        tokio::spawn(async move {
            while let Some(data) = receiver.recv().await {
                if let Some(data) = data {
                    match data {
                        Ok(msg) => msg.handle_cache_msg().await,
                        Err(e) => warn!("{}", e),
                    }
                } else {
                    warn!("cache clear received");
                    tokio::spawn(async move {
                        let sync_map = DbConn::inc_all_cache_sync().await;
                        models::_clear_cache(&sync_map, false).await;
                    });
                }
            }
            let _ = exit_tx.try_send(1);
        });
    }
    @%- endif %@
    Ok(())
}

async fn set_bulk_insert_max_size() -> Result<(), anyhow::Error> {
    if BULK_INSERT_MAX_SIZE.get().is_none() {
        let conn = DbConn::_new(0);
        let mut writer = conn.acquire_writer().await?;
        let row = sqlx::query("SHOW VARIABLES LIKE 'max_allowed_packet';")
            .fetch_one(writer.as_mut())
            .await?;
        let max_allowed_packet: String = row.get(1);
        BULK_INSERT_MAX_SIZE
            .set(max_allowed_packet.parse::<usize>()? / 8)
            .unwrap();
    }
    Ok(())
}

pub async fn start_test() -> Result<MutexGuard<'static, u8>> {
    static TEST_LOCK: Lazy<Mutex<u8>> = Lazy::new(|| Mutex::new(0));
    let guard = TEST_LOCK.lock().await;
    TEST_MODE.store(true, Ordering::SeqCst);
    migrate(true, true, false).await?;
    set_bulk_insert_max_size().await?;
    models::start_test().await?;
    Ok(guard)
}

pub async fn migrate(use_test: bool, clean: bool, ignore_missing: bool) -> Result<()> {
    connection::reset_database(use_test, clean).await?;
    if use_test {
        connection::init_test().await?;
    } else {
        connection::init().await?;
    }
    if clean {
        models::_clear_cache(&DbConn::inc_all_cache_sync().await, true).await;
    }
    let mut join_set = tokio::task::JoinSet::new();
    for shard_id in DbConn::shard_num_range() {
        join_set.spawn_local(async move { models::exec_migrate(shard_id, ignore_missing).await });
    }
    while let Some(res) = join_set.join_next().await {
        res??;
    }
    Ok(())
}

pub async fn check(use_test: bool) -> Result<()> {
    if use_test {
        connection::init_test().await?;
    } else {
        connection::init().await?;
    }
    models::check().await?;
    Ok(())
}

pub fn stop() {
    SYS_STOP.store(true, Ordering::SeqCst);
    @%- if !config.force_disable_cache %@
    Cache::stop();
    @%- endif %@
}

#[allow(dead_code)]
pub(crate) fn is_stopped() -> bool {
    SYS_STOP.load(Ordering::SeqCst)
}

pub(crate) fn get_shutdown_guard() -> Option<Arc<mpsc::Sender<u8>>> {
    if let Some(guard) = SHUTDOWN_GUARD.get() {
        guard.upgrade()
    } else {
        warn!("SHUTDOWN_GUARD lost!");
        None
    }
}

pub fn exit(code: i32) {
    if let Some(exit_tx) = EXIT.get() {
        let _ = exit_tx.try_send(code);
    }
}

pub fn is_test_mode() -> bool {
    TEST_MODE.load(Ordering::Relaxed)
}

pub async fn clear_local_cache() {
    let sync_map = DbConn::inc_all_cache_sync().await;
    models::_clear_cache(&sync_map, false).await;
}

pub async fn clear_whole_cache() {
    CacheMsg(vec![CacheOp::_AllClear], DbConn::inc_all_cache_sync().await)
        .do_send()
        .await;
}

pub(crate) fn db_options_for_write() -> sqlx::pool::PoolOptions<connection::DbType> {
    sqlx::pool::PoolOptions::new()
        .acquire_timeout(Duration::from_secs(5))
        .max_connections(DbConn::max_connections_for_write())
}

pub(crate) fn db_options_for_read() -> sqlx::pool::PoolOptions<connection::DbType> {
    sqlx::pool::PoolOptions::new()
        .acquire_timeout(Duration::from_secs(5))
        .max_connections(DbConn::max_connections_for_read())
}

pub(crate) fn db_options_for_cache() -> sqlx::pool::PoolOptions<connection::DbType> {
    sqlx::pool::PoolOptions::new()
        .acquire_timeout(Duration::from_secs(5))
        .max_connections(DbConn::max_connections_for_cache())
}
@{-"\n"}@