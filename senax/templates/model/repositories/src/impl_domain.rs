use db::DbConn;
use domain::repository::@{ db|snake|to_var_name }@::@{ group_name|snake|to_var_name }@::_super::{self as _domain, QueryService_, Repository_};
use std::sync::Arc;
use tokio::sync::Mutex;

macro_rules! get_repo {
    ($n:ident, $o:ty, $i:ty) => {
        fn $n(&self) -> Box<$o> {
            Box::new(<$i>::new(std::sync::Arc::clone(&self._conn)))
        }
    };
}

// Do not modify below this line. (ModStart)
// Do not modify up to this line. (ModEnd)

#[derive(derive_new::new)]
pub struct RepositoryImpl_ {
    _conn: Arc<Mutex<DbConn>>,
}
#[rustfmt::skip]
impl Repository_ for RepositoryImpl_ {
    // Do not modify below this line. (RepoStart)
    // Do not modify up to this line. (RepoEnd)
}

#[derive(derive_new::new)]
pub struct QueryServiceImpl_ {
    _conn: Arc<Mutex<DbConn>>,
}
#[rustfmt::skip]
impl QueryService_ for QueryServiceImpl_ {
    // Do not modify below this line. (QueryServiceStart)
    // Do not modify up to this line. (QueryServiceEnd)
}
@{-"\n"}@