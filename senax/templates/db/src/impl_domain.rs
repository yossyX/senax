use anyhow::Result;
use async_trait::async_trait;
use crate::DbConn;
use domain::repository::@{ db|snake|ident }@ as _repository;
use domain::repository::@{ db|snake|ident }@::{@{ db|pascal }@QueryService, @{ db|pascal }@Repository};
use std::sync::Arc;
use tokio::sync::Mutex;

macro_rules! get_repo {
    ($n:ident, $o:ty, $c:ty) => {
        fn $n(&self) -> Box<$o> {
            Box::new(<$c>::new(self._conn.clone()))
        }
    };
}

// Do not modify below this line. (ModStart)
// Do not modify above this line. (ModEnd)

#[derive(derive_new::new)]
pub struct @{ db|pascal }@RepositoryImpl {
    _conn: Arc<Mutex<DbConn>>,
}
#[rustfmt::skip]
#[async_trait]
impl @{ db|pascal }@Repository for @{ db|pascal }@RepositoryImpl {
    async fn begin(&self) -> Result<()> {
        self._conn.lock().await.begin().await
    }
    async fn commit(&self) -> Result<()> {
        self._conn.lock().await.commit().await
    }
    async fn rollback(&self) -> Result<()> {
        self._conn.lock().await.rollback().await
    }
    async fn begin_without_transaction(&self) -> Result<()> {
        self._conn.lock().await.begin_without_transaction().await
    }
    async fn end_without_transaction(&self) -> Result<()> {
        self._conn.lock().await.end_without_transaction().await
    }
    async fn get_lock(&self, key: &str, timeout_secs: i32) -> Result<()> {
        self._conn.lock().await.lock(key, timeout_secs).await
    }
    fn should_retry(&self, err: &anyhow::Error) -> bool {
        DbConn::is_retryable_error(err)
    }
    async fn reset_tx(&self) {
        self._conn.lock().await.reset_tx()
    }
    // Do not modify below this line. (RepoStart)
    // Do not modify above this line. (RepoEnd)
}
#[rustfmt::skip]
#[async_trait]
impl @{ db|pascal }@QueryService for @{ db|pascal }@RepositoryImpl {
    async fn begin_read_tx(&self) -> Result<()> {
        self._conn.lock().await.begin_read_tx().await
    }
    async fn release_read_tx(&self) -> Result<()> {
        self._conn.lock().await.release_read_tx()
    }
    fn should_retry(&self, err: &anyhow::Error) -> bool {
        DbConn::is_retryable_error(err)
    }
    async fn reset_tx(&self) {
        self._conn.lock().await.reset_tx()
    }
    // Do not modify below this line. (QueryServiceStart)
    // Do not modify above this line. (QueryServiceEnd)
}
@{-"\n"}@