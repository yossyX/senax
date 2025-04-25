#[cfg(any(feature = "mock", test))]
use ::std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[rustfmt::skip]
#[allow(clippy::overly_complex_bool_expr)]
#[allow(clippy::nonminimal_bool)]
#[allow(clippy::useless_conversion)]
#[allow(clippy::only_used_in_recursion)]
#[allow(clippy::map_identity)]
#[allow(clippy::collapsible_if)]
// Do not modify below this line. (ModStart)
// Do not modify up to this line. (ModEnd)
pub use crate::repositories as _super;

#[rustfmt::skip]
#[cfg_attr(any(feature = "mock", test), mockall::automock)]
pub trait @{ group_name|pascal }@Repository: Send + Sync {
    // Do not modify below this line. (RepoStart)
    // Do not modify up to this line. (RepoEnd)
}

#[rustfmt::skip]
#[cfg_attr(any(feature = "mock", test), mockall::automock)]
pub trait @{ group_name|pascal }@QueryService: Send + Sync {
    // Do not modify below this line. (QueryServiceStart)
    // Do not modify up to this line. (QueryServiceEnd)
}

#[cfg(any(feature = "mock", test))]
#[derive(derive_new::new)]
pub struct Emu@{ group_name|pascal }@Repository {
    _repo: Arc<Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

#[rustfmt::skip]
#[cfg(any(feature = "mock", test))]
impl @{ group_name|pascal }@Repository for Emu@{ group_name|pascal }@Repository {
    // Do not modify below this line. (EmuRepoStart)
    // Do not modify up to this line. (EmuRepoEnd)
}

#[cfg(any(feature = "mock", test))]
#[derive(derive_new::new)]
pub struct Emu@{ group_name|pascal }@QueryService {
    _repo: Arc<Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

#[rustfmt::skip]
#[cfg(any(feature = "mock", test))]
impl @{ group_name|pascal }@QueryService for Emu@{ group_name|pascal }@QueryService {
    // Do not modify below this line. (EmuQueryServiceStart)
    // Do not modify up to this line. (EmuQueryServiceEnd)
}
@{-"\n"}@
