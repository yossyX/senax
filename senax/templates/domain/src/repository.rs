use ::anyhow::Result;
use ::async_trait::async_trait;

// Do not modify this line. (Mod)

// Do not modify this line. (UseRepo)

#[cfg_attr(any(feature = "mock", test), mockall::automock)]
#[async_trait]
pub trait Repository: Send + Sync {
    // Do not modify this line. (Repo)
    async fn begin(&self) -> Result<()>;
    async fn commit(&self) -> Result<()>;
    async fn rollback(&self) -> Result<()>;
}

#[cfg(any(feature = "mock", test))]
#[derive(Clone, Default)]
pub struct EmuRepository {
    // Do not modify this line. (EmuRepo)
}
#[rustfmt::skip]
#[cfg(any(feature = "mock", test))]
impl EmuRepository {
    pub fn new() -> Self {
        Self::default()
    }
}
#[rustfmt::skip]
#[cfg(any(feature = "mock", test))]
#[async_trait]
impl Repository for EmuRepository {
    // Do not modify this line. (EmuImpl)
    async fn begin(&self) -> Result<()> {
        // Do not modify this line. (EmuImplStart)
        Ok(())
    }
    async fn commit(&self) -> Result<()> {
        // Do not modify this line. (EmuImplCommit)
        Ok(())
    }
    async fn rollback(&self) -> Result<()> {
        // Do not modify this line. (EmuImplRollback)
        Ok(())
    }
}
@{-"\n"}@