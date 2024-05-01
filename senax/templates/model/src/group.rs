// This code is auto-generated and will always be overwritten.

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unreachable_patterns)]

use ::anyhow::Result;
use ::fxhash::FxHashMap;
use ::indexmap::IndexMap;
use ::schemars::JsonSchema;
use ::senax_common::{cache::msec::MSec, ShardId};
use ::serde::{Deserialize, Serialize};
use ::std::path::Path;
use ::std::sync::Arc;
use ::std::time::Duration;

use crate::{DbConn, DELAYED_DB_DIR};

#[rustfmt::skip]
#[allow(clippy::map_identity)]
#[allow(clippy::match_single_binding)]
#[allow(clippy::clone_on_copy)]
#[allow(clippy::nonminimal_bool)]
#[allow(clippy::useless_conversion)]
#[allow(clippy::enum_variant_names)]
#[allow(clippy::collapsible_if)]
pub mod _base {
@%- for name in mod_names %@
    pub mod _@{ name }@;
@%- endfor %@
}
@% for name in mod_names %@
pub mod @{ name|to_var_name }@;
@%- endfor %@

#[rustfmt::skip]
pub(crate) async fn start(db_dir: Option<&Path>) -> Result<()> {
@%- for (name, def) in models %@
    _base::_@{ def.mod_name() }@::init().await?;
@%- endfor %@

    if !crate::is_test_mode() {
        let path = db_dir.unwrap().join(DELAYED_DB_DIR).join("@{ group_name }@");
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let db = sled::open(&path);
                match db {
                    Ok(db) => {
                        @%- for (name, def) in models %@
                        _base::_@{ def.mod_name() }@::init_db(&db).await.unwrap();
                        @%- endfor %@
                        break;
                    }
                    Err(e) => ::log::error!("{}", e),
                }
            }
        });
    }

    Ok(())
}

pub(crate) async fn check(shard_id: ShardId) -> Result<()> {
    @%- for (name, def) in models %@
    @%- if !def.skip_ddl.unwrap_or_default() %@
    _base::_@{ def.mod_name() }@::check(shard_id).await?;
    @%- endif %@
    @%- endfor %@
    Ok(())
}

#[rustfmt::skip]
#[allow(clippy::large_enum_variant)]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) enum CacheOp {
@%- for (name, def) in models %@
    @{ name|to_pascal_name }@(_base::_@{ def.mod_name() }@::CacheOp),
@%- endfor %@
}

#[rustfmt::skip]
impl CacheOp {
    #[cfg(not(feature="cache_update_only"))]
    pub(crate) async fn handle_cache_msg(self, sync_map: Arc<FxHashMap<ShardId, u64>>) {
        match self {
@%- for (name, def) in models %@
            CacheOp::@{ name|to_pascal_name }@(msg) => msg.handle_cache_msg(sync_map).await,
@%- endfor %@
        };
    }
}
@%- if !config.force_disable_cache %@

#[cfg(not(feature="cache_update_only"))]
#[rustfmt::skip]
pub(crate) async fn clear_cache_all(shard_id: ShardId, sync: u64, clear_test: bool) {
@%- for (name, def) in models %@
    _base::_@{ def.mod_name() }@::clear_cache_all(shard_id, sync, clear_test).await;
@%- endfor %@
}
@%- endif %@

#[rustfmt::skip]
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct @{ group_name|pascal }@ {
@%- for (name, def) in models %@
    #[serde(default)]
    @{ name|to_var_name }@: IndexMap<String, @{ def.mod_name()|to_var_name }@::_@{ name|pascal }@Factory>,
@%- endfor %@
}

#[rustfmt::skip]
#[allow(clippy::single_match)]
#[allow(clippy::match_single_binding)]
pub(crate) async fn seed(seed: &serde_yaml::Value, conns: &mut [DbConn]) -> Result<()> {
    if let Some(mapping) = seed.as_mapping() {
        for (name, value) in mapping {
            match name.as_str() {
@%- for (name, def) in models %@
                Some("@{ name }@") => _base::_@{ def.mod_name() }@::_seed(value, conns).await?,
@%- endfor %@
                _ => {}
            }
        }
    }
    Ok(())
}
@{-"\n"}@