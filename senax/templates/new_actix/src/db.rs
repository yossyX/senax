use anyhow::Result;
use async_trait::async_trait;
use domain::models::Repositories;
use rand::RngCore;
use std::{path::Path, sync::Arc};
use tokio::sync::{mpsc, Mutex};

use crate::context::Ctx;

pub async fn start(
    is_hot_deploy: bool,
    exit_tx: mpsc::Sender<i32>,
    db_guard: &Arc<mpsc::Sender<u8>>,
    db_dir: &Path,
    linker_port: &Option<String>,
    pw: &Option<String>,
) -> Result<()> {
    let mut uuid_node = [0u8; 6];
    rand::thread_rng().fill_bytes(&mut uuid_node);
    let uuid_node = Some(uuid_node);

    db_session::start(
        is_hot_deploy,
        exit_tx.clone(),
        Arc::downgrade(db_guard),
        db_dir,
        linker_port,
        pw,
        &uuid_node,
    )
    .await?;
    // Do not modify this line. (DbStart)
    Ok(())
}

#[cfg(test)]
pub async fn start_test() -> Result<Vec<tokio::sync::MutexGuard<'static, u8>>> {
    let mut guard = Vec::new();
    guard.push(db_session::start_test().await?);
    // Do not modify this line. (DbStartTest)
    Ok(guard)
}

pub fn stop() {
    db_session::stop();
    // Do not modify this line. (DbStop)
}

pub async fn clear_local_cache() {
    db_session::clear_local_cache().await;
    // Do not modify this line. (DbClearLocalCache)
}

pub async fn clear_whole_cache() {
    db_session::clear_whole_cache().await;
    // Do not modify this line. (DbClearCache)
}

#[derive(Clone)]
pub struct RepositoriesImpl {
    // Do not modify this line. (Repo)
}

impl Default for RepositoriesImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[rustfmt::skip]
impl RepositoriesImpl {
    pub fn new() -> Self {
        let ctx = Ctx::new();
        Self::new_with_ctx(&ctx)
    }
    pub fn new_with_ctx(ctx: &Ctx) -> Self {
        Self {
            // Do not modify this line. (RepoNew)
        }
    }
}

#[rustfmt::skip]
#[async_trait]
impl Repositories for RepositoriesImpl {
    // Do not modify this line. (RepoImpl)
    async fn begin(&self) -> Result<()> {
        // Do not modify this line. (RepoImplStart)
        Ok(())
    }
    async fn commit(&self) -> Result<()> {
        // Do not modify this line. (RepoImplCommit)
        Ok(())
    }
    async fn rollback(&self) -> Result<()> {
        // Do not modify this line. (RepoImplRollback)
        Ok(())
    }
}

#[rustfmt::skip]
pub async fn migrate(use_test: bool, clean: bool, ignore_missing: bool) -> Result<()> {
    tokio::try_join!(
        db_session::migrate(use_test, clean, ignore_missing),
        // Do not modify this line. (migrate)
    )?;
    Ok(())
}

pub fn gen_seed_schema() -> Result<()> {
    // Do not modify this line. (gen_seed_schema)
    Ok(())
}

pub async fn seed(use_test: bool) -> Result<()> {
    tokio::try_join!(
        db_session::seeder::seed(use_test, None),
        // Do not modify this line. (seed)
    )?;
    Ok(())
}

#[rustfmt::skip]
pub async fn check(use_test: bool) -> Result<()> {
    tokio::try_join!(
        db_session::check(use_test),
        // Do not modify this line. (check)
    )?;
    Ok(())
}
@{-"\n"}@