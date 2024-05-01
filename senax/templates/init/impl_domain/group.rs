use crate::DbConn;
use domain::models::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::{self as _domain, @{ group_name|pascal }@Queries, @{ group_name|pascal }@Repositories};
use std::sync::Arc;
use tokio::sync::Mutex;

#[rustfmt::skip]
#[allow(clippy::map_identity)]
// Do not modify below this line. (ModStart)
// Do not modify up to this line. (ModEnd)

#[derive(derive_new::new)]
pub struct @{ group_name|pascal }@RepositoriesImpl {
    _conn: Arc<Mutex<DbConn>>,
}
#[rustfmt::skip]
impl @{ group_name|pascal }@Repositories for @{ group_name|pascal }@RepositoriesImpl {
    // Do not modify below this line. (RepoStart)
    // Do not modify up to this line. (RepoEnd)
}

#[derive(derive_new::new)]
pub struct @{ group_name|pascal }@QueriesImpl {
    _conn: Arc<Mutex<DbConn>>,
}
#[rustfmt::skip]
impl @{ group_name|pascal }@Queries for @{ group_name|pascal }@QueriesImpl {
    // Do not modify below this line. (QueriesStart)
    // Do not modify up to this line. (QueriesEnd)
}
@{-"\n"}@