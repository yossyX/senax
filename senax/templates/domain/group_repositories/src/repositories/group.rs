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
// Do not modify above this line. (ModEnd)
pub use crate::repositories as _super;

#[rustfmt::skip]
#[cfg_attr(any(feature = "mock", test), mockall::automock)]
#[allow(non_snake_case)]
pub trait @{ group_name|pascal }@Repository: Send + Sync {
    fn _super(&self) -> Box<dyn super::Repository_>;
    // Do not modify below this line. (RepoStart)
    // Do not modify above this line. (RepoEnd)
}

impl From<&dyn @{ group_name|pascal }@Repository> for Box<dyn super::Repository_> {
    fn from(value: &dyn @{ group_name|pascal }@Repository) -> Self {
        value._super()
    }
}

#[rustfmt::skip]
#[cfg_attr(any(feature = "mock", test), mockall::automock)]
#[allow(non_snake_case)]
pub trait @{ group_name|pascal }@QueryService: Send + Sync {
    fn _super(&self) -> Box<dyn super::QueryService_>;
    // Do not modify below this line. (QueryServiceStart)
    // Do not modify above this line. (QueryServiceEnd)
}

impl From<&dyn @{ group_name|pascal }@QueryService> for Box<dyn super::QueryService_> {
    fn from(value: &dyn @{ group_name|pascal }@QueryService) -> Self {
        value._super()
    }
}

#[cfg(any(feature = "mock", test))]
#[derive(derive_new::new, Clone)]
pub struct Emu@{ group_name|pascal }@Repository {
    _repo: Arc<Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

#[rustfmt::skip]
#[cfg(any(feature = "mock", test))]
impl @{ group_name|pascal }@Repository for Emu@{ group_name|pascal }@Repository {
    get_emu_repo!(_super, dyn super::Repository_, super::EmuRepository_);
    // Do not modify below this line. (EmuRepoStart)
    // Do not modify above this line. (EmuRepoEnd)
}

#[cfg(any(feature = "mock", test))]
#[derive(derive_new::new, Clone)]
pub struct Emu@{ group_name|pascal }@QueryService {
    _repo: Arc<Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
}

#[rustfmt::skip]
#[cfg(any(feature = "mock", test))]
impl @{ group_name|pascal }@QueryService for Emu@{ group_name|pascal }@QueryService {
    get_emu_repo!(_super, dyn super::QueryService_, super::EmuQueryService_);
    // Do not modify below this line. (EmuQueryServiceStart)
    // Do not modify above this line. (EmuQueryServiceEnd)
}
@{-"\n"}@
