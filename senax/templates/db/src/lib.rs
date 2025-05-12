@% if non_snake_case -%@
#![allow(non_snake_case)]
@% endif -%@
#[allow(unused_imports)]
use anyhow::{Context as _, Result};
use std::{
    path::Path,
    sync::Weak,
};
#[allow(unused_imports)]
use tokio::{
    sync::{mpsc, Mutex, MutexGuard, Semaphore},
    task::LocalSet,
    time::{sleep, Duration},
};

pub use _base::*;
@%- if !config.exclude_from_domain %@
#[rustfmt::skip]
#[allow(clippy::module_inception)]
#[allow(clippy::type_complexity)]
pub mod impl_domain;
@%- endif %@
#[rustfmt::skip]
#[allow(clippy::module_inception)]
pub mod models;
#[rustfmt::skip]
#[cfg(feature = "seeder")]
pub mod seeder;

#[rustfmt::skip]
pub mod repositories {
}

#[rustfmt::skip]
pub fn init() {
}

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
    _base::_start(is_hot_deploy,exit_tx,guard,db_dir,linker_port,pw,uuid_node).await?;
    models::start(db_dir).await?;
    Ok(())
}

pub async fn start_test() -> Result<MutexGuard<'static, u8>> {
    let guard = _base::_start_test().await?;
    migrate(true, true, false).await?;
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
    let mut error = None;
    while let Some(res) = join_set.join_next().await {
        if let Err(e) = res? {
            if let Some(e) = error.replace(e) {
                log::error!("{}", e);
            }
        }
    }
    if let Some(e) = error {
        return Err(e);
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
@{-"\n"}@