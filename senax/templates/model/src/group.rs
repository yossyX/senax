// This code is auto-generated and will always be overwritten.

#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unreachable_patterns)]

use actix::ArbiterHandle;
use anyhow::Result;
use indexmap::IndexMap;
use schemars::JsonSchema;
use senax_common::{cache::msec::MSec, ShardId};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

use crate::{DbConn, DELAYED_DB_DIR};

#[rustfmt::skip]
#[allow(clippy::map_identity)]
#[allow(clippy::match_single_binding)]
#[allow(clippy::clone_on_copy)]
#[allow(clippy::nonminimal_bool)]
#[allow(clippy::useless_conversion)]
mod base {
@%- for name in mod_names %@
    pub mod _@{ name }@;
@%- endfor %@
}
@% for name in mod_names %@
pub mod @{ name|to_var_name }@;
@%- endfor %@

#[rustfmt::skip]
pub(crate) async fn start(handle: Option<&ArbiterHandle>, db_dir: Option<&Path>) -> Result<()> {
@%- for (name, defs) in tables  %@
    @{ defs.mod_name()|to_var_name }@::_@{ defs.mod_name() }@::init(handle).await?;
@%- endfor %@

    if let Some(handle) = handle {
        let path = db_dir.unwrap().join(DELAYED_DB_DIR).join("@{ group_name }@");
        handle.spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(2)).await;
                let db = sled::open(&path);
                match db {
                    Ok(db) => {
                        @%- for (name, defs) in tables  %@
                        @{ defs.mod_name()|to_var_name }@::_@{ defs.mod_name() }@::init_db(&db).await.unwrap();
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
    @%- for (name, defs) in tables  %@
    @{ defs.mod_name()|to_var_name }@::_@{ defs.mod_name() }@::check(shard_id).await?;
    @%- endfor %@
    Ok(())
}

#[rustfmt::skip]
#[allow(clippy::large_enum_variant)]
#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) enum CacheOp {
@%- for (name, defs) in tables  %@
    @{ name|to_pascal_name }@(@{ defs.mod_name()|to_var_name }@::_@{ defs.mod_name() }@::CacheOp),
@%- endfor %@
}

#[rustfmt::skip]
impl CacheOp {
    pub(crate) async fn handle_cache_msg(self, time: MSec, propagated: bool) {
        match self {
@%- for (name, defs) in tables  %@
            CacheOp::@{ name|to_pascal_name }@(msg) => msg.handle_cache_msg(time, propagated).await,
@%- endfor %@
        };
    }
}

#[rustfmt::skip]
pub(crate) fn clear_cache_all() {
@% for (name, defs) in tables  %@    @{ defs.mod_name()|to_var_name }@::_@{ defs.mod_name() }@::clear_cache_all();
@% endfor %@}

#[rustfmt::skip]
#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct @{ group_name|pascal }@ {
@%- for (name, defs) in tables  %@
    #[serde(default)]
    @{ name|to_var_name }@: IndexMap<String, @{ defs.mod_name()|to_var_name }@::_@{ name|to_pascal_name }@Factory>,
@%- endfor %@
}

#[rustfmt::skip]
#[allow(clippy::single_match)]
#[allow(clippy::match_single_binding)]
pub(crate) async fn seed(seed: &serde_yaml::Value, conns: &mut [DbConn]) -> Result<()> {
    if let Some(mapping) = seed.as_mapping() {
        for (name, value) in mapping {
            match name.as_str() {
@%- for (name, defs) in tables  %@
                Some("@{ name }@") => @{ defs.mod_name()|to_var_name }@::_@{ defs.mod_name() }@::_seed(value, conns).await?,
@%- endfor %@
                _ => {}
            }
        }
    }
    Ok(())
}
@{-"\n"}@