use anyhow::Result;
use senax_common::ShardId;
use std::path::Path;

#[allow(clippy::module_inception)]
pub mod repositories;
#[rustfmt::skip]
pub mod misc;

pub async fn start(db_dir: &Path) -> Result<()> {
    @%- for (name, (_, defs, _)) in groups %@
    repositories::@{ name|snake|ident }@::start(Some(db_dir)).await?;
    @%- endfor %@
    Ok(())
}
pub async fn start_test() -> Result<()> {
    @%- for (name, (_, defs, _)) in groups %@
    repositories::@{ name|snake|ident }@::start(None).await?;
    @%- endfor %@
    Ok(())
}
pub async fn check(shard_id: ShardId) -> Result<()> {
    @%- for (name, (_, defs, _)) in groups %@
    repositories::@{ name|snake|ident }@::check(shard_id).await?;
    @%- endfor %@
    Ok(())
}
@{-"\n"}@