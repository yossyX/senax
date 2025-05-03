@%- if !config.exclude_from_domain %@
#[allow(clippy::module_inception)]
pub mod impl_domain;
@%- endif %@
#[allow(clippy::module_inception)]
pub mod repositories;
#[rustfmt::skip]
pub mod misc;

pub fn init() {
    db::models::set_@{ group|snake }@(Box::new(repositories::Controller));
@%- if !config.exclude_from_domain %@

    use db::DbConn;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    db::impl_domain::NEW_@{ group|upper }@_REPO
        .set(Box::new(|conn: &Arc<Mutex<DbConn>>| {
            Box::new(impl_domain::@{ group|snake|to_var_name }@::@{ group|pascal }@RepositoryImpl::new(conn.clone()))
        }))
        .unwrap_or_else(|_| panic!("duplicate init"));
    db::impl_domain::NEW_@{ group|upper }@_QS
        .set(Box::new(|conn: &Arc<Mutex<DbConn>>| {
            Box::new(impl_domain::@{ group|snake|to_var_name }@::@{ group|pascal }@QueryServiceImpl::new(conn.clone()))
        }))
        .unwrap_or_else(|_| panic!("duplicate init"));
@%- endif %@
}
@{-"\n"}@