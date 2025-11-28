use db::DbConn;
use domain::repository::@{ db|snake|ident }@::@{ group_name|snake|ident }@::_super::{self as _domain, QueryService_, Repository_};
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
// Do not modify above this line. (ModEnd)

#[derive(derive_new::new)]
pub struct RepositoryImpl_ {
    _conn: Arc<Mutex<DbConn>>,
}
#[rustfmt::skip]
impl Repository_ for RepositoryImpl_ {
    // Do not modify below this line. (RepoStart)
    // Do not modify above this line. (RepoEnd)
}

#[derive(derive_new::new)]
pub struct QueryServiceImpl_ {
    _conn: Arc<Mutex<DbConn>>,
}
#[rustfmt::skip]
impl QueryService_ for QueryServiceImpl_ {
    // Do not modify below this line. (QueryServiceStart)
    // Do not modify above this line. (QueryServiceEnd)
}
@{-"\n"}@