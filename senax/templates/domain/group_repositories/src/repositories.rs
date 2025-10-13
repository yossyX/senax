#[cfg(any(feature = "mock", test))]
use ::std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[cfg(any(feature = "mock", test))]
macro_rules! get_emu_repo {
    ($n:ident, $o:ty, $i:ty) => {
        fn $n(&self) -> Box<$o> {
            let mut repo = self._repo.lock().unwrap();
            let repo = repo
                .entry(::std::any::TypeId::of::<$i>())
                .or_insert_with(|| Box::new(<$i>::new(::std::sync::Arc::clone(&self._repo))));
            Box::new(repo.downcast_ref::<$i>().unwrap().clone())
        }
    };
}

#[cfg(any(feature = "mock", test))]
macro_rules! get_emu_table {
    ($n:ident, $o:ty, $i:ty) => {
        fn $n(&self) -> Box<$o> {
            let mut repo = self._repo.lock().unwrap();
            let repo = repo
                .entry(::std::any::TypeId::of::<$i>())
                .or_insert_with(|| Box::new(<$i>::new(::std::sync::Arc::clone(&self._repo), Default::default())));
            Box::new(repo.downcast_ref::<$i>().unwrap().clone())
        }
    };
}

// Do not modify below this line. (ModStart)
// Do not modify above this line. (ModEnd)

#[cfg_attr(any(feature = "mock", test), mockall::automock)]
pub trait Repository_: Send + Sync {
    // Do not modify below this line. (RepoStart)
    // Do not modify above this line. (RepoEnd)
}

#[cfg_attr(any(feature = "mock", test), mockall::automock)]
pub trait QueryService_: Send + Sync {
    // Do not modify below this line. (QueryServiceStart)
    // Do not modify above this line. (QueryServiceEnd)
}

#[cfg(any(feature = "mock", test))]
#[derive(derive_new::new, Clone, Default)]
pub struct EmuRepository_ {
    _repo: Arc<Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

#[rustfmt::skip]
#[cfg(any(feature = "mock", test))]
impl Repository_ for EmuRepository_ {
    // Do not modify below this line. (EmuRepoStart)
    // Do not modify above this line. (EmuRepoEnd)
}

#[cfg(any(feature = "mock", test))]
#[derive(derive_new::new, Clone, Default)]
pub struct EmuQueryService_ {
    _repo: Arc<Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

#[rustfmt::skip]
#[cfg(any(feature = "mock", test))]
impl QueryService_ for EmuQueryService_ {
    // Do not modify below this line. (EmuQueryServiceStart)
    // Do not modify above this line. (EmuQueryServiceEnd)
}
@{-"\n"}@
