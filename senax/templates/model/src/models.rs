// This code is auto-generated and will always be overwritten.
// Senax v@{ ""|senax_version }@
@%- if !config.force_disable_cache %@
use crate::cache::Cache;
@%- endif %@
use crate::connection::{DbConn, DbType};
use ::anyhow::Result;
use ::futures::TryStreamExt;
use ::fxhash::FxHashMap;
use ::log::error;
use ::senax_common::ShardId;
use ::serde::{Deserialize, Serialize};
use ::std::path::Path;
use ::std::sync::Arc;
use ::tokio::sync::RwLock;

#[allow(dead_code)]
pub(crate) const USE_FAST_CACHE: bool = @{ config.use_fast_cache() }@;
#[allow(dead_code)]
pub(crate) const USE_STORAGE_CACHE: bool = @{ config.use_storage_cache }@;
pub(crate) static CACHE_UPDATE_LOCK: RwLock<()> = RwLock::const_new(());
@{-"\n"}@
@%- for (name, defs) in groups %@
pub mod @{ name|snake|to_var_name }@;
@%- endfor %@

#[derive(serde::Serialize, serde::Deserialize, Hash, PartialEq, Eq, Clone, Copy, Debug, strum::IntoStaticStr)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub enum NotifyOp {
    insert,
    update,
    upsert,
    delete,
    delete_all,
    invalidate,
    invalidate_all,
}

#[derive(serde::Serialize, serde::Deserialize, Hash, PartialEq, Eq, Clone, Copy, Debug, strum::EnumString, strum::IntoStaticStr)]
#[allow(non_camel_case_types)]
#[allow(dead_code)]
pub enum TableName {
    @%- for (_, defs) in groups %@
    @%- for (_, def) in defs %@
    @{ def.table_name() }@,
    @%- endfor %@
    @%- endfor %@
}

pub(crate) async fn start(db_dir: &Path) -> Result<()> {
@%- for (name, defs) in groups %@
    @{ name|snake|to_var_name }@::start(Some(db_dir)).await?;
@%- endfor %@
    Ok(())
}

pub(crate) async fn start_test() -> Result<()> {
@%- for (name, defs) in groups %@
    @{ name|snake|to_var_name }@::start(None).await?;
@%- endfor %@
    Ok(())
}

#[rustfmt::skip]
pub(crate) async fn check() -> Result<()> {
    for shard_id in DbConn::shard_num_range() {
        tokio::try_join!(
            @%- for (name, defs) in groups %@
            @{ name|snake|to_var_name }@::check(shard_id),
            @%- endfor %@
        )?;
    }
    Ok(())
}

pub(crate) struct CacheActor;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct CacheMsg(pub(crate) Vec<CacheOp>, pub(crate) FxHashMap<ShardId, u64>);

#[allow(clippy::large_enum_variant)]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) enum CacheOp {
@%- for (name, defs) in groups %@
    @{ name|to_pascal_name }@(@{ name|snake|to_var_name }@::CacheOp),
@%- endfor %@
    _AllClear,
}

impl CacheMsg {
    pub(crate) async fn handle_cache_msg(self) {
        let _lock = CACHE_UPDATE_LOCK.write().await;
        let _sync_map = Arc::new(self.1);
        #[cfg(not(feature = "cache_update_only"))]
        for op in self.0.into_iter() {
            match op {
@%- for (name, defs) in groups %@
                CacheOp::@{ name|to_pascal_name }@(op) => op.handle_cache_msg(Arc::clone(&_sync_map)).await,
@%- endfor %@
                CacheOp::_AllClear => _clear_cache(&_sync_map, false).await,
            };
        }
    }

    pub(crate) async fn do_send(self) {
        if !crate::is_test_mode() {
            CacheActor::handle(self);
        } else {
            self.handle_cache_msg().await;
        }
    }

    pub(crate) async fn do_send_to_internal(self) {
        self.handle_cache_msg().await;
    }
}

#[rustfmt::skip]
impl CacheActor {
    pub fn handle(msg: CacheMsg) {
        tokio::spawn(
            async move {
                let _guard = crate::get_shutdown_guard();
                if let Some(linker) = crate::LINKER_SENDER.get() {
                    if let Err(e) = linker.send(&msg) {
                        error!("{}", e);
                    }
                }
                msg.handle_cache_msg().await;
            }
        );
    }
}

pub(crate) async fn _clear_cache(_sync_map: &FxHashMap<ShardId, u64>, _clear_test: bool) {
@%- if !config.force_disable_cache %@
    #[cfg(not(feature = "cache_update_only"))]
    for (shard_id, sync) in _sync_map.iter() {
@%- for (name, defs) in groups %@
        @{ name|snake|to_var_name }@::clear_cache_all(*shard_id, *sync, _clear_test).await;
@%- endfor %@
    }
    Cache::invalidate_all();
@%- endif %@
}

pub(crate) async fn exec_ddl<'c, E>(sql: &str, conn: E) -> Result<()>
where
    E: sqlx::Executor<'c, Database = DbType>,
{
    let mut s = conn.execute_many(sql);
    while s.try_next().await?.is_some() {}
    Ok(())
}

pub(crate) async fn exec_migrate(shard_id: ShardId, ignore_missing: bool) -> Result<()> {
    let conn = DbConn::_new(shard_id);
    let mut source = conn.acquire_source().await?;
    @%- if config.collation.is_some() %@
    exec_ddl(
        r#"ALTER DATABASE COLLATE @{ config.collation.as_ref().unwrap() }@;"#,
        source.as_mut(),
    )
    .await?;
    @%- endif %@
    exec_ddl(
        r#"
            CREATE TABLE IF NOT EXISTS _sqlx_migrations (
                version BIGINT PRIMARY KEY,
                description TEXT NOT NULL,
                installed_on DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
                success BOOLEAN NOT NULL,
                checksum BLOB NOT NULL,
                execution_time BIGINT NOT NULL
            );
        "#,
        source.as_mut(),
    )
    .await?;
    @%- if config.use_sequence || !config.force_disable_cache %@
    exec_ddl(
        r#"
            CREATE TABLE IF NOT EXISTS "_sequence" (
                "id" INT UNSIGNED NOT NULL PRIMARY KEY,
                "seq" BIGINT UNSIGNED NOT NULL
            );
            INSERT IGNORE INTO "_sequence" VALUES (1, 0);
            INSERT IGNORE INTO "_sequence" VALUES (2, 0);
        "#,
        source.as_mut(),
    )
    .await?;
    @%- endif %@
    sqlx::migrate!()
        .set_ignore_missing(ignore_missing)
        .run(source.as_mut())
        .await?;
    Ok(())
}
@{-"\n"}@