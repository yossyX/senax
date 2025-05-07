use anyhow::Result;
use async_trait::async_trait;
use domain::repository::Repository;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::context::Ctx;

#[allow(dead_code)]
pub async fn clear_local_cache() {
    @%- if session %@
    db_session::clear_local_cache().await;
    @%- endif %@
    // Do not modify this line. (DbClearLocalCache)
}

pub async fn clear_whole_cache() {
    @%- if session %@
    db_session::clear_whole_cache().await;
    @%- endif %@
    // Do not modify this line. (DbClearCache)
}

#[derive(Clone)]
pub struct RepositoryImpl {
    // Do not modify this line. (Repo)
}

impl Default for RepositoryImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[rustfmt::skip]
impl RepositoryImpl {
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
impl Repository for RepositoryImpl {
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
    let mut join_set = tokio::task::JoinSet::new();
    @%- if session %@
    join_set.spawn_local(db_session::migrate(use_test, clean, ignore_missing));
    @%- endif %@
    // Do not modify this line. (migrate)
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

pub fn gen_seed_schema() -> Result<()> {
    // Do not modify this line. (gen_seed_schema)
    Ok(())
}

pub async fn seed(_use_test: bool) -> Result<()> {
    // Do not modify this line. (seed)
    Ok(())
}

#[rustfmt::skip]
pub async fn check(use_test: bool) -> Result<()> {
    tokio::try_join!(
        @%- if session %@
        db_session::check(use_test),
        @%- endif %@
        // Do not modify this line. (check)
    )?;
    Ok(())
}
@{-"\n"}@