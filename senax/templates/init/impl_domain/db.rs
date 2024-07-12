use anyhow::Result;
use async_trait::async_trait;
use crate::DbConn;
use domain::models::@{ db|snake|to_var_name }@::{self as _domain, @{ db|pascal }@Queries, @{ db|pascal }@Repositories};
use std::sync::Arc;
use tokio::sync::Mutex;

macro_rules! get_repo {
    ($n:ident, $o:ty, $i:ty) => {
        fn $n(&self) -> Box<$o> {
            Box::new(<$i>::new(Arc::clone(&self._conn)))
        }
    };
}

// Do not modify below this line. (ModStart)
// Do not modify up to this line. (ModEnd)

#[derive(derive_new::new)]
pub struct @{ db|pascal }@RepositoriesImpl {
    _conn: Arc<Mutex<DbConn>>,
}
#[rustfmt::skip]
#[async_trait]
impl @{ db|pascal }@Repositories for @{ db|pascal }@RepositoriesImpl {
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
    async fn end_of_without_transaction(&self) -> Result<()> {
        self._conn.lock().await.end_of_without_transaction().await
    }
    async fn get_lock(&self, key: &str, time: i32) -> Result<()> {
        self._conn.lock().await.lock(key, time).await
    }
    fn should_retry(&self, err: &anyhow::Error) -> bool {
        DbConn::is_retryable_error(err)
    }
    // Do not modify below this line. (RepoStart)
    // Do not modify up to this line. (RepoEnd)
}
#[rustfmt::skip]
#[async_trait]
impl @{ db|pascal }@Queries for @{ db|pascal }@RepositoriesImpl {
    async fn begin_read_tx(&self) -> Result<()> {
        self._conn.lock().await.begin_read_tx().await
    }
    async fn release_read_tx(&self) -> Result<()> {
        self._conn.lock().await.release_read_tx()
    }
    fn should_retry(&self, err: &anyhow::Error) -> bool {
        DbConn::is_retryable_error(err)
    }
    // Do not modify below this line. (QueriesStart)
    // Do not modify up to this line. (QueriesEnd)
}
@{-"\n"}@