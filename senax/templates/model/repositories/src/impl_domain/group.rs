use db::DbConn;
use domain::repository::@{ db|snake|to_var_name }@::@{ base_group_name|snake|to_var_name }@::_super::@{ group_name|snake|to_var_name }@::{self as _domain, @{ group_name|pascal }@QueryService, @{ group_name|pascal }@Repository};
use std::sync::Arc;
use tokio::sync::Mutex;

#[rustfmt::skip]
#[allow(clippy::map_identity)]
// Do not modify below this line. (ModStart)
// Do not modify up to this line. (ModEnd)

#[derive(derive_new::new)]
pub struct @{ group_name|pascal }@RepositoryImpl {
    _conn: Arc<Mutex<DbConn>>,
}
#[rustfmt::skip]
impl @{ group_name|pascal }@Repository for @{ group_name|pascal }@RepositoryImpl {
    get_repo!(_super, dyn _domain::_super::Repository_, super::RepositoryImpl_);
    // Do not modify below this line. (RepoStart)
    // Do not modify up to this line. (RepoEnd)
}

#[derive(derive_new::new)]
pub struct @{ group_name|pascal }@QueryServiceImpl {
    _conn: Arc<Mutex<DbConn>>,
}
#[rustfmt::skip]
impl @{ group_name|pascal }@QueryService for @{ group_name|pascal }@QueryServiceImpl {
    get_repo!(_super, dyn _domain::_super::QueryService_, super::QueryServiceImpl_);
    // Do not modify below this line. (QueryServiceStart)
    // Do not modify up to this line. (QueryServiceEnd)
}
@{-"\n"}@