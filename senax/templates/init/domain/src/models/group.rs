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

#[rustfmt::skip]
#[cfg_attr(any(feature = "mock", test), mockall::automock)]
pub trait @{ group_name|pascal }@Repositories: Send + Sync {
    // Do not modify below this line. (RepoStart)
    // Do not modify up to this line. (RepoEnd)
}

#[rustfmt::skip]
#[cfg_attr(any(feature = "mock", test), mockall::automock)]
pub trait @{ group_name|pascal }@Queries: Send + Sync {
    // Do not modify below this line. (QueriesStart)
    // Do not modify up to this line. (QueriesEnd)
}

#[cfg(any(feature = "mock", test))]
#[derive(derive_new::new)]
pub struct Emu@{ group_name|pascal }@Repositories {
    _repo: Arc<Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

#[rustfmt::skip]
#[cfg(any(feature = "mock", test))]
impl @{ group_name|pascal }@Repositories for Emu@{ group_name|pascal }@Repositories {
    // Do not modify below this line. (EmuRepoStart)
    // Do not modify up to this line. (EmuRepoEnd)
}

#[cfg(any(feature = "mock", test))]
#[derive(derive_new::new)]
pub struct Emu@{ group_name|pascal }@Queries {
    _repo: Arc<Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

#[rustfmt::skip]
#[cfg(any(feature = "mock", test))]
impl @{ group_name|pascal }@Queries for Emu@{ group_name|pascal }@Queries {
    // Do not modify below this line. (EmuQueriesStart)
    // Do not modify up to this line. (EmuQueriesEnd)
}
@{-"\n"}@