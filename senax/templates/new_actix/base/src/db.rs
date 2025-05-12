use anyhow::Result;
use async_trait::async_trait;
use domain::repository::Repository;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::context::Ctx;

#[allow(dead_code)]
pub async fn clear_local_cache() {
    @%- if session %@
    _db_session::clear_local_cache().await;
    @%- endif %@
    // Do not modify this line. (DbClearLocalCache)
}

pub async fn clear_whole_cache() {
    @%- if session %@
    _db_session::clear_whole_cache().await;
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

// Do not modify this line. (RepoStatic)

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
@{-"\n"}@