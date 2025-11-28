use anyhow::Result;
use db::DbConn;
use senax_common::ShardId;
use std::path::Path;

@%- if !config.exclude_from_domain %@
#[allow(clippy::module_inception)]
pub mod impl_domain;
@%- endif %@
#[allow(clippy::module_inception)]
pub mod repositories;
#[rustfmt::skip]
pub mod misc;

pub async fn start(db_dir: &Path) -> Result<()> {
    @%- for (name, (_, defs)) in groups %@
    repositories::@{ name|snake|ident }@::start(Some(db_dir)).await?;
    @%- endfor %@
    Ok(())
}
pub async fn start_test() -> Result<()> {
    @%- for (name, (_, defs)) in groups %@
    repositories::@{ name|snake|ident }@::start(None).await?;
    @%- endfor %@
    Ok(())
}
pub async fn check(shard_id: ShardId) -> Result<()> {
    repositories::@{ group|snake|ident }@::check(shard_id).await
}
pub async fn seed(value: &serde_yaml::Value, conns: &mut [DbConn]) -> Result<()> {
    repositories::@{ group|snake|ident }@::seed(value, conns).await
}
@{-"\n"}@