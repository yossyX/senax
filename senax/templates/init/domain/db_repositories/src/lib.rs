use ::anyhow::Result;
use ::async_trait::async_trait;
#[rustfmt::skip]
#[cfg(any(feature = "mock", test))]
use ::std::{sync::{atomic::{AtomicIsize, Ordering}, Arc, Mutex}, any::{TypeId, Any}, collections::HashMap};

// Do not modify below this line. (ModStart)
// Do not modify up to this line. (ModEnd)

#[cfg_attr(any(feature = "mock", test), mockall::automock)]
#[async_trait]
pub trait @{ db|pascal }@Repository: Send + Sync {
    async fn begin(&self) -> Result<()>;
    async fn commit(&self) -> Result<()>;
    async fn rollback(&self) -> Result<()>;
    async fn begin_without_transaction(&self) -> Result<()>;
    async fn end_without_transaction(&self) -> Result<()>;
    async fn get_lock(&self, key: &str, timeout_secs: i32) -> Result<()>;
    fn should_retry(&self, err: &anyhow::Error) -> bool;
    async fn reset_tx(&self);
    // Do not modify below this line. (RepoStart)
    // Do not modify up to this line. (RepoEnd)
}

#[cfg_attr(any(feature = "mock", test), mockall::automock)]
#[async_trait]
pub trait @{ db|pascal }@QueryService: Send + Sync {
    async fn begin_read_tx(&self) -> Result<()>;
    async fn release_read_tx(&self) -> Result<()>;
    fn should_retry(&self, err: &anyhow::Error) -> bool;
    async fn reset_tx(&self);
    // Do not modify below this line. (QueryServiceStart)
    // Do not modify up to this line. (QueryServiceEnd)
}

#[cfg(any(feature = "mock", test))]
#[derive(Clone)]
pub struct Emu@{ db|pascal }@Repository {
    tx: Arc<Checker>,
    wo_tx: Arc<Checker>,
    read_tx: Arc<Checker>,
    repo: Arc<Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}
#[cfg(any(feature = "mock", test))]
impl Default for Emu@{ db|pascal }@Repository {
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
impl Emu@{ db|pascal }@Repository {
    pub fn new() -> Self {
        Self {
            tx: Arc::new(Checker(AtomicIsize::new(0), "commits")),
            wo_tx: Arc::new(Checker(AtomicIsize::new(0), "end_without_transaction")),
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

#[cfg(any(feature = "mock", test))]
macro_rules! get_emu_group {
    ($n:ident, $o:ty, $i:ty) => {
        fn $n(&self) -> Box<$o> {
            Box::new(<$i>::new(Arc::clone(&self.repo)))
        }
    };
}

#[rustfmt::skip]
#[cfg(any(feature = "mock", test))]
#[async_trait]
impl @{ db|pascal }@Repository for Emu@{ db|pascal }@Repository {
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
    async fn end_without_transaction(&self) -> Result<()> {
        self.wo_tx.release();
        Ok(())
    }
    async fn get_lock(&self, _key: &str, _timeout_secs: i32) -> Result<()> {
        Ok(())
    }
    fn should_retry(&self, _err: &anyhow::Error) -> bool {
        false
    }
    async fn reset_tx(&self) {}
    // Do not modify below this line. (EmuRepoStart)
    // Do not modify up to this line. (EmuRepoEnd)
}
#[rustfmt::skip]
#[cfg(any(feature = "mock", test))]
#[async_trait]
impl @{ db|pascal }@QueryService for Emu@{ db|pascal }@Repository {
    async fn begin_read_tx(&self) -> Result<()> {
        self.read_tx.acquire();
        Ok(())
    }
    async fn release_read_tx(&self) -> Result<()> {
        self.read_tx.release();
        Ok(())
    }
    fn should_retry(&self, _err: &anyhow::Error) -> bool {
        false
    }
    async fn reset_tx(&self) {}
    // Do not modify below this line. (EmuQueryServiceStart)
    // Do not modify up to this line. (EmuQueryServiceEnd)
}
@{-"\n"}@