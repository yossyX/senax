use ::anyhow::Result;
use ::async_trait::async_trait;
#[rustfmt::skip]
#[cfg(any(feature = "mock", test))]
use ::std::{sync::{atomic::{AtomicIsize, Ordering}, Arc, Mutex}, any::{TypeId, Any}, collections::HashMap};

// Do not modify below this line. (ModStart)
// Do not modify up to this line. (ModEnd)

#[cfg_attr(any(feature = "mock", test), mockall::automock)]
#[async_trait]
pub trait @{ db|pascal }@Repositories: Send + Sync {
    async fn begin(&self) -> Result<()>;
    async fn commit(&self) -> Result<()>;
    async fn rollback(&self) -> Result<()>;
    async fn begin_without_transaction(&self) -> Result<()>;
    async fn end_of_without_transaction(&self) -> Result<()>;
    async fn lock(&self, key: &str, time: i32) -> Result<()>;
    // Do not modify below this line. (RepoStart)
    // Do not modify up to this line. (RepoEnd)
}

#[cfg_attr(any(feature = "mock", test), mockall::automock)]
#[async_trait]
pub trait @{ db|pascal }@Queries: Send + Sync {
    async fn begin_read_tx(&self) -> Result<()>;
    async fn release_read_tx(&self) -> Result<()>;
    // Do not modify below this line. (QueriesStart)
    // Do not modify up to this line. (QueriesEnd)
}

#[cfg(any(feature = "mock", test))]
#[derive(Clone)]
pub struct Emu@{ db|pascal }@Repositories {
    tx: Arc<Checker>,
    wo_tx: Arc<Checker>,
    read_tx: Arc<Checker>,
    repo: Arc<Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}
#[cfg(any(feature = "mock", test))]
impl Default for Emu@{ db|pascal }@Repositories {
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(any(feature = "mock", test))]
struct Checker(AtomicIsize, &'static str);
#[cfg(any(feature = "mock", test))]
impl Checker {
    fn acquire(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
    fn release(&self) {
        assert!(
            self.0.fetch_sub(1, Ordering::SeqCst) > 0,
            "Too many {}",
            self.1
        );
    }
}
#[cfg(any(feature = "mock", test))]
impl Drop for Checker {
    fn drop(&mut self) {
        assert!(
            self.0.load(Ordering::SeqCst) == 0,
            "Insufficient {}",
            self.1
        );
    }
}
#[rustfmt::skip]
#[cfg(any(feature = "mock", test))]
impl Emu@{ db|pascal }@Repositories {
    pub fn new() -> Self {
        Self {
            tx: Arc::new(Checker(AtomicIsize::new(0), "commits")),
            wo_tx: Arc::new(Checker(AtomicIsize::new(0), "end_of_without_transaction")),
            read_tx: Arc::new(Checker(AtomicIsize::new(0), "release_read_tx")),
            repo: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    pub fn _get<T: Default + Send + Sync + Clone + 'static>(&self) -> Box<T> {
        let mut repo = self.repo.lock().unwrap();
        let repo = repo
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(<T>::default()));
        Box::new(repo.downcast_ref::<T>().unwrap().clone())
    }
}
#[rustfmt::skip]
#[cfg(any(feature = "mock", test))]
#[async_trait]
impl @{ db|pascal }@Repositories for Emu@{ db|pascal }@Repositories {
    async fn begin(&self) -> Result<()> {
        self.tx.acquire();
        Ok(())
    }
    async fn commit(&self) -> Result<()> {
        self.tx.release();
        Ok(())
    }
    async fn rollback(&self) -> Result<()> {
        self.tx.release();
        Ok(())
    }
    async fn begin_without_transaction(&self) -> Result<()> {
        self.wo_tx.acquire();
        Ok(())
    }
    async fn end_of_without_transaction(&self) -> Result<()> {
        self.wo_tx.release();
        Ok(())
    }
    async fn lock(&self, _key: &str, _time: i32) -> Result<()> {
        Ok(())
    }
    // Do not modify below this line. (EmuRepoStart)
    // Do not modify up to this line. (EmuRepoEnd)
}
#[rustfmt::skip]
#[cfg(any(feature = "mock", test))]
#[async_trait]
impl @{ db|pascal }@Queries for Emu@{ db|pascal }@Repositories {
    async fn begin_read_tx(&self) -> Result<()> {
        self.read_tx.acquire();
        Ok(())
    }
    async fn release_read_tx(&self) -> Result<()> {
        self.read_tx.release();
        Ok(())
    }
    // Do not modify below this line. (EmuQueriesStart)
    // Do not modify up to this line. (EmuQueriesEnd)
}
@{-"\n"}@