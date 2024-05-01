// This code is auto-generated and will always be overwritten.

use ahash::{AHashMap, AHasher};
use anyhow::{ensure, Context as _, Result};
use arc_swap::ArcSwapOption;
use async_trait::async_trait;
use core::option::Option;
use crossbeam::queue::SegQueue;
use derive_more::Display;
use futures::stream::StreamExt;
use futures::{future, future::BoxFuture, Future, FutureExt, Stream, TryStreamExt};
use fxhash::{FxHashMap, FxHashSet, FxHasher64};
use indexmap::IndexMap;
use log::{debug, error, info, warn};
use once_cell::sync::{Lazy, OnceCell};
use schemars::JsonSchema;
use senax_common::cache::db_cache::{CacheVal, HashVal};
use senax_common::cache::msec::MSec;
use senax_common::cache::{calc_mem_size, CycleCounter};
use senax_common::ShardId;
use senax_common::{err, types::blob::*, types::geo_point::*, types::point::*, SqlColumns};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_repr::{Deserialize_repr, Serialize_repr};
use sqlx::query::{Query, QueryAs};
use sqlx::Execute as _;
use std::borrow::Borrow;
use std::boxed::Box;
use std::collections::{BTreeMap, HashMap};
use std::convert::TryInto;
use std::fmt::Write;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use std::vec::Vec;
use std::{cmp, fmt};
use strum::{EnumMessage, EnumString, FromRepr, IntoStaticStr};
use tokio::sync::{mpsc, Mutex, RwLock, Semaphore};
use tokio::time::{sleep, Duration};
use tracing::debug_span;
use zstd::{decode_all, encode_all};
@%- if !config.force_disable_cache %@

use crate::cache::Cache;
@%- else %@
@% endif %@
use crate::connection::{DbArguments, DbConn, DbRow, DbType};
use crate::misc::{BindArrayTr, BindTr, ColRelTr, ColTr, FilterTr, IntoJson as _, OrderTr};
use crate::misc::{BindValue, Updater, Size, TrashMode};
use crate::models::USE_FAST_CACHE;
use crate::{
    accessor::*, CacheMsg, BULK_FETCH_SEMAPHORE, BULK_INSERT_MAX_SIZE, IN_CONDITION_LIMIT,
};
@%- if !config.excluded_from_domain %@
#[allow(unused_imports)]
use domain::value_objects;
pub use domain::models::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::@{ mod_name|to_var_name }@::{join, Joiner_};
@%- endif %@
@%- for mod_name in def.relation_mods() %@
use crate::models::@{ mod_name[0]|to_var_name }@::_base::_@{ mod_name[1] }@ as rel_@{ mod_name[0] }@_@{ mod_name[1] }@;
@%- if !config.excluded_from_domain %@
use domain::models::@{ db|snake|to_var_name }@::@{ mod_name[0]|to_var_name }@::@{ mod_name[1]|to_var_name }@ as join_@{ mod_name[0] }@_@{ mod_name[1] }@;
@%- else %@
use crate::models::@{ mod_name[0]|to_var_name }@::_base::_@{ mod_name[1] }@ as join_@{ mod_name[0] }@_@{ mod_name[1] }@;
@%- endif %@
@%- endfor %@
const USE_CACHE: bool = @{ def.use_cache() }@;
const USE_CACHE_ALL: bool = @{ def.use_all_row_cache() }@;
const USE_UPDATE_NOTICE: bool = @{ def.use_update_notice() }@;
pub@{ visibility }@ const TABLE_NAME: &str = "@{ table_name }@";
pub(crate) const TRASHED_SQL: &str = r#"@{ def.inheritance_cond(" AND ") }@"#;
pub(crate) const NOT_TRASHED_SQL: &str = r#"@{ def.soft_delete_tpl("","deleted_at IS NULL AND ","deleted = 0 AND ")}@@{ def.inheritance_cond(" AND ") }@"#;
pub(crate) const ONLY_TRASHED_SQL: &str = r#"@{ def.soft_delete_tpl("","deleted_at IS NOT NULL AND ","deleted != 0 AND ")}@@{ def.inheritance_cond(" AND ") }@"#;

@% if !config.force_disable_cache -%@
static CACHE_ALL: OnceCell<Vec<ArcSwapOption<Vec<_@{ pascal_name }@Cache>>>> = OnceCell::new();
@% else -%@
static CACHE_ALL: OnceCell<Vec<ArcSwapOption<Vec<()>>>> = OnceCell::new();
@% endif -%@
static CACHE_RESET_SYNC: OnceCell<Vec<RwLock<u64>>> = OnceCell::new();
static CACHE_RESET_SYNC_ALL: OnceCell<Vec<Mutex<u64>>> = OnceCell::new();
static BULK_FETCH_QUEUE: OnceCell<Vec<SegQueue<InnerPrimary>>> = OnceCell::new();
static PRIMARY_TYPE_ID: u64 = @{ def.get_type_id("PRIMARY_TYPE_ID") }@;
static COL_KEY_TYPE_ID: u64 = @{ def.get_type_id("COL_KEY_TYPE_ID") }@;
static VERSION_TYPE_ID: u64 = @{ def.get_type_id("VERSION_TYPE_ID") }@;
static CACHE_SYNC_TYPE_ID: u64 = @{ def.get_type_id("CACHE_SYNC_TYPE_ID") }@;
static CACHE_TYPE_ID: u64 = @{ def.get_type_id("CACHE_TYPE_ID") }@;
@%- if def.act_as_job_queue() %@
pub static QUEUE_NOTIFIER: tokio::sync::Notify = tokio::sync::Notify::const_new();
@%- endif %@

#[allow(clippy::needless_if)]
pub(crate) async fn init() -> Result<()> {
    if CACHE_ALL.get().is_none() {
        CACHE_ALL.set(DbConn::shard_num_range().map(|_| ArcSwapOption::const_empty()).collect()).unwrap();
        CACHE_RESET_SYNC.set(DbConn::shard_num_range().map(|_| RwLock::new(0)).collect()).unwrap();
        CACHE_RESET_SYNC_ALL.set(DbConn::shard_num_range().map(|_| Mutex::new(0)).collect()).unwrap();
        @%- if def.use_save_delayed() %@
        SAVE_DELAYED_QUEUE.set(DbConn::shard_num_range().map(|_| SegQueue::new()).collect()).unwrap();
        @%- endif %@
        @%- if def.use_update_delayed() %@
        UPDATE_DELAYED_QUEUE.set(DbConn::shard_num_range().map(|_| SegQueue::new()).collect()).unwrap();
        @%- endif %@
        @%- if def.use_upsert_delayed() %@
        UPSERT_DELAYED_QUEUE.set(DbConn::shard_num_range().map(|_| SegQueue::new()).collect()).unwrap();
        @%- endif %@
        BULK_FETCH_QUEUE.set(DbConn::shard_num_range().map(|_| SegQueue::new()).collect()).unwrap();
    }

    if !crate::is_test_mode() {
        @%- if def.use_insert_delayed() %@
        tokio::spawn(async {
            while !crate::is_stopped() {
                DelayedActor::handle(DelayedMsg::InsertFromDisk);
                sleep(Duration::from_secs(10)).await;
            }
        });
        @%- endif %@
    }
    Ok(())
}

pub(crate) async fn check(shard_id: ShardId) -> Result<()> {
    let mut conn = DbConn::_new(shard_id);
    _@{ pascal_name }@::query().limit(0).select(&mut conn).await?;
    Ok(())
}

pub(crate) async fn init_db(db: &sled::Db) -> Result<()> {
    @%- if def.use_insert_delayed() %@
    let tree = db.open_tree(TABLE_NAME)?;
    INSERT_DELAYED_DB.set(tree).unwrap();
    DelayedActor::handle(DelayedMsg::InsertFromMemory);
    DelayedActor::handle(DelayedMsg::InsertFromDisk);
    @%- endif %@
    Ok(())
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) enum CacheOp {
    None,
@%- if def.act_as_job_queue() %@
    Queued,
@%- endif %@
@%- if !config.force_disable_cache && !def.use_clear_whole_cache() && !def.act_as_job_queue() %@
    Insert {
        shard_id: ShardId,
        data: Data,
@{- def.relations_one_cache(false)|fmt_rel_join("
        #[serde(default, skip_serializing_if = \"Option::is_none\")]
        _{rel_name}: Option<Vec<rel_{class_mod}::CacheOp>>,", "") }@
@{- def.relations_many_cache(false)|fmt_rel_join("
        #[serde(default, skip_serializing_if = \"Option::is_none\")]
        _{rel_name}: Option<Vec<rel_{class_mod}::CacheOp>>,", "") }@
    },
    BulkInsert {
        shard_id: ShardId,
        list: Vec<ForInsert>,
    },
    Update {
        id: InnerPrimary,
        shard_id: ShardId,
        update: Data,
        op: OpData,
@{- def.relations_one_cache(false)|fmt_rel_join("
        #[serde(default, skip_serializing_if = \"Option::is_none\")]
        _{rel_name}: Option<Vec<rel_{class_mod}::CacheOp>>,", "") }@
@{- def.relations_many_cache(false)|fmt_rel_join("
        #[serde(default, skip_serializing_if = \"Option::is_none\")]
        _{rel_name}: Option<Vec<rel_{class_mod}::CacheOp>>,", "") }@
    },
    UpdateMany {
        ids: Vec<InnerPrimary>,
        shard_id: ShardId,
        update: Data,
        data_list: Vec<Data>,
        op: OpData,
    },
    BulkUpsert {
        shard_id: ShardId,
        data_list: Vec<Data>,
        update: Data,
        op: OpData,
    },
    Delete {
        id: InnerPrimary,
        shard_id: ShardId,
    },
    DeleteMany {
        ids: Vec<InnerPrimary>,
        shard_id: ShardId,
    },
    DeleteAll {
        shard_id: ShardId,
    },
    Cascade {
        ids: Vec<InnerPrimary>,
        shard_id: ShardId,
    },
    Invalidate {
        id: InnerPrimary,
        shard_id: ShardId,
    },
    InvalidateAll,
@%- for (mod_name, rel_name, local, val, val2, rel) in def.relations_on_delete_not_cascade() %@
    Reset@{ rel_name|pascal }@@{ val|pascal }@ {
        ids: Vec<InnerPrimary>,
        shard_id: ShardId,
    },
@%- endfor %@
@%- endif %@
}

#[cfg(not(feature="cache_update_only"))]
impl CacheOp {
    @%- if !config.force_disable_cache && !def.use_clear_whole_cache() && !def.act_as_job_queue() %@
    pub(crate) fn update(mut obj: CacheData, update: &Data, op: &OpData) -> CacheData {
        @{- def.cache_cols_without_primary()|fmt_join("
        Accessor{accessor_with_sep_type}::_set(op.{var}, &mut obj.{var}, &update.{var});", "") }@
        obj
    }

    pub(crate) fn apply_to_obj(obj: &Option<Arc<CacheWrapper>>, msgs: &[CacheOp], shard_id: ShardId, time: MSec) -> Option<Arc<CacheWrapper>> {
        let mut obj = obj.as_ref().cloned();
        for msg in msgs {
            match msg {
                CacheOp::None => {},
                CacheOp::Insert { data, .. } => {
                    obj = Some(Arc::new(CacheWrapper::_from_data(data.clone(), shard_id, time)));
                }
                CacheOp::BulkInsert { .. } => {},
                CacheOp::Update { update, op, .. } => {
                    if let Some(ref arc_wrapper) = obj {
                        let mut wrapper = arc_wrapper.as_ref().clone();
                        wrapper._inner = Self::update(wrapper._inner, update, op);
                        obj = Some(Arc::new(wrapper));
                    } else {
                        obj = None;
                    }
                }
                CacheOp::UpdateMany { .. } => {},
                CacheOp::BulkUpsert { .. } => {},
                CacheOp::Delete { .. } => {
                    obj = None;
                }
                CacheOp::DeleteMany { .. } => {},
                CacheOp::DeleteAll { .. } => {},
                CacheOp::Cascade { .. } => {},
                CacheOp::Invalidate { .. } => {},
                CacheOp::InvalidateAll => {},
                @%- for (mod_name, rel_name, local, val, val2, rel) in def.relations_on_delete_not_cascade() %@
                CacheOp::Reset@{ rel_name|pascal }@@{ val|pascal }@ { .. } => {},
                @%- endfor %@
            }
        }
        obj
    }

    pub(crate) fn apply_to_list(list: &[Arc<CacheWrapper>], msgs: &[CacheOp], shard_id: ShardId, time: MSec) -> Vec<Arc<CacheWrapper>> {
        let mut map = list.iter().map(|v| (InnerPrimary::from(&v._inner), Arc::clone(v))).collect::<IndexMap<_, _>>();
        for msg in msgs {
            match msg {
                CacheOp::None => {},
                CacheOp::Insert { data, .. } => {
                    map.insert(InnerPrimary::from(data), Arc::new(CacheWrapper::_from_data(data.clone(), shard_id, time)));
                }
                CacheOp::BulkInsert { .. } => {},
                CacheOp::Update { id, update, op, .. } => {
                    if let Some(obj) = map.get(id) {
                        let mut wrapper = obj.as_ref().clone();
                        wrapper._inner = Self::update(wrapper._inner, update, op);
                        map.insert(id.clone(), Arc::new(wrapper));
                    }
                }
                CacheOp::UpdateMany { ids, update, data_list, op, .. } => {
                    let mut data_map = FxHashMap::default();
                    for data in data_list.iter() {
                        data_map.insert(InnerPrimary::from(data), data);
                    }
                    for id in ids {
                        if let Some(obj) = map.get(id) {
                            let mut wrapper = obj.as_ref().clone();
                            wrapper._inner = if let Some(data) = data_map.get(id) { 
                                Self::update(wrapper._inner, data, op)
                            } else {
                                Self::update(wrapper._inner, update, op)
                            };
                            map.insert(id.clone(), Arc::new(wrapper));
                        }
                    }
                }
                CacheOp::BulkUpsert { .. } => {},
                CacheOp::Delete { id, .. } => {
                    map.remove(id);
                }
                CacheOp::DeleteMany { ids, .. } => {
                    for id in ids {
                        map.remove(id);
                    }
                }
                CacheOp::DeleteAll { .. } => {},
                CacheOp::Cascade { .. } => {},
                CacheOp::Invalidate { .. } => {},
                CacheOp::InvalidateAll => {},
                @%- for (mod_name, rel_name, local, val, val2, rel) in def.relations_on_delete_not_cascade() %@
                CacheOp::Reset@{ rel_name|pascal }@@{ val|pascal }@ { .. } => {},
                @%- endfor %@
            }
        }
        map.into_values().collect()
    }

    #[allow(clippy::let_and_return)]
    async fn update_with_unique_cache(id: &PrimaryHasher, obj: CacheData, update: &Data, op: &OpData, time: MSec) -> CacheData {
@%- for (index_name, index) in def.unique_index() %@
        if @{ index.fields(index_name, def)|fmt_index_col_not_null_or_null("op.{var} != Op::None", "op.{var} != Op::None && obj.{var}.is_some()", " && ") }@ {
            let key = VecColKey(vec![@{- index.fields(index_name, def)|fmt_index_col_not_null_or_null("ColKey_::{var}(obj.{var}.clone(){inner_to_raw}.into())", "ColKey_::{var}(obj.{var}.as_ref().unwrap().clone(){inner_to_raw}.into())", ", ") }@]);
            Cache::invalidate(&key, id._shard_id()).await;
        }
@%- endfor %@
        let obj = CacheOp::update(obj, update, op);
@%- for (index_name, index) in def.unique_index() %@
        if @{ index.fields(index_name, def)|fmt_index_col_not_null_or_null("op.{var} != Op::None", "op.{var} != Op::None && obj.{var}.is_some()", " && ") }@ {
            let key = VecColKey(vec![@{- index.fields(index_name, def)|fmt_index_col_not_null_or_null("ColKey_::{var}(obj.{var}.clone(){inner_to_raw}.into())", "ColKey_::{var}(obj.{var}.as_ref().unwrap().clone(){inner_to_raw}.into())", ", ") }@]);
            Cache::insert_short(&key, Arc::new(id.to_wrapper(time))).await;
        }
@%- endfor %@
        obj
    }
    @%- endif %@

    #[allow(clippy::redundant_clone)]
    pub(crate) fn handle_cache_msg(self, sync_map: Arc<FxHashMap<ShardId, u64>>) -> BoxFuture<'static, ()> {
        async move {
            let time = MSec::now();
            _@{pascal_name}@::_receive_update_notice(&self).await;
            match self {
                CacheOp::None => {},
                @%- if def.act_as_job_queue() %@
                CacheOp::Queued => {
                    QUEUE_NOTIFIER.notify_waiters();
                },
                @%- endif %@
                @%- if !config.force_disable_cache && !def.use_clear_whole_cache() && !def.act_as_job_queue() %@
                CacheOp::Insert { shard_id, data
                    @{- def.relations_one_cache(false)|fmt_rel_join(", _{rel_name}", "") -}@ 
                    @{- def.relations_many_cache(false)|fmt_rel_join(", _{rel_name}", "") }@ } => {
                    let sync = *sync_map.get(&shard_id).unwrap();
                    clear_cache_all(shard_id, sync, false).await;
                    let mut cache = CacheWrapper::_from_data(data.clone(), shard_id, time);
                    let id = InnerPrimary::from(&cache._inner);
                    if USE_UPDATE_NOTICE && DbConn::_has_update_notice() {
                        DbConn::_publish_update_notice(crate::models::TableName::@{ table_name }@, crate::models::NotifyOp::insert, &id).await;
                    }
                    let id = PrimaryHasher(id, shard_id);
                    @%- if def.versioned %@
                    let vw = VersionWrapper {
                        id: id.0.clone(),
                        shard_id,
                        time,
                        version: 0,
                    };
                    if Cache::get_version::<VersionWrapper>(&vw, shard_id).await.filter(|o| o.id == id.0).is_some() {
                        return;
                    }
                    @%- endif %@
                    if Cache::get_from_memory::<CacheWrapper>(&id, shard_id, USE_FAST_CACHE).await.filter(|o| InnerPrimary::from(o) == id.0).is_some() {
                        return;
                    }
    @{- def.relations_one_cache(false)|fmt_rel_join("
                    if let Some(_{rel_name}) = _{rel_name} {
                        cache.{rel_name} = rel_{class_mod}::CacheOp::apply_to_obj(&cache.{rel_name}, &_{rel_name}, shard_id, time);
                        for msg in _{rel_name} {
                            msg.handle_cache_msg(Arc::clone(&sync_map)).await;
                        }
                    }", "") }@
    @{- def.relations_many_cache(false)|fmt_rel_join("
                    if let Some(_{rel_name}) = _{rel_name} {
                        cache.{rel_name} = rel_{class_mod}::CacheOp::apply_to_list(&cache.{rel_name}, &_{rel_name}, shard_id, time);
                        {cache_list_sort}
                        {cache_list_limit}
                        for msg in _{rel_name} {
                            msg.handle_cache_msg(Arc::clone(&sync_map)).await;
                        }
                    }", "") }@
                    Cache::insert_short(&id, Arc::new(cache)).await;
                    @%- for (index_name, index) in def.unique_index() %@
                    if @{ index.fields(index_name, def)|fmt_index_col_not_null_or_null("true", " data.{var}.is_some()", " && ") }@ {
                        let key = VecColKey(vec![@{- index.fields(index_name, def)|fmt_index_col_not_null_or_null("ColKey_::{var}(data.{var}.clone(){inner_to_raw}.into())", "ColKey_::{var}(data.{var}.unwrap().clone(){inner_to_raw}.into())", ", ") }@]);
                        Cache::invalidate(&key, shard_id).await;
                        Cache::insert_short(&key, Arc::new(id.to_wrapper(time))).await;
                    }
                    @%- endfor %@
                }
                CacheOp::BulkInsert { shard_id, list } => {
                    let sync = *sync_map.get(&shard_id).unwrap();
                    clear_cache_all(shard_id, sync, false).await;
                    for row in list {
                        let mut cache = CacheWrapper::_from_data(row._data.clone(), shard_id, time);
                        let id = InnerPrimary::from(&cache._inner);
                        if USE_UPDATE_NOTICE && DbConn::_has_update_notice() {
                            DbConn::_publish_update_notice(crate::models::TableName::@{ table_name }@, crate::models::NotifyOp::insert, &id).await;
                        }
                        let id = PrimaryHasher(id, shard_id);
                        @%- if def.versioned %@
                        let vw = VersionWrapper {
                            id: id.0.clone(),
                            shard_id,
                            time,
                            version: cache._inner.@{ ConfigDef::version()|to_var_name }@,
                        };
                        if let Some(old) = Cache::get_version::<VersionWrapper>(&vw, shard_id).await.filter(|o| o.id == id.0) {
                            if old.version.less_than(vw.version) {
                                Cache::insert_version(&vw, Arc::new(vw.clone())).await;
                            } else {
                                continue;
                            }
                        } else if vw.version > 1 {
                            Cache::insert_version(&vw, Arc::new(vw.clone())).await;
                        }
                        @%- endif %@
                        let mut has_cache = false;
                        if let Some(_cache) = Cache::get_from_memory::<CacheWrapper>(&id, shard_id, USE_FAST_CACHE).await.filter(|o| InnerPrimary::from(o) == id.0) {
                            has_cache = true;
                            @%- if def.versioned %@
                            if _cache._inner.@{ ConfigDef::version()|to_var_name }@.greater_equal(cache._inner.@{ ConfigDef::version()|to_var_name }@) {
                                continue;
                            }
                            @%- endif %@
                            @{- def.relations_one_cache(false)|fmt_rel_join("
                            if row.{rel_name}.is_none() {
                                cache.{rel_name} = _cache.{rel_name}.clone();
                            }", "") }@
                            @{- def.relations_many_cache(false)|fmt_rel_join("
                            if row.{rel_name}.is_none() {
                                cache.{rel_name} = _cache.{rel_name}.clone();
                            }", "") }@
                        } else {
                            let cs = CacheSyncWrapper {
                                id: id.0.clone(),
                                shard_id,
                                time,
                                sync,
                            };
                            Cache::insert_version(&cs, Arc::new(cs.clone())).await;
                            Cache::invalidate(&id, shard_id).await;
                        }
                        @{- def.relations_one_cache(false)|fmt_rel_join("
                        if let Some(_{rel_name}) = row.{rel_name} {
                            cache.{rel_name} = _{rel_name}.as_ref(){soft_delete_filter}.map(|v| Arc::new(rel_{class_mod}::CacheWrapper::_from_data(v._data.clone(), shard_id, time)));
                            if let Some(_{rel_name}) = _{rel_name} {
                                rel_{class_mod}::CacheOp::BulkInsert {
                                    shard_id,
                                    list: vec![*_{rel_name}],
                                }.handle_cache_msg(Arc::clone(&sync_map)).await;
                            }
                        }", "")|replace1("_data") }@
                        @{- def.relations_many_cache(false)|fmt_rel_join("
                        if let Some(_{rel_name}) = row.{rel_name} {
                            cache.{rel_name} = _{rel_name}.iter(){soft_delete_filter}.map(|v| Arc::new(rel_{class_mod}::CacheWrapper::_from_data(v._data.clone(), shard_id, time))).collect();
                            {cache_list_sort}
                            {cache_list_limit}
                            rel_{class_mod}::CacheOp::BulkInsert {
                                shard_id,
                                list: _{rel_name},
                            }.handle_cache_msg(Arc::clone(&sync_map)).await;
                        }", "")|replace1("_data") }@
                        Cache::insert(&id, Arc::new(cache), USE_FAST_CACHE, has_cache).await;
                        @%- for (index_name, index) in def.unique_index() %@
                        if @{ index.fields(index_name, def)|fmt_index_col_not_null_or_null("true", " row._data.{var}.is_some()", " && ") }@ {
                            let key = VecColKey(vec![@{- index.fields(index_name, def)|fmt_index_col_not_null_or_null("ColKey_::{var}(row._data.{var}.clone(){inner_to_raw}.into())", "ColKey_::{var}(row._data.{var}.unwrap().clone(){inner_to_raw}.into())", ", ") }@]);
                            Cache::invalidate(&key, shard_id).await;
                            Cache::insert(&key, Arc::new(id.to_wrapper(time)), USE_FAST_CACHE, has_cache).await;
                        }
                        @%- endfor %@
                    }
                }
                CacheOp::Update { id, shard_id, update, op
                    @{- def.relations_one_cache(false)|fmt_rel_join(", _{rel_name}", "") -}@ 
                    @{- def.relations_many_cache(false)|fmt_rel_join(", _{rel_name}", "") }@ } => {
                    let sync = *sync_map.get(&shard_id).unwrap();
                    clear_cache_all(shard_id, sync, false).await;
                    if USE_UPDATE_NOTICE && DbConn::_has_update_notice() {
                        DbConn::_publish_update_notice(crate::models::TableName::@{ table_name }@, crate::models::NotifyOp::update, &id).await;
                    }
                    if USE_CACHE {
                        let id = PrimaryHasher(id.clone(), shard_id);
                        @%- if def.versioned %@
                        let vw = VersionWrapper {
                            id: id.0.clone(),
                            shard_id,
                            time,
                            version: update.@{ version_col }@,
                        };
                        if let Some(old) = Cache::get_version::<VersionWrapper>(&vw, shard_id).await.filter(|o| o.id == id.0) {
                            if old.version.less_than(vw.version) {
                                Cache::insert_version(&vw, Arc::new(vw.clone())).await;
                            } else {
                                return;
                            }
                        } else {
                            Cache::insert_version(&vw, Arc::new(vw.clone())).await;
                        }
                        @%- endif %@
                        if let Some(cache) = Cache::get_from_memory::<CacheWrapper>(&id, shard_id, USE_FAST_CACHE).await.filter(|o| InnerPrimary::from(o) == id.0) {
                            let mut cache = cache.as_ref().clone();
                            @%- if def.versioned %@
                            if op.@{ version_col }@ == Op::Set {
                                if cache._inner.@{ version_col }@.greater_equal(update.@{ version_col }@) {
                                    return;
                                } else if cache._inner.@{ version_col }@.cycle_add(1).less_than(update.@{ version_col }@) {
                                    Cache::invalidate(&id, shard_id).await;
                                    return;
                                }
                            }
                            @%- endif %@
                            cache._inner = CacheOp::update_with_unique_cache(&id, cache._inner, &update, &op, time).await;
                            @{- def.relations_one_cache(false)|fmt_rel_join("
                            if let Some(_{rel_name}) = _{rel_name} {
                                cache.{rel_name} = rel_{class_mod}::CacheOp::apply_to_obj(&cache.{rel_name}, &_{rel_name}, shard_id, time);
                                for msg in _{rel_name} {
                                    msg.handle_cache_msg(Arc::clone(&sync_map)).await;
                                }
                            }", "") }@
                            @{- def.relations_many_cache(false)|fmt_rel_join("
                            if let Some(_{rel_name}) = _{rel_name} {
                                cache.{rel_name} = rel_{class_mod}::CacheOp::apply_to_list(&cache.{rel_name}, &_{rel_name}, shard_id, time);
                                {cache_list_sort}
                                {cache_list_limit}
                                for msg in _{rel_name} {
                                    msg.handle_cache_msg(Arc::clone(&sync_map)).await;
                                }
                            }", "") }@
                            Cache::insert_long(&id, Arc::new(cache), USE_FAST_CACHE).await;
                        } else {
                            let cs = CacheSyncWrapper {
                                id: id.0.clone(),
                                shard_id,
                                time,
                                sync,
                            };
                            Cache::insert_version(&cs, Arc::new(cs.clone())).await;
                            Cache::invalidate(&id, shard_id).await;
                        }
                    }
                }
                CacheOp::UpdateMany { ids, shard_id, update, data_list, op } => {
                    let sync = *sync_map.get(&shard_id).unwrap();
                    clear_cache_all(shard_id, sync, false).await;
                    if USE_UPDATE_NOTICE && DbConn::_has_update_notice() {
                        for id in &ids {
                            DbConn::_publish_update_notice(crate::models::TableName::@{ table_name }@, crate::models::NotifyOp::update, id).await;
                        }
                    }
                    if USE_CACHE {
                        let mut data_map = FxHashMap::default();
                        for data in data_list.into_iter() {
                            data_map.insert(PrimaryHasher(InnerPrimary::from(&data), shard_id), data);
                        }
                        for id in &ids {
                            let id = PrimaryHasher(id.clone(), shard_id);
                            if let Some(cache) = Cache::get_from_memory::<CacheWrapper>(&id, shard_id, USE_FAST_CACHE).await.filter(|o| InnerPrimary::from(o) == id.0) {
                                let mut cache = cache.as_ref().clone();
                                cache._inner = if let Some(data) = data_map.get(&id) {
                                    CacheOp::update_with_unique_cache(&id, cache._inner, data, &op, time).await
                                } else {
                                    CacheOp::update_with_unique_cache(&id, cache._inner, &update, &op, time).await
                                };
                                Cache::insert_long(&id, Arc::new(cache), USE_FAST_CACHE).await;
                            } else {
                                let cs = CacheSyncWrapper {
                                    id: id.0.clone(),
                                    shard_id,
                                    time,
                                    sync,
                                };
                                Cache::insert_version(&cs, Arc::new(cs.clone())).await;
                                Cache::invalidate(&id, shard_id).await;
                            }
                        }
                    }
                }
                CacheOp::BulkUpsert { shard_id, data_list, update, op } => {
                    let sync = *sync_map.get(&shard_id).unwrap();
                    clear_cache_all(shard_id, sync, false).await;
                    if USE_UPDATE_NOTICE && DbConn::_has_update_notice() {
                        for data in &data_list {
                            let id = InnerPrimary::from(data);
                            DbConn::_publish_update_notice(crate::models::TableName::@{ table_name }@, crate::models::NotifyOp::upsert, &id).await;
                        }
                    }
                    if USE_CACHE {
                        for data in &data_list {
                            let id = PrimaryHasher(InnerPrimary::from(data), shard_id);
                            if let Some(cache) = Cache::get_from_memory::<CacheWrapper>(&id, shard_id, USE_FAST_CACHE).await.filter(|o| InnerPrimary::from(o) == id.0) {
                                let mut cache = cache.as_ref().clone();
                                cache._inner = CacheOp::update_with_unique_cache(&id, cache._inner, &update, &op, time).await;
                                Cache::insert_long(&id, Arc::new(cache), USE_FAST_CACHE).await;
                            } else {
                                let cs = CacheSyncWrapper {
                                    id: id.0.clone(),
                                    shard_id,
                                    time,
                                    sync,
                                };
                                Cache::insert_version(&cs, Arc::new(cs.clone())).await;
                                Cache::invalidate(&id, shard_id).await;
                            }
                        }
                    }
                }
                CacheOp::Delete { id, shard_id } => {
                    let sync = *sync_map.get(&shard_id).unwrap();
                    clear_cache_all(shard_id, sync, false).await;
                    if USE_UPDATE_NOTICE && DbConn::_has_update_notice() {
                        DbConn::_publish_update_notice(crate::models::TableName::@{ table_name }@, crate::models::NotifyOp::delete, &id).await;
                    }
                    if USE_CACHE {
                        let id = PrimaryHasher(id.clone(), shard_id);
                        @%- if def.versioned %@
                        let vw = VersionWrapper {
                            id: id.0.clone(),
                            shard_id,
                            time,
                            version: 0,
                        };
                        Cache::invalidate_version(&id, shard_id).await;
                        @%- endif %@
                        let cs = CacheSyncWrapper {
                            id: id.0.clone(),
                            shard_id,
                            time,
                            sync,
                        };
                        Cache::insert_version(&cs, Arc::new(cs.clone())).await;
                        Cache::invalidate(&id, shard_id).await;
                    }
                }
                CacheOp::DeleteMany { ids, shard_id } => {
                    let sync = *sync_map.get(&shard_id).unwrap();
                    clear_cache_all(shard_id, sync, false).await;
                    if USE_UPDATE_NOTICE && DbConn::_has_update_notice() {
                        for id in &ids {
                            DbConn::_publish_update_notice(crate::models::TableName::@{ table_name }@, crate::models::NotifyOp::delete, id).await;
                        }
                    }
                    if USE_CACHE {
                        for id in &ids {
                            let id = PrimaryHasher(id.clone(), shard_id);
                            @%- if def.versioned %@
                            let vw = VersionWrapper {
                                id: id.0.clone(),
                                shard_id,
                                time,
                                version: 0,
                            };
                            Cache::invalidate_version(&id, shard_id).await;
                            @%- endif %@
                            let cs = CacheSyncWrapper {
                                id: id.0.clone(),
                                shard_id,
                                time,
                                sync,
                            };
                            Cache::insert_version(&cs, Arc::new(cs.clone())).await;
                            Cache::invalidate(&id, shard_id).await;
                        }
                    }
                }
                CacheOp::DeleteAll { shard_id } => {
                    let sync = *sync_map.get(&shard_id).unwrap();
                    _clear_cache(shard_id, sync).await;
                    if USE_UPDATE_NOTICE && DbConn::_has_update_notice() {
                        DbConn::_publish_update_notice(crate::models::TableName::@{ table_name }@, crate::models::NotifyOp::delete_all, &serde_json::Value::Null).await;
                    }
                }
                CacheOp::Cascade { ids, shard_id } => {
                    let sync = *sync_map.get(&shard_id).unwrap();
                    clear_cache_all(shard_id, sync, false).await;
                    if USE_UPDATE_NOTICE && DbConn::_has_update_notice() {
                        for id in &ids {
                            DbConn::_publish_update_notice(crate::models::TableName::@{ table_name }@, crate::models::NotifyOp::delete, id).await;
                        }
                    }
                    if USE_CACHE {
                        for id in &ids {
                            let id = PrimaryHasher(id.clone(), shard_id);
                            @%- if def.versioned %@
                            let vw = VersionWrapper {
                                id: id.0.clone(),
                                shard_id,
                                time,
                                version: 0,
                            };
                            Cache::invalidate_version(&id, shard_id).await;
                            @%- endif %@
                            let cs = CacheSyncWrapper {
                                id: id.0.clone(),
                                shard_id,
                                time,
                                sync,
                            };
                            Cache::insert_version(&cs, Arc::new(cs.clone())).await;
                            Cache::invalidate(&id, shard_id).await;
                        }
                    }
                }
                CacheOp::Invalidate { id, shard_id  } => {
                    let sync = *sync_map.get(&shard_id).unwrap();
                    clear_cache_all(shard_id, sync, false).await;
                    if USE_UPDATE_NOTICE && DbConn::_has_update_notice() {
                        DbConn::_publish_update_notice(crate::models::TableName::@{ table_name }@, crate::models::NotifyOp::invalidate, &id).await;
                    }
                    if USE_CACHE {
                        let id = PrimaryHasher(id.clone(), shard_id);
                        @%- if def.versioned %@
                        let vw = VersionWrapper {
                            id: id.0.clone(),
                            shard_id,
                            time,
                            version: 0,
                        };
                        Cache::invalidate_version(&id, shard_id).await;
                        @%- endif %@
                        let cs = CacheSyncWrapper {
                            id: id.0.clone(),
                            shard_id,
                            time,
                            sync,
                        };
                        Cache::insert_version(&cs, Arc::new(cs.clone())).await;
                        Cache::invalidate(&id, shard_id).await;
                    }
                }
                CacheOp::InvalidateAll => {
                    for (shard_id, sync) in sync_map.iter() {
                        _clear_cache(*shard_id, *sync).await;
                    }
                    if USE_UPDATE_NOTICE && DbConn::_has_update_notice() {
                        DbConn::_publish_update_notice(crate::models::TableName::@{ table_name }@, crate::models::NotifyOp::invalidate_all, &serde_json::Value::Null).await;
                    }
                }
                @%- for (mod_name, rel_name, local, val, val2, rel) in def.relations_on_delete_not_cascade() %@
                CacheOp::Reset@{ rel_name|pascal }@@{ val|pascal }@ { ids, shard_id } => {
                    let sync = *sync_map.get(&shard_id).unwrap();
                    clear_cache_all(shard_id, sync, false).await;
                    if USE_UPDATE_NOTICE && DbConn::_has_update_notice() {
                        for id in &ids {
                            DbConn::_publish_update_notice(crate::models::TableName::@{ table_name }@, crate::models::NotifyOp::update, id).await;
                        }
                    }
                    if USE_CACHE {
                        for id in &ids {
                            let id = PrimaryHasher(id.clone(), shard_id);
                            let mut update = Data::default();
                            let mut op = OpData::default();
                            update.@{ local|to_var_name }@ = @{ val2 }@;
                            op.@{ local|to_var_name }@ = Op::Set;
                            if let Some(cache) = Cache::get::<CacheWrapper>(&id, shard_id, USE_FAST_CACHE).await.filter(|o| InnerPrimary::from(o) == id.0) {
                                let mut cache = cache.as_ref().clone();
                                cache._inner = CacheOp::update(cache._inner, &update, &op);
                                Cache::insert_long(&id, Arc::new(cache), USE_FAST_CACHE).await;
                            } else {
                                let cs = CacheSyncWrapper {
                                    id: id.0.clone(),
                                    shard_id,
                                    time,
                                    sync,
                                };
                                Cache::insert_version(&cs, Arc::new(cs.clone())).await;
                            }
                        }
                    }
                }
                @%- endfor %@
                @%- endif %@
            }
        }.boxed()
    }
}
impl CacheOp {
    pub fn wrap(self) -> crate::CacheOp {
        crate::CacheOp::@{ group_name|to_pascal_name }@(crate::models::@{ group_name|to_var_name }@::CacheOp::@{ model_name|to_pascal_name }@(self))
    }
}
@%- if !config.force_disable_cache %@

#[cfg(not(feature="cache_update_only"))]
pub(crate) async fn clear_cache_all(shard_id: ShardId, sync: u64, clear_test: bool) {
    @{- def.auto_inc_or_seq()|fmt_join("
    if clear_test {
        if let Some(ids) = GENERATED_IDS.get() { ids.write().unwrap().clear() }
    }", "") }@
    if USE_CACHE_ALL {
        if let Some(list) = CACHE_RESET_SYNC_ALL.get() {
            if clear_test {
                for shard in list {
                    let mut _sync = shard.lock().await;
                    *_sync = 0;
                }
            }
            let mut _sync = list[shard_id as usize].lock().await;
            if sync > *_sync {
                *_sync = sync;
                let _ = CACHE_ALL.get().unwrap()[shard_id as usize].swap(None);
            }
        }
    }
}
#[cfg(not(feature="cache_update_only"))]
async fn _clear_cache(shard_id: ShardId, sync: u64) {
    clear_cache_all(shard_id, sync, false).await;
    if USE_CACHE {
        let mut _sync = CACHE_RESET_SYNC.get().unwrap()[shard_id as usize].write().await;
        *_sync = sync;
        Cache::invalidate_all_of::<CacheWrapper>();
        Cache::invalidate_all_of::<PrimaryWrapper>();
        Cache::invalidate_all_of_version::<VersionWrapper>();
        Cache::invalidate_all_of_version::<CacheSyncWrapper>();
    }
}
@%- endif %@
@%- if def.use_insert_delayed() %@

static INSERT_DELAYED_QUEUE: Lazy<SegQueue<ForInsert>> = Lazy::new(SegQueue::new);
static INSERT_DELAYED_DB: OnceCell<sled::Tree> = OnceCell::new();
static INSERT_DELAYED_WAITING: AtomicBool = AtomicBool::new(false);
@%- endif %@
static DELAYED_DB_NO: Lazy<AtomicU64> = Lazy::new(|| {
    let now = SystemTime::now();
    let time = now.duration_since(UNIX_EPOCH).unwrap();
    AtomicU64::new(time.as_secs() << 20)
});
@%- if def.use_save_delayed() %@
static SAVE_DELAYED_QUEUE: OnceCell<Vec<SegQueue<_Updater_>>> = OnceCell::new();
static SAVE_DELAYED_WAITING: AtomicBool = AtomicBool::new(false);
static SAVE_DELAYED_SEMAPHORE: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(1));
@%- endif %@
@%- if def.use_update_delayed() %@
static UPDATE_DELAYED_QUEUE: OnceCell<Vec<SegQueue<_Updater_>>> = OnceCell::new();
static UPDATE_DELAYED_WAITING: AtomicBool = AtomicBool::new(false);
static UPDATE_DELAYED_SEMAPHORE: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(1));
@%- endif %@
@%- if def.use_upsert_delayed() %@
static UPSERT_DELAYED_QUEUE: OnceCell<Vec<SegQueue<_Updater_>>> = OnceCell::new();
static UPSERT_DELAYED_WAITING: AtomicBool = AtomicBool::new(false);
static UPSERT_DELAYED_SEMAPHORE: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(1));
@%- endif %@

struct DelayedActor;
@%- if def.use_insert_delayed() %@

struct InsertDelayedBuf(Vec<ForInsert>);
impl Drop for InsertDelayedBuf {
    fn drop(&mut self) {
        if !self.0.is_empty() {
            if let Err(err) = push_delayed_db(&self.0) {
                error!("push_delayed_db:{}", err);
            }
        }
    }
}
@%- endif %@

enum DelayedMsg {
    @%- if def.use_insert_delayed() %@
    InsertFromMemory,
    InsertFromDisk,
    @%- endif %@
    @%- if def.use_save_delayed() %@
    Save,
    @%- endif %@
    @%- if def.use_update_delayed() %@
    Update,
    @%- endif %@
    @%- if def.use_upsert_delayed() %@
    Upsert,
    @%- endif %@
}
impl DelayedActor {
    fn handle(msg: DelayedMsg) {
        match msg {
            @%- if def.use_insert_delayed() %@
            DelayedMsg::InsertFromMemory => {
                if INSERT_DELAYED_WAITING.load(Ordering::SeqCst) {
                    return;
                }
                tokio::spawn(
                    async move {
                        INSERT_DELAYED_WAITING.store(true, Ordering::SeqCst);
                        let _guard = crate::get_shutdown_guard();
                        sleep(Duration::from_millis(100)).await;
                        INSERT_DELAYED_WAITING.store(false, Ordering::SeqCst);
                        handle_delayed_msg_insert_from_memory().await;
                    }
                );
            }
            DelayedMsg::InsertFromDisk => {
                tokio::spawn(
                    async move {
                        let _guard = crate::get_shutdown_guard();
                        let mut handles = Vec::new();
                        for shard_id in DbConn::shard_num_range() {
                            handles.push(handle_delayed_msg_insert_from_disk(shard_id));
                        }
                        future::join_all(handles).await.iter().for_each(|r| {
                            if let Err(err) = r {
                                error!(table = TABLE_NAME; "INSERT DELAYED ERROR:{}", err);
                            }
                        });
                    }
                );
            }
            @%- endif %@
            @%- if def.use_save_delayed() %@
            DelayedMsg::Save => {
                if SAVE_DELAYED_WAITING.load(Ordering::SeqCst) {
                    return;
                }
                tokio::spawn(
                    async move {
                        SAVE_DELAYED_WAITING.store(true, Ordering::SeqCst);
                        let _guard = crate::get_shutdown_guard();
                        let _semaphore = SAVE_DELAYED_SEMAPHORE.acquire().await;
                        SAVE_DELAYED_WAITING.store(false, Ordering::SeqCst);
                        handle_delayed_msg_save().await;
                    }
                );
            }
            @%- endif %@
            @%- if def.use_update_delayed() %@
            DelayedMsg::Update => {
                if UPDATE_DELAYED_WAITING.load(Ordering::SeqCst) {
                    return;
                }
                tokio::spawn(
                    async move {
                        UPDATE_DELAYED_WAITING.store(true, Ordering::SeqCst);
                        let _guard = crate::get_shutdown_guard();
                        let _semaphore = UPDATE_DELAYED_SEMAPHORE.acquire().await;
                        UPDATE_DELAYED_WAITING.store(false, Ordering::SeqCst);
                        handle_delayed_msg_update().await;
                    }
                );
            }
            @%- endif %@
            @%- if def.use_upsert_delayed() %@
            DelayedMsg::Upsert => {
                if UPSERT_DELAYED_WAITING.load(Ordering::SeqCst) {
                    return;
                }
                tokio::spawn(
                    async move {
                        UPSERT_DELAYED_WAITING.store(true, Ordering::SeqCst);
                        let _guard = crate::get_shutdown_guard();
                        let _semaphore = UPSERT_DELAYED_SEMAPHORE.acquire().await;
                        UPSERT_DELAYED_WAITING.store(false, Ordering::SeqCst);
                        handle_delayed_msg_upsert().await;
                    }
                );
            }
            @%- endif %@
        }
    }
}
@%- if def.use_save_delayed() %@

async fn handle_delayed_msg_save() {
    let mut handles = Vec::new();
    for shard_id in DbConn::shard_num_range() {
        handles.push(_handle_delayed_msg_save(shard_id));
    }
    future::join_all(handles).await;
}
@%- endif %@
@%- if def.use_update_delayed() %@

async fn handle_delayed_msg_update() {
    let mut handles = Vec::new();
    for shard_id in DbConn::shard_num_range() {
        handles.push(_handle_delayed_msg_update(shard_id));
    }
    future::join_all(handles).await;
}
@%- endif %@
@%- if def.use_upsert_delayed() %@

async fn handle_delayed_msg_upsert() {
    let mut handles = Vec::new();
    for shard_id in DbConn::shard_num_range() {
        handles.push(_handle_delayed_msg_upsert(shard_id));
    }
    future::join_all(handles).await;
}
@%- endif %@
@%- if def.use_insert_delayed() %@

async fn handle_delayed_msg_insert_from_memory() {
    let mut vec = Vec::with_capacity(INSERT_DELAYED_QUEUE.len() + 10);
    while let Some(x) = INSERT_DELAYED_QUEUE.pop() {
        vec.push(x);
    }
    if vec.is_empty() {
        return;
    }
    let mut conn_list = Vec::new();
    for shard_id in DbConn::shard_num_range() {
        let mut conn = DbConn::_new(shard_id);
        match conn.begin_immediately().await {
            Ok(_) => {
                conn_list.push(conn);
            }
            Err(err) => {
                error!(table = TABLE_NAME; "INSERT DELAYED ERROR:{}", err);
            }
        }
    }
    if conn_list.is_empty() {
        if let Err(err) = push_delayed_db(&vec) {
            error!("push_delayed_db:{}", err);
        }
        return;
    }
    let len = vec.len() / conn_list.len() + 1;
    let mut handles = Vec::new();
    for conn in conn_list {
        if !vec.is_empty() {
            let vec2 = vec.split_off(vec.len().saturating_sub(len));
            handles.push(_handle_delayed_msg_insert_from_memory(conn, vec2));
        }
    }
    future::join_all(handles).await;
}

async fn _handle_delayed_msg_insert_from_memory(mut conn: DbConn, vec: Vec<ForInsert>) {
    let mut buf = InsertDelayedBuf(vec);
    let result = _@{ pascal_name }@::__bulk_insert(&mut conn, &buf.0, true, false).await;
    if let Err(err) = result {
        if let Some(err) = err.downcast_ref::<sqlx::Error>() {
            match err {
                sqlx::Error::Io(..) => {
                    // retry all
                    error!(table = TABLE_NAME; "INSERT DELAYED ERROR:{}", err);
                    drop(buf);
                    return;
                }
                sqlx::Error::WorkerCrashed => {
                    // retry all
                    error!(table = TABLE_NAME; "INSERT DELAYED ERROR:{}", err);
                    drop(buf);
                    return;
                }
                _ => {
                    let data = serde_json::to_string(&buf.0).unwrap();
                    error!(table = TABLE_NAME, data = data; "INSERT DELAYED FAILED:{}", err);
                }
            }
        } else {
            let data = serde_json::to_string(&buf.0).unwrap();
            error!(table = TABLE_NAME, data = data; "INSERT DELAYED FAILED:{}", err);
        }
    }
    let result = conn.commit().await;
    if crate::connection::is_retryable_error(result, TABLE_NAME) {
        drop(buf);
        return;
    }
    buf.0.clear();
}

async fn handle_delayed_msg_insert_from_disk(shard_id: ShardId) -> Result<()> {
    if crate::is_stopped() {
        return Ok(());
    }
    let db = match INSERT_DELAYED_DB.get() {
        Some(db) => db,
        None => {
            return Ok(());
        }
    };
    if db.is_empty() {
        return Ok(());
    }
    let mut conn = DbConn::_new(shard_id);
    if let Err(err) = conn.begin_immediately().await {
        error!(table = TABLE_NAME; "INSERT DELAYED ERROR:{}", err);
        return Ok(());
    }
    let mut vec = Vec::new();
    let mut total_size = 0;
    let max_size = *BULK_INSERT_MAX_SIZE.get().unwrap();
    while let Ok(x) = db.pop_min() {
        if let Some(x) = x {
            let list: Vec<ForInsert> = ciborium::from_reader(decode_all::<&[u8]>(x.1.borrow())?.as_slice())?;
            for data in list {
                total_size += data._data._size();
                vec.push(data);
            }
            if total_size >= max_size {
                break;
            }
        } else {
            break;
        }
    }
    if vec.is_empty() {
        return Ok(());
    }
    _handle_delayed_msg_insert_from_memory(conn, vec).await;
    if db.is_empty() {
        info!("Insert delayed successfully recovered.");
    } else if !crate::is_stopped() {
        DelayedActor::handle(DelayedMsg::InsertFromDisk);
    }
    tokio::spawn(async move {
        let _guard = crate::get_shutdown_guard();
        let _ = db.flush_async().await;
    });
    Ok(())
}

fn push_delayed_db(list: &Vec<ForInsert>) -> Result<()> {
    if let Some(db) = INSERT_DELAYED_DB.get() {
        let no = DELAYED_DB_NO.fetch_add(1, Ordering::SeqCst);
        let mut buf = Vec::new();
        ciborium::into_writer(list, &mut buf)?;
        let mut buf = encode_all(buf.as_slice(), 3)?;
        db.insert(no.to_be_bytes(), buf)?;
        tokio::spawn(async move {
            let _guard = crate::get_shutdown_guard();
            let _ = db.flush_async().await;
        });
    } else {
        for data in list {
            INSERT_DELAYED_QUEUE.push(data.clone());
        }
    }
    Ok(())
}
@%- endif %@
@%- if def.use_save_delayed() %@

async fn _handle_delayed_msg_save(shard_id: ShardId) {
    let mut map: BTreeMap<InnerPrimary, IndexMap<OpData, _Updater_>> = BTreeMap::new();
    while let Some(x) = SAVE_DELAYED_QUEUE.get().unwrap()[shard_id as usize].pop() {
        let inner_map = map.entry(InnerPrimary::from(&x)).or_insert_with(IndexMap::new);
        if let Some(old) = inner_map.get_mut(&x._op) {
            aggregate_update(&x, old);
        } else {
            inner_map.insert(x._op.clone(), x);
        }
    }
    if map.is_empty() {
        return;
    }
    let mut vec: Vec<IndexMap<OpData, _Updater_>> = map.into_values().collect();
    let chunk_num = cmp::max(10, vec.len() * 2 / (DbConn::max_connections_for_write() as usize) + 1);
    let mut join_set = tokio::task::JoinSet::new();
    while !vec.is_empty() {
        let mut buf = vec.split_off(vec.len().saturating_sub(chunk_num));
        join_set.spawn(async move {
            loop {
                let mut conn = DbConn::_new(shard_id);
                if let Err(err) = conn.begin_immediately().await {
                    error!(table = TABLE_NAME; "SAVE DELAYED ERROR:{}", err);
                    sleep(Duration::from_secs(10)).await;
                    continue;
                }
                for inner_map in buf.iter() {
                    for (_op, updater) in inner_map.iter() {
                        let result = _@{ pascal_name }@::__save(&mut conn, updater.clone()).await;
                        if crate::connection::is_retryable_error(result, TABLE_NAME) {
                            sleep(Duration::from_secs(1)).await;
                            continue;
                        }
                    }
                }
                let result = conn.commit().await;
                if crate::connection::is_retryable_error(result, TABLE_NAME) {
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
                break;
            }
        });
    }
    while join_set.join_next().await.is_some() {};
}
@%- endif %@
@%- if def.has_delayed_update() %@

fn aggregate_update(x: &_@{ pascal_name }@Updater, old: &mut _@{ pascal_name }@Updater) {
    @{- def.non_primaries()|fmt_join("
    Accessor{accessor_with_sep_type}::_set(x._op.{var}, &mut old._update.{var}, &x._update.{var});", "") }@
}
@%- endif %@
@%- if def.use_update_delayed() %@

async fn _handle_delayed_msg_update(shard_id: ShardId) {
    @{- def.soft_delete_tpl2("","
    let deleted_at = Some(SystemTime::now().into());","","
    let deleted = cmp::max(1, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as u32);")}@
    let mut map: BTreeMap<InnerPrimary, IndexMap<OpData, _Updater_>> = BTreeMap::new();
    while let Some(mut x) = UPDATE_DELAYED_QUEUE.get().unwrap()[shard_id as usize].pop() {
        @{- def.soft_delete_tpl2("","
        if x.will_be_deleted() {
            x.mut_deleted_at().set(deleted_at);
        }","
        if x.will_be_deleted() {
            x.mut_deleted().set(true);
        }","
        if x.will_be_deleted() {
            x.mut_deleted().set(deleted);
        }")}@
        let inner_map = map.entry(InnerPrimary::from(&x)).or_insert_with(IndexMap::new);
        if let Some(old) = inner_map.get_mut(&x._op) {
            aggregate_update(&x, old);
        } else {
            inner_map.insert(x._op.clone(), x);
        }
    }
    if map.is_empty() {
        return;
    }
    let mut vec: Vec<(OpData, Data, Vec<InnerPrimary>)> = Vec::new();
    for m in map.into_values() {
        'next: for up in m.into_values() {
            let id: InnerPrimary = (&up).into();
            for row in vec.iter_mut() {
                if row.0 == up._op && row.1 == up._update {
                    row.2.push(id);
                    continue 'next;
                }
            }
            vec.push((up._op, up._update, vec![id]));
        }
    }
    let mut join_set = tokio::task::JoinSet::new();
    for (op, update, list) in vec {
        join_set.spawn(async move {
            loop {
                let mut conn = DbConn::_new(shard_id);
                if let Err(err) = conn.begin_immediately().await {
                    error!(table = TABLE_NAME; "UPDATE DELAYED ERROR:{}", err);
                    sleep(Duration::from_secs(10)).await;
                    continue;
                }
                let mut updater = _Updater_ {
                    _data: Data::default(),
                    _update: update.clone(),
                    _is_new: false,
                    _do_delete: false,
                    _upsert: false,
                    _is_loaded: true,
                    _op: op.clone(),
@{- def.relations_one(false)|fmt_rel_join("
                    {rel_name}: None,", "") }@
@{- def.relations_many(false)|fmt_rel_join("
                    {rel_name}: None,", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("
                    {rel_name}: None,", "") }@
                };
                @%- if def.updated_at_conf().is_some() %@
                if updater._op.@{ ConfigDef::updated_at()|to_var_name }@ == Op::None {
                    updater.mut_@{ ConfigDef::updated_at() }@().set(@{(def.updated_at_conf().unwrap() == Timestampable::RealTime)|if_then_else_ref("SystemTime::now()","conn.time()")}@.into());
                }
                @%- endif %@
                let result = _@{ pascal_name }@::__update_many(&mut conn, list.clone(), updater).await;
                if crate::connection::is_retryable_error(result, TABLE_NAME) {
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
                let result = conn.commit().await;
                if crate::connection::is_retryable_error(result, TABLE_NAME) {
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
                break;
            }
        });
    }
    while join_set.join_next().await.is_some() {};
}
@%- endif %@
@%- if def.use_upsert_delayed() %@

async fn _handle_delayed_msg_upsert(shard_id: ShardId) {
    @{- def.soft_delete_tpl2("","
    let deleted_at = Some(SystemTime::now().into());","","
    let deleted = cmp::max(1, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as u32);")}@
    let mut map: BTreeMap<InnerPrimary, IndexMap<OpData, _Updater_>> = BTreeMap::new();
    while let Some(mut x) = UPSERT_DELAYED_QUEUE.get().unwrap()[shard_id as usize].pop() {
        @{- def.soft_delete_tpl2("","
        if x.will_be_deleted() {
            x.mut_deleted_at().set(deleted_at);
        }","
        if x.will_be_deleted() {
            x.mut_deleted().set(true);
        }","
        if x.will_be_deleted() {
            x.mut_deleted().set(deleted);
        }")}@
        let inner_map = map.entry(InnerPrimary::from(&x)).or_insert_with(IndexMap::new);
        if let Some(old) = inner_map.get_mut(&x._op) {
            aggregate_update(&x, old);
        } else {
            inner_map.insert(x._op.clone(), x);
        }
    }
    if map.is_empty() {
        return;
    }
    let mut vec: Vec<(OpData, Data, Vec<Data>)> = Vec::new();
    for m in map.into_values() {
        'next: for up in m.into_values() {
            for row in vec.iter_mut() {
                if row.0 == up._op && row.1 == up._update {
                    row.2.push(up._data);
                    continue 'next;
                }
            }
            vec.push((up._op, up._update, vec![up._data]));
        }
    }
    let mut join_set = tokio::task::JoinSet::new();
    for (op, update, list) in vec {
        join_set.spawn(async move {
            loop {
                let mut conn = DbConn::_new(shard_id);
                if let Err(err) = conn.begin_immediately().await {
                    error!(table = TABLE_NAME; "UPSERT DELAYED ERROR:{}", err);
                    sleep(Duration::from_secs(10)).await;
                    continue;
                }
                let mut updater = _Updater_ {
                    _data: list[0].clone(),
                    _update: update.clone(),
                    _is_new: false,
                    _do_delete: false,
                    _upsert: false,
                    _is_loaded: true,
                    _op: op.clone(),
@{- def.relations_one(false)|fmt_rel_join("
                    {rel_name}: None,", "") }@
@{- def.relations_many(false)|fmt_rel_join("
                    {rel_name}: None,", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("
                    {rel_name}: None,", "") }@
                };
                @%- if def.updated_at_conf().is_some() %@
                if updater._op.@{ ConfigDef::updated_at()|to_var_name }@ == Op::None {
                    updater.mut_@{ ConfigDef::updated_at() }@().set(@{(def.updated_at_conf().unwrap() == Timestampable::RealTime)|if_then_else_ref("SystemTime::now()","conn.time()")}@.into());
                }
                @%- endif %@
                let result = _@{ pascal_name }@::__bulk_upsert(&mut conn, &list, &updater).await;
                if crate::connection::is_retryable_error(result, TABLE_NAME) {
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
                let result = conn.commit().await;
                if crate::connection::is_retryable_error(result, TABLE_NAME) {
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
                break;
            }
        });
    }
    while join_set.join_next().await.is_some() {};
}
@%- endif %@

@% for (name, column_def) in def.id_except_auto_increment() -%@
#[derive(Deserialize, Serialize, Hash, PartialEq, Eq, PartialOrd, Ord, Clone,@% if column_def.is_copyable() %@ Copy,@% endif %@ Display, Debug, Default, JsonSchema)]
#[serde(transparent)]
@%- if !column_def.is_displayable() %@
#[display(fmt = "{:?}", _0)]
@%- endif %@
pub@{ visibility }@ struct @{ id_name }@(pub(crate) @{ column_def.get_inner_type(false, false) }@);
@% endfor -%@
@% for (name, column_def) in def.id_auto_inc_or_seq() -%@
#[derive(Serialize, Hash, PartialEq, Eq, PartialOrd, Ord, Clone,@% if column_def.is_copyable() %@ Copy,@% endif %@ Display, Debug, Default, JsonSchema)]
#[serde(transparent)]
@%- if !column_def.is_displayable() %@
#[display(fmt = "{:?}", _0)]
@%- endif %@
pub@{ visibility }@ struct @{ id_name }@(
    #[schemars(schema_with = "crate::seeder::id_schema")]
    pub(crate) @{ column_def.get_inner_type(true, false) }@
);

static GENERATED_IDS: OnceCell<std::sync::RwLock<HashMap<String, @{ id_name }@>>> = OnceCell::new();

#[allow(clippy::unnecessary_cast)]
impl<'de> serde::Deserialize<'de> for @{ id_name }@ {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Visitor;

        struct IdVisitor;

        impl<'de> Visitor<'de> for IdVisitor {
            type Value = @{ id_name }@;

            #[inline]
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an integer")
            }

            #[inline]
            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let ids = GENERATED_IDS.get().map(|ids| ids.read().unwrap());
                if let Some(id) = ids.and_then(|ids| ids.get(v).copied()) {
                    return Ok(id);
                }
                Err(serde::de::Error::invalid_value(serde::de::Unexpected::Str(v), &self))
            }

            #[inline]
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(@{ id_name }@(v as @{ column_def.get_inner_type(true, false) }@))
            }
        }
        deserializer.deserialize_u64(IdVisitor)
    }
}

@% endfor -%@
#[derive(Clone, Hash, PartialEq, Eq)]
pub@{ visibility }@ struct Primary(@{ def.primaries()|fmt_join("pub(crate) {outer_owned}", ", ") }@);
impl Primary {
    pub(crate) fn cols() -> &'static str {
        r#"@{ def.primaries()|fmt_join("{col_esc}", ", ") }@"#
    }
    pub(crate) fn cols_with_paren() -> &'static str {
        r#"@{ def.primaries()|fmt_join_with_paren("{col_esc}", ", ") }@"#
    }
    pub(crate) fn cols_with_idx(idx: usize) -> String {
        format!(r#"@{ def.primaries()|fmt_join_with_paren("_t{}.{col_esc}", ", ") }@"#, @{ def.primaries()|fmt_join("idx", ", ") }@)
    }
}
#[derive(
    Hash, PartialEq, Eq, Deserialize, Serialize, Clone, Debug, PartialOrd, Ord,
)]
pub(crate) struct InnerPrimary(@{ def.primaries()|fmt_join("pub(crate) {inner}", ", ") }@);
impl sqlx::FromRow<'_, DbRow> for InnerPrimary {
    fn from_row(row: &DbRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(InnerPrimary (
            @{- def.primaries()|fmt_join("{from_row}", ", ") }@
        ))
    }
}

#[derive(Hash, PartialEq, Eq, Deserialize, Serialize, Clone, Debug)]
struct PrimaryHasher(InnerPrimary, ShardId);
@%- if !config.force_disable_cache %@

impl HashVal for PrimaryHasher {
    fn hash_val(&self, shard_id: ShardId) -> u128 {
        let mut hasher = FxHasher64::default();
        PRIMARY_TYPE_ID.hash(&mut hasher);
        shard_id.hash(&mut hasher);
        self.0.hash(&mut hasher);
        let hash = (hasher.finish() as u128) << 64;

        let mut hasher = AHasher::default();
        PRIMARY_TYPE_ID.hash(&mut hasher);
        shard_id.hash(&mut hasher);
        self.0.hash(&mut hasher);
        hash | (hasher.finish() as u128)
    }
}
@%- endif %@

impl PrimaryHasher {
    fn _shard_id(&self) -> ShardId {
        self.1
    }
    fn to_wrapper(&self, time: MSec) -> PrimaryWrapper {
        PrimaryWrapper(self.0.clone(), self.1, time)
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
struct PrimaryWrapper(InnerPrimary, ShardId, MSec);
@%- if !config.force_disable_cache %@

impl CacheVal for PrimaryWrapper {
    fn _size(&self) -> u32 {
        let size = calc_mem_size(std::mem::size_of::<Self>());
        size.try_into().unwrap_or(u32::MAX)
    }
    fn _type_id(&self) -> u64 {
        Self::__type_id()
    }
    fn __type_id() -> u64 {
        PRIMARY_TYPE_ID
    }
    fn _shard_id(&self) -> ShardId {
        self.1
    }
    fn _time(&self) -> MSec {
        self.2
    }
    fn _estimate() -> usize {
        calc_mem_size(std::mem::size_of::<Self>())
    }
    fn _encode(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        ciborium::into_writer(self, &mut buf)?;
        Ok(buf)
    }
    fn _decode(v: &[u8]) -> Result<Self> {
        Ok(ciborium::from_reader::<Self, _>(v)?)
    }
}
@%- endif %@

#[derive(Deserialize, Serialize, PartialEq, Clone, Debug, senax_macros::SqlCol)]
pub(crate) struct Data {
@{ def.all_fields()|fmt_join("{serde}{column_query}    pub(crate) {var}: {inner},\n", "") -}@
}
impl Data {
    fn validate(&self) -> Result<(), validator::ValidationErrors> {
        use std::borrow::Cow;
        let mut errors = validator::ValidationErrors::new();
        @{- def.all_fields()|fmt_join("{validate}", "") }@
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
impl sqlx::FromRow<'_, DbRow> for Data {
    fn from_row(row: &DbRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(Data {
            @{- def.all_fields()|fmt_join("{var}: {from_row},", "") }@
        })
    }
}

#[allow(clippy::derivable_impls)]
impl Default for Data {
    fn default() -> Self {
        Self {
@{- def.all_fields()|fmt_join("
            {var}: {default},", "") }@
        }
    }
}

impl fmt::Display for Data {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        @{- def.all_except_secret()|fmt_join("
        Accessor{accessor_with_sep_type}::_write_insert(f, \"{comma}\", \"{raw_var}\", &self.{var})?;", "") }@
        write!(f, "}}")?;
        Ok(())
    }
}

impl Data {
    #[allow(clippy::let_and_return)]
    fn _size(&self) -> usize {
        let mut size = std::mem::size_of::<Self>();
        @{- def.cache_cols_not_null_sized()|fmt_join("
        size += self.{var}._size();", "") }@
        @{- def.cache_cols_null_sized()|fmt_join("
        size += self.{var}.as_ref().map(|v| v._size()).unwrap_or(0);", "") }@
        size
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct OpData {
@{- def.all_fields()|fmt_join("
    #[serde(default, skip_serializing_if = \"Op::is_none\")]
    pub(crate) {var}: Op,", "") }@
}
@%- if !config.force_disable_cache %@

#[derive(Serialize, Deserialize, Clone, Debug, Default, senax_macros::SqlCol)]
pub(crate) struct CacheData {
@{ def.cache_cols()|fmt_join("{serde}{column_query}    pub(crate) {var}: {inner},\n", "") -}@
}
impl sqlx::FromRow<'_, DbRow> for CacheData {
    fn from_row(row: &DbRow) -> sqlx::Result<Self> {
        use sqlx::Row;
        Ok(CacheData {
            @{- def.cache_cols()|fmt_join("{var}: {from_row},", "") }@
        })
    }
}
@%- endif %@

@% for (name, column_def) in def.num_enums(false) -%@
@% let values = column_def.enum_values.as_ref().unwrap() -%@
#[derive(Serialize_repr, Deserialize_repr, sqlx::Type, Hash, PartialEq, Eq, Clone, Copy, Debug, Default, strum::Display, FromRepr, EnumMessage, EnumString, IntoStaticStr, JsonSchema)]
#[repr(@{ column_def.get_inner_type(true, true) }@)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
pub@{ visibility }@ enum _@{ name|pascal }@ {
@% for row in values -%@@{ row.label|label4 }@@{ row.comment|comment4 }@@{ row.label|strum_message4 }@@{ row.comment|strum_detailed4 }@    @% if loop.first %@#[default]@% endif %@@{ row.name|to_var_name }@@{ row.value_str() }@,
@% endfor -%@
}
impl _@{ name|pascal }@ {
    pub@{ visibility }@ fn inner(&self) -> @{ column_def.get_inner_type(true, true) }@ {
        *self as @{ column_def.get_inner_type(true, true) }@
    }
@%- for row in values %@
    pub fn is_@{ row.name }@(&self) -> bool {
        self == &Self::@{ row.name|to_var_name }@
    }
@%- endfor %@
}
impl From<@{ column_def.get_inner_type(true, true) }@> for _@{ name|pascal }@ {
    fn from(val: @{ column_def.get_inner_type(true, true) }@) -> Self {
        if let Some(val) = Self::from_repr(val) {
            val
        } else {
            panic!("{} is a value outside the range of _@{ name|pascal }@.", val)
        }
    }
}
impl From<_@{ name|pascal }@> for @{ column_def.get_inner_type(true, true) }@ {
    fn from(val: _@{ name|pascal }@) -> Self {
        val.inner()
    }
}
impl From<_@{ name|pascal }@> for BindValue {
    fn from(val: _@{ name|pascal }@) -> Self {
        Self::Enum(Some(val.inner() as i64))
    }
}
impl From<Option<_@{ name|pascal }@>> for BindValue {
    fn from(val: Option<_@{ name|pascal }@>) -> Self {
        Self::Enum(val.map(|t| t.inner() as i64))
    }
}
@%- if !config.excluded_from_domain %@
@%- let a = crate::schema::set_domain_mode(true) %@
impl From<@{ column_def.get_filter_type(true) }@> for _@{ name|pascal }@ {
    fn from(v: @{ column_def.get_filter_type(true) }@) -> Self {
        match v {
@%- for row in values %@
            @{ column_def.get_filter_type(true) }@::@{ row.name }@ => _@{ name|pascal }@::@{ row.name }@,
@%- endfor %@
        }
    }
}
impl From<_@{ name|pascal }@> for @{ column_def.get_filter_type(true) }@ {
    fn from(v: _@{ name|pascal }@) -> Self {
        match v {
@%- for row in values %@
            _@{ name|pascal }@::@{ row.name }@ => @{ column_def.get_filter_type(true) }@::@{ row.name }@,
@%- endfor %@
        }
    }
}
@%- let a = crate::schema::set_domain_mode(false) %@
@%- endif %@

@% endfor -%@
@% for (name, column_def) in def.str_enums(false) -%@
@% let values = column_def.enum_values.as_ref().unwrap() -%@
#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Clone, Copy, Debug, Default, strum::Display, EnumMessage, EnumString, IntoStaticStr, JsonSchema)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
pub@{ visibility }@ enum _@{ name|pascal }@ {
@% for row in values -%@@{ row.label|label4 }@@{ row.comment|comment4 }@@{ row.label|strum_message4 }@@{ row.comment|strum_detailed4 }@    @% if loop.first %@#[default]@% endif %@@{ row.name|to_var_name }@,
@% endfor -%@
}
impl _@{ name|pascal }@ {
    pub fn as_static_str(&self) -> &'static str {
        Into::<&'static str>::into(self)
    }
@%- for row in values %@
    pub fn is_@{ row.name }@(&self) -> bool {
        self == &Self::@{ row.name|to_var_name }@
    }
@%- endfor %@
}
@%- if !config.excluded_from_domain %@
@%- let a = crate::schema::set_domain_mode(true) %@
impl From<@{ column_def.get_filter_type(true) }@> for _@{ name|pascal }@ {
    fn from(v: @{ column_def.get_filter_type(true) }@) -> Self {
        match v {
@%- for row in values %@
            @{ column_def.get_filter_type(true) }@::@{ row.name }@ => _@{ name|pascal }@::@{ row.name }@,
@%- endfor %@
        }
    }
}
impl From<_@{ name|pascal }@> for @{ column_def.get_filter_type(true) }@ {
    fn from(v: _@{ name|pascal }@) -> Self {
        match v {
@%- for row in values %@
            _@{ name|pascal }@::@{ row.name }@ => @{ column_def.get_filter_type(true) }@::@{ row.name }@,
@%- endfor %@
        }
    }
}
@%- let a = crate::schema::set_domain_mode(false) %@
@%- endif %@

@% endfor -%@
@{ def.label|label0 -}@
@{ def.comment|comment0 -}@
#[derive(Clone, Debug)]
pub@{ visibility }@ struct _@{ pascal_name }@ {
    pub(crate) _inner: Data,
@{ def.relations_one(false)|fmt_rel_join("    pub(crate) {rel_name}: Option<Option<Box<rel_{class_mod}::{class}>>>,\n", "") -}@
@{ def.relations_many(false)|fmt_rel_join("    pub(crate) {rel_name}: Option<Vec<rel_{class_mod}::{class}>>,\n", "") -}@
@{ def.relations_belonging(false)|fmt_rel_join("    pub(crate) {rel_name}: Option<Option<Box<rel_{class_mod}::{class}>>>,\n", "") -}@
}

#[derive(Serialize, Deserialize, Hash, PartialEq, Eq, Clone, Copy, Debug, strum::Display, EnumMessage, EnumString, IntoStaticStr, strum_macros::EnumIter, strum_macros::EnumProperty)]
#[allow(non_camel_case_types)]
pub@{ visibility }@ enum _@{ pascal_name }@Info {
@%- for (col_name, column_def) in def.all_fields() %@
@{ column_def.label|strum_message4 }@@{ column_def.comment|strum_detailed4 }@@{ column_def|strum_props4 }@    @{ col_name|to_var_name }@,
@%- endfor %@
}
@%- if !config.force_disable_cache %@

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct CacheWrapper {
    pub(crate) _inner: CacheData,
    _shard_id: ShardId,
    _time: MSec,
@{ def.relations_one_cache(false)|fmt_rel_join("    pub(crate) {rel_name}: Option<Arc<rel_{class_mod}::CacheWrapper>>,\n", "") -}@
@{ def.relations_many_cache(false)|fmt_rel_join("    pub(crate) {rel_name}: Vec<Arc<rel_{class_mod}::CacheWrapper>>,\n", "") -}@
}

#[derive(Clone, Debug)]
pub@{ visibility }@ struct _@{ pascal_name }@Cache {
    pub(crate) _wrapper: Arc<CacheWrapper>,
@{ def.relations_one_cache(false)|fmt_rel_join("    pub(crate) {rel_name}: Option<Option<Box<rel_{class_mod}::{class}Cache>>>,\n", "") -}@
@{ def.relations_one_uncached(false)|fmt_rel_join("    pub(crate) {rel_name}: Option<Option<Box<rel_{class_mod}::{class}>>>,\n", "") -}@
@{ def.relations_many_cache(false)|fmt_rel_join("    pub(crate) {rel_name}: Option<Vec<rel_{class_mod}::{class}Cache>>,\n", "") -}@
@{ def.relations_many_uncached(false)|fmt_rel_join("    pub(crate) {rel_name}: Option<Vec<rel_{class_mod}::{class}>>,\n", "") -}@
@{ def.relations_belonging_cache(false)|fmt_rel_join("    pub(crate) {rel_name}: Option<Option<Box<rel_{class_mod}::{class}Cache>>>,\n", "") -}@
@{ def.relations_belonging_uncached(false)|fmt_rel_join("    pub(crate) {rel_name}: Option<Option<Box<rel_{class_mod}::{class}>>>,\n", "") -}@
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct VersionWrapper {
    id: InnerPrimary,
    shard_id: ShardId,
    time: MSec,
    version: u32,
}

impl CacheVal for VersionWrapper {
    fn _size(&self) -> u32 {
        let size = calc_mem_size(std::mem::size_of::<Self>());
        size.try_into().unwrap()
    }
    fn _type_id(&self) -> u64 {
        Self::__type_id()
    }
    fn __type_id() -> u64 {
        VERSION_TYPE_ID
    }
    fn _shard_id(&self) -> ShardId {
        self.shard_id
    }
    fn _time(&self) -> MSec {
        self.time
    }
    fn _estimate() -> usize {
        calc_mem_size(std::mem::size_of::<Self>())
    }
    fn _encode(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        ciborium::into_writer(self, &mut buf)?;
        Ok(buf)
    }
    fn _decode(v: &[u8]) -> Result<Self> {
        Ok(ciborium::from_reader::<Self, _>(v)?)
    }
}
impl HashVal for VersionWrapper {
    fn hash_val(&self, shard_id: ShardId) -> u128 {
        let mut hasher = FxHasher64::default();
        VERSION_TYPE_ID.hash(&mut hasher);
        shard_id.hash(&mut hasher);
        self.id.hash(&mut hasher);
        let hash = (hasher.finish() as u128) << 64;

        let mut hasher = AHasher::default();
        VERSION_TYPE_ID.hash(&mut hasher);
        shard_id.hash(&mut hasher);
        self.id.hash(&mut hasher);
        hash | (hasher.finish() as u128)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct CacheSyncWrapper {
    id: InnerPrimary,
    shard_id: ShardId,
    time: MSec,
    sync: u64,
}

impl CacheVal for CacheSyncWrapper {
    fn _size(&self) -> u32 {
        let size = calc_mem_size(std::mem::size_of::<Self>());
        size.try_into().unwrap()
    }
    fn _type_id(&self) -> u64 {
        Self::__type_id()
    }
    fn __type_id() -> u64 {
        CACHE_SYNC_TYPE_ID
    }
    fn _shard_id(&self) -> ShardId {
        self.shard_id
    }
    fn _time(&self) -> MSec {
        self.time
    }
    fn _estimate() -> usize {
        calc_mem_size(std::mem::size_of::<Self>())
    }
    fn _encode(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        ciborium::into_writer(self, &mut buf)?;
        Ok(buf)
    }
    fn _decode(v: &[u8]) -> Result<Self> {
        Ok(ciborium::from_reader::<Self, _>(v)?)
    }
}
impl HashVal for CacheSyncWrapper {
    fn hash_val(&self, shard_id: ShardId) -> u128 {
        let mut hasher = FxHasher64::default();
        CACHE_SYNC_TYPE_ID.hash(&mut hasher);
        shard_id.hash(&mut hasher);
        self.id.hash(&mut hasher);
        let hash = (hasher.finish() as u128) << 64;

        let mut hasher = AHasher::default();
        CACHE_SYNC_TYPE_ID.hash(&mut hasher);
        shard_id.hash(&mut hasher);
        self.id.hash(&mut hasher);
        hash | (hasher.finish() as u128)
    }
}
@%- endif %@

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub@{ visibility }@ struct _@{ pascal_name }@Factory {
@{ def.for_factory()|fmt_join("{label}{comment}{factory_default}    pub {var}: {factory},", "\n") }@
@{ def.relations_one(false)|fmt_rel_join("    pub {rel_name}: Option<rel_{class_mod}::{class}Factory>,\n", "") -}@
@{ def.relations_many(false)|fmt_rel_join("    pub {rel_name}: Option<Vec<rel_{class_mod}::{class}Factory>>,\n", "") -}@
}

#[derive(Clone, Debug)]
pub@{ visibility }@ struct _@{ pascal_name }@Updater {
    pub(crate) _data: Data,
    pub(crate) _update: Data,
    pub(crate) _is_new: bool,
    pub(crate) _do_delete: bool,
    pub(crate) _upsert: bool,
    pub(crate) _is_loaded: bool,
    pub(crate) _op: OpData,
@{ def.relations_one(false)|fmt_rel_join("    pub(crate) {rel_name}: Option<Vec<rel_{class_mod}::{class}Updater>>,\n", "") -}@
@{ def.relations_many(false)|fmt_rel_join("    pub(crate) {rel_name}: Option<Vec<rel_{class_mod}::{class}Updater>>,\n", "") -}@
@{ def.relations_belonging(false)|fmt_rel_join("    pub(crate) {rel_name}: Option<Option<Box<rel_{class_mod}::{class}>>>,\n", "") -}@
}
type _Updater_ = _@{ pascal_name }@Updater;
@%- if !config.excluded_from_domain %@

#[allow(clippy::needless_update)]
impl From<domain::models::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::@{ mod_name|to_var_name }@::@{ pascal_name }@Factory> for _@{ pascal_name }@Updater {
    fn from(v: domain::models::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::@{ mod_name|to_var_name }@::@{ pascal_name }@Factory) -> Self {
        Self {
            _data: Data {
@{ def.for_factory()|fmt_join("                {var}: v.{var}{convert_from_entity},", "\n") }@
                ..Data::default()
            },
            _update: Data::default(),
            _is_new: true,
            _do_delete: false,
            _upsert: false,
            _is_loaded: true,
            _op: OpData::default(),
@{- def.relations_one(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_many(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
        }
    }
}
@%- endif %@

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct ForInsert {
    pub(crate) _data: Data,
@{- def.relations_one(false)|fmt_rel_join("
    #[serde(skip_serializing_if = \"Option::is_none\")]
    pub(crate) {rel_name}: Option<Option<Box<rel_{class_mod}::ForInsert>>>,", "") }@
@{- def.relations_many(false)|fmt_rel_join("
    #[serde(skip_serializing_if = \"Option::is_none\")]
    pub(crate) {rel_name}: Option<Vec<rel_{class_mod}::ForInsert>>,", "") }@
}

impl From<_@{ pascal_name }@Updater> for ForInsert {
    fn from(v: _@{ pascal_name }@Updater) -> Self {
        Self {
            _data: v._data,
            @{- def.relations_one(false)|fmt_rel_join("
            {rel_name}: v.{rel_name}.map(|v| v.into_iter().filter(|v| !v.will_be_deleted()).last().map(|v| Box::new(v.into()))),", "") }@
            @{- def.relations_many(false)|fmt_rel_join("
            {rel_name}: v.{rel_name}.map(|v| v.into_iter().map(|v| v.into()).collect()),", "") }@
        }
    }
}

impl From<Box<_@{ pascal_name }@Updater>> for Box<ForInsert> {
    fn from(v: Box<_@{ pascal_name }@Updater>) -> Self {
        Box::new(ForInsert {
            _data: v._data,
            @{- def.relations_one(false)|fmt_rel_join("
            {rel_name}: v.{rel_name}.map(|v| v.into_iter().filter(|v| !v.will_be_deleted()).last().map(|v| Box::new(v.into()))),", "") }@
            @{- def.relations_many(false)|fmt_rel_join("
            {rel_name}: v.{rel_name}.map(|v| v.into_iter().map(|v| v.into()).collect()),", "") }@
        })
    }
}

impl fmt::Display for ForInsert {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{INSERT: {}}}", &self._data)?;
        Ok(())
    }
}

pub@{ visibility }@ trait _@{ pascal_name }@Getter: Send + Sync + 'static {
@{ def.all_fields()|fmt_join("{label}{comment}    fn _{raw_var}(&self) -> {outer};
", "") -}@
@{ def.relations_one_and_belonging(false)|fmt_rel_join("{label}{comment}    fn _{raw_rel_name}(&self) -> Option<&rel_{class_mod}::{class}>;
", "") -}@
@{ def.relations_many(false)|fmt_rel_join("{label}{comment}    fn _{raw_rel_name}(&self) -> &Vec<rel_{class_mod}::{class}>;
", "") -}@
}

@%- for (model, rel_name, rel) in def.relations_belonging(false) %@
struct RelCol@{ rel_name|pascal }@;
impl RelCol@{ rel_name|pascal }@ {
    fn cols() -> &'static str {
        r#"@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("{col_esc}", ", ") }@"#
    }
    fn cols_with_idx(idx: usize) -> String {
        format!(r#"@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("_t{}.{col_esc}", ", ") }@"#, @{ rel.get_local_cols(rel_name, def)|fmt_join("idx", ", ") }@)
    }
}

trait RelPk@{ rel_name|pascal }@ {
    fn primary(&self) -> Option<rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Primary>;
}
impl RelPk@{ rel_name|pascal }@ for _@{ pascal_name }@ {
    fn primary(&self) -> Option<rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Primary> {
        Some(@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("self._{raw_var}(){null_question}", ", ") }@.into())
    }
}
impl RelPk@{ rel_name|pascal }@ for _@{ pascal_name }@Updater {
    fn primary(&self) -> Option<rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Primary> {
        Some(@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("self._{raw_var}(){null_question}", ", ") }@.into())
    }
}
@%- if !config.force_disable_cache %@
impl RelPk@{ rel_name|pascal }@ for _@{ pascal_name }@Cache {
    fn primary(&self) -> Option<rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Primary> {
        Some(@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("self._{raw_var}(){null_question}", ", ") }@.into())
    }
}
@%- endif %@
@%- endfor %@
@%- for (model, rel_name, rel) in def.relations_one(false) %@
struct RelCol@{ rel_name|pascal }@;
impl RelCol@{ rel_name|pascal }@ {
    fn cols() -> &'static str {
        r#"@{ rel.get_foreign_cols(def)|fmt_join_foreign("{col_esc}", ", ") }@"#
    }
    fn cols_with_paren() -> &'static str {
        r#"@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("{col_esc}", ", ") }@"#
    }
    fn set_op_none(op: &mut rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::OpData) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
        op.{var} = Op::None;", "") }@
    }
}

trait RelFil@{ rel_name|pascal }@ where Self: Sized {
    fn filter(&self) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_;
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_;
}
impl RelFil@{ rel_name|pascal }@ for _@{ pascal_name }@ {
    fn filter(&self) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@ as rel;
        let pk: Primary = self.into();
        rel::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@ as rel;
        let mut filter = rel::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(rel::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@);
        }
        filter
    }
}
impl RelFil@{ rel_name|pascal }@ for _@{ pascal_name }@Updater {
    fn filter(&self) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@ as rel;
        let pk: Primary = self.into();
        rel::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@ as rel;
        let mut filter = rel::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(rel::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@);
        }
        filter
    }
}
impl RelFil@{ rel_name|pascal }@ for &ForInsert {
    fn filter(&self) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@ as rel;
        let pk: Primary = (&self._data).into();
        rel::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@ as rel;
        let mut filter = rel::Filter_::new_or();
        for row in list {
            let pk: Primary = (&row._data).into();
            filter = filter.or(rel::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@);
        }
        filter
    }
}
@%- if !config.force_disable_cache %@
impl RelFil@{ rel_name|pascal }@ for CacheWrapper {
    fn filter(&self) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@ as rel;
        let pk: Primary = self.into();
        rel::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@ as rel;
        let mut filter = rel::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(rel::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@);
        }
        filter
    }
}
impl RelFil@{ rel_name|pascal }@ for _@{ pascal_name }@Cache {
    fn filter(&self) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@ as rel;
        let pk: Primary = self.into();
        rel::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@ as rel;
        let mut filter = rel::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(rel::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@);
        }
        filter
    }
}
@%- endif %@
pub(crate) trait RelFk@{ rel_name|pascal }@ {
    fn get_fk(&self) -> Option<Primary>;
    fn set_fk(&mut self, pk: InnerPrimary);
}
impl RelFk@{ rel_name|pascal }@ for rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Data {
    fn get_fk(&self) -> Option<Primary> {
        Some(@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("self.{raw_var}{null_question}{clone}", ", ") }@.into())
    }
    fn set_fk(&mut self, pk: InnerPrimary) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign_not_null_or_null("
        self.{raw_var} = pk.{index}{raw_to_inner};", "
        self.{raw_var} = Some(pk.{index}{raw_to_inner});", "") }@
    }
}
@%- if !config.force_disable_cache %@
impl RelFk@{ rel_name|pascal }@ for rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::CacheData {
    fn get_fk(&self) -> Option<Primary> {
        Some(@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("self.{raw_var}{null_question}{clone}", ", ") }@.into())
    }
    fn set_fk(&mut self, pk: InnerPrimary) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign_not_null_or_null("
        self.{raw_var} = pk.{index}{raw_to_inner};", "
        self.{raw_var} = Some(pk.{index}{raw_to_inner});", "") }@
    }
}
@%- endif %@
impl RelFk@{ rel_name|pascal }@ for rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::ForInsert {
    fn get_fk(&self) -> Option<Primary> {
        Some(@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("self._data.{raw_var}{null_question}{clone}", ", ") }@.into())
    }
    fn set_fk(&mut self, pk: InnerPrimary) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign_not_null_or_null("
        self._data.{raw_var} = pk.{index}{raw_to_inner};", "
        self._data.{raw_var} = Some(pk.{index}{raw_to_inner});", "") }@
    }
}
@%- endfor %@
@%- for (model, rel_name, rel) in def.relations_many(false) %@
struct RelCol@{ rel_name|pascal }@;
impl RelCol@{ rel_name|pascal }@ {
    fn cols() -> &'static str {
        r#"@{ rel.get_foreign_cols(def)|fmt_join_foreign("{col_esc}", ", ") }@"#
    }
    fn cols_with_paren() -> &'static str {
        r#"@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("{col_esc}", ", ") }@"#
    }
    fn set_op_none(op: &mut rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::OpData) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
        op.{var} = Op::None;", "") }@
    }
}

trait RelFil@{ rel_name|pascal }@ where Self: Sized {
    fn filter(&self) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_;
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_;
}
impl RelFil@{ rel_name|pascal }@ for _@{ pascal_name }@ {
    fn filter(&self) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@ as rel;
        let pk: Primary = self.into();
        rel::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@ as rel;
        let mut filter = rel::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(rel::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@);
        }
        filter
    }
}
impl RelFil@{ rel_name|pascal }@ for _@{ pascal_name }@Updater {
    fn filter(&self) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@ as rel;
        let pk: Primary = self.into();
        rel::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@ as rel;
        let mut filter = rel::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(rel::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@);
        }
        filter
    }
}
impl RelFil@{ rel_name|pascal }@ for &ForInsert {
    fn filter(&self) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@ as rel;
        let pk: Primary = (&self._data).into();
        rel::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@ as rel;
        let mut filter = rel::Filter_::new_or();
        for row in list {
            let pk: Primary = (&row._data).into();
            filter = filter.or(rel::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@);
        }
        filter
    }
}
@%- if !config.force_disable_cache %@
impl RelFil@{ rel_name|pascal }@ for CacheWrapper {
    fn filter(&self) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@ as rel;
        let pk: Primary = self.into();
        rel::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@ as rel;
        let mut filter = rel::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(rel::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@);
        }
        filter
    }
}
impl RelFil@{ rel_name|pascal }@ for _@{ pascal_name }@Cache {
    fn filter(&self) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@ as rel;
        let pk: Primary = self.into();
        rel::Filter_::new_and()
        @{- rel.get_foreign_cols(def)|fmt_join_foreign("
            .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@
    }
    fn in_filter(list: &[Self]) -> rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Filter_ {
        use rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@ as rel;
        let mut filter = rel::Filter_::new_or();
        for row in list {
            let pk: Primary = row.into();
            filter = filter.or(rel::Filter_::new_and()
            @{- rel.get_foreign_cols(def)|fmt_join_foreign("
                .and(rel::Filter_::Eq(rel::ColOne_::{var}(pk.{index}.inner().into())))", "") }@);
        }
        filter
    }
}
@%- endif %@
pub(crate) trait RelFk@{ rel_name|pascal }@ {
    fn get_fk(&self) -> Option<Primary>;
    fn set_fk(&mut self, pk: InnerPrimary);
}
impl RelFk@{ rel_name|pascal }@ for rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::Data {
    fn get_fk(&self) -> Option<Primary> {
        Some(@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("self.{raw_var}{null_question}{clone}", ", ") }@.into())
    }
    fn set_fk(&mut self, pk: InnerPrimary) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign_not_null_or_null("
        self.{raw_var} = pk.{index}{raw_to_inner};", "
        self.{raw_var} = Some(pk.{index}{raw_to_inner});", "") }@
    }
}
@%- if !config.force_disable_cache %@
impl RelFk@{ rel_name|pascal }@ for rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::CacheData {
    fn get_fk(&self) -> Option<Primary> {
        Some(@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("self.{raw_var}{null_question}{clone}", ", ") }@.into())
    }
    fn set_fk(&mut self, pk: InnerPrimary) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign_not_null_or_null("
        self.{raw_var} = pk.{index}{raw_to_inner};", "
        self.{raw_var} = Some(pk.{index}{raw_to_inner});", "") }@
    }
}
@%- endif %@
impl RelFk@{ rel_name|pascal }@ for rel_@{ rel.get_group_name() }@_@{ rel.get_mod_name() }@::ForInsert {
    fn get_fk(&self) -> Option<Primary> {
        Some(@{ rel.get_foreign_cols(def)|fmt_join_foreign_with_paren("self._data.{raw_var}{null_question}{clone}", ", ") }@.into())
    }
    fn set_fk(&mut self, pk: InnerPrimary) {
        @{- rel.get_foreign_cols(def)|fmt_join_foreign_not_null_or_null("
        self._data.{raw_var} = pk.{index}{raw_to_inner};", "
        self._data.{raw_var} = Some(pk.{index}{raw_to_inner});", "") }@
    }
}
@%- endfor %@

#[async_trait]
pub@{ visibility }@ trait _@{ pascal_name }@Joiner {
    async fn join(&mut self, conn: &mut DbConn, joiner: Option<Box<Joiner_>>) -> Result<()> {
        if let Some(joiner) = joiner {
            @{- def.relations()|fmt_rel_join("
            if joiner.{rel_name}.is_some() {
                self.join_{raw_rel_name}(conn, joiner.{rel_name}).await?;
            }", "") }@
        }
        Ok(())
    }
@{- def.relations_one_and_belonging(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()>;", "") }@
@{- def.relations_many(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()>;", "") }@
}

#[async_trait]
impl _@{ pascal_name }@Joiner for _@{ pascal_name }@ {
@{- def.relations_belonging(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        if self.{rel_name}.is_some() {
            return Ok(());
        }
        if let Some(id) = RelPk{rel_name_pascal}::primary(self) {
            let mut obj = rel_{class_mod}::{class}::find_optional{with_trashed}(conn, id, None).await?;
            if let Some(mut obj) = obj {
                rel_{class_mod}::{class}Joiner::join(&mut obj, conn, joiner).await?;
                self.{rel_name} = Some(Some(Box::new(obj)));
            } else {
                self.{rel_name} = Some(None);
            }
        } else {
            self.{rel_name} = Some(None);
        }
        Ok(())
    }", "") }@
@{- def.relations_one(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        if self.{rel_name}.is_some() {
            return Ok(());
        }
        let filter = RelFil{rel_name_pascal}::filter(self){additional_filter};
        self.{rel_name} = Some(rel_{class_mod}::{class}::query().filter(filter).join(joiner).select(conn).await?.pop().map(Box::new));
        Ok(())
    }", "") }@
@{- def.relations_many(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        if self.{rel_name}.is_some() {
            return Ok(());
        }
        let filter = RelFil{rel_name_pascal}::filter(self){additional_filter};
        let order = vec![{order}];
        self.{rel_name} = Some(rel_{class_mod}::{class}::query().filter(filter).join(joiner).order_by(order){limit}.select(conn).await?);
        Ok(())
    }", "") }@
}
@%- if !def.disable_update() %@

#[async_trait]
impl _@{ pascal_name }@Joiner for _@{ pascal_name }@Updater {
@{- def.relations_belonging(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        if self.{rel_name}.is_some() {
            return Ok(());
        }
        if let Some(id) = RelPk{rel_name_pascal}::primary(self) {
            let mut obj = rel_{class_mod}::{class}::find_optional{with_trashed}(conn, id, None).await?;
            if let Some(mut obj) = obj {
                rel_{class_mod}::{class}Joiner::join(&mut obj, conn, joiner).await?;
                self.{rel_name} = Some(Some(Box::new(obj)));
            } else {
                self.{rel_name} = Some(None);
            }
        } else {
            self.{rel_name} = Some(None);
        }
        Ok(())
    }", "") }@
@{- def.relations_one(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        if self.{rel_name}.is_some() {
            return Ok(());
        }
        let filter = RelFil{rel_name_pascal}::filter(self){additional_filter};
        self.{rel_name} = Some(rel_{class_mod}::{class}::query().filter(filter).join(joiner).select_for_update(conn).await?);
        Ok(())
    }", "") }@
@{- def.relations_many(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        if self.{rel_name}.is_some() {
            return Ok(());
        }
        let filter = RelFil{rel_name_pascal}::filter(self){additional_filter};
        let order = vec![{order}];
        self.{rel_name} = Some(rel_{class_mod}::{class}::query().filter(filter).join(joiner).order_by(order){limit}.select_for_update(conn).await?);
        Ok(())
    }", "") }@
}
@%- endif %@
@%- if !config.force_disable_cache %@

#[cfg(not(feature="cache_update_only"))]
impl CacheWrapper {
@{- def.relations_one_cache(false)|fmt_rel_join("
    async fn fetch_{raw_rel_name}(&mut self, conn: &mut DbConn) -> Result<()> {
        let filter = RelFil{rel_name_pascal}::filter(self){additional_filter};
        self.{rel_name} = rel_{class_mod}::{class}::query().filter(filter).__select_for_cache(conn).await?.into_iter().map(|v| v._wrapper).next();
        Ok(())
    }", "") }@
@{- def.relations_many_cache(false)|fmt_rel_join("
    async fn fetch_{raw_rel_name}(&mut self, conn: &mut DbConn) -> Result<()> {
        let filter = RelFil{rel_name_pascal}::filter(self){additional_filter};
        let order = vec![{order}];
        self.{rel_name} = rel_{class_mod}::{class}::query().filter(filter).order_by(order){limit}.__select_for_cache(conn).await?.into_iter().map(|v| v._wrapper).collect();
        Ok(())
    }", "") }@
}

#[async_trait]
impl _@{ pascal_name }@Joiner for _@{ pascal_name }@Cache {
@{- def.relations_belonging_cache(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        if let Some(id) = RelPk{rel_name_pascal}::primary(self) {
            let mut obj = rel_{class_mod}::{class}::find_optional_from_cache{with_trashed}(conn, id).await?;
            if let Some(mut obj) = obj {
                rel_{class_mod}::{class}Joiner::join(&mut obj, conn, joiner).await?;
                self.{rel_name} = Some(Some(Box::new(obj)));
            } else {
                self.{rel_name} = Some(None);
            }
        } else {
            self.{rel_name} = Some(None);
        }
        Ok(())
    }", "") }@
@{- def.relations_belonging_uncached(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        if self.{rel_name}.is_some() {
            return Ok(());
        }
        if let Some(id) = RelPk{rel_name_pascal}::primary(self) {
            let mut obj = rel_{class_mod}::{class}::find_optional{with_trashed}(conn, id, None).await?;
            if let Some(mut obj) = obj {
                rel_{class_mod}::{class}Joiner::join(&mut obj, conn, joiner).await?;
                self.{rel_name} = Some(Some(Box::new(obj)));
            } else {
                self.{rel_name} = Some(None);
            }
        } else {
            self.{rel_name} = Some(None);
        }
        Ok(())
    }", "") }@
@{- def.relations_one_cache(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        if !matches!(joiner.as_ref().map(|v| v.has_some()), Some(true)) {
            return Ok(());
        }
        if let Some(ref obj) = self._wrapper.{rel_name} {
            let id: rel_{class_mod}::InnerPrimary = obj.into();
            let mut obj = rel_{class_mod}::{class}::find_optional_from_cache{ignore_soft_delete}(conn, &id).await?;
            if let Some(mut obj) = obj {
                rel_{class_mod}::{class}Joiner::join(&mut obj, conn, joiner).await?;
                self.{rel_name} = Some(Some(Box::new(obj)));
            } else {
                self.{rel_name} = Some(None);
            }
        }
        Ok(())
    }", "") }@
@{- def.relations_one_uncached(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        if self.{rel_name}.is_some() {
            return Ok(());
        }
        let filter = RelFil{rel_name_pascal}::filter(self){additional_filter};
        self.{rel_name} = Some(rel_{class_mod}::{class}::query().filter(filter).join(joiner).select(conn).await?.pop().map(Box::new));
        Ok(())
    }", "") }@
@{- def.relations_many_cache(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        if !matches!(joiner.as_ref().map(|v| v.has_some()), Some(true)) {
            return Ok(());
        }
        let ids: Vec<rel_{class_mod}::InnerPrimary> = self._wrapper.{rel_name}.iter().map(|v| v.into()).collect();
        if !ids.is_empty() {
            let mut list = rel_{class_mod}::{class}::find_many_from_cache{ignore_soft_delete}(conn, ids.iter()).await?;
            rel_{class_mod}::{class}Joiner::join(&mut list, conn, joiner).await?;
            let mut map: AHashMap<_, _> = list.into_iter().map(|v| (rel_{class_mod}::InnerPrimary::from(&v), v)).collect();
            self.{rel_name} = Some(ids.iter().flat_map(|id| map.remove(id)).collect());
        } else {
            self.{rel_name} = Some(Vec::new());
        }
        Ok(())
    }", "") }@
@{- def.relations_many_uncached(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        if self.{rel_name}.is_some() {
            return Ok(());
        }
        let filter = RelFil{rel_name_pascal}::filter(self){additional_filter};
        let order = vec![{order}];
        self.{rel_name} = Some(rel_{class_mod}::{class}::query().filter(filter).join(joiner).order_by(order){limit}.select(conn).await?);
        Ok(())
    }", "") }@
}
@%- endif %@

#[async_trait]
impl _@{ pascal_name }@Joiner for Vec<_@{ pascal_name }@> {
@{- def.relations_belonging(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        let ids: FxHashSet<_> = self.iter().flat_map(RelPk{rel_name_pascal}::primary).collect();
        if ids.is_empty() { return Ok(()); }
        let mut list = rel_{class_mod}::{class}::find_many{with_trashed}(conn, ids.iter()).await?;
        rel_{class_mod}::{class}Joiner::join(&mut list, conn, joiner).await?;
        let map = rel_{class_mod}::{class}::list_to_map(list);
        for val in self.iter_mut() {
            if let Some(id) = RelPk{rel_name_pascal}::primary(val) {
                val.{rel_name} = Some(map.get(&id).map(|v| Box::new(v.clone())));
            } else {
                val.{rel_name} = Some(None);
            }
        }
        Ok(())
    }", "") }@
@{- def.relations_one(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        if self.is_empty() { return Ok(()); }
        let union: Vec<_> = self.iter().map(|v| {
            let filter = RelFil{rel_name_pascal}::filter(v){additional_filter};
            rel_{class_mod}::{class}::query().filter(filter)
        }).collect();
        use rel_{class_mod}::UnionBuilder;
        let mut list = union.select(conn, None, None, None).await?;
        rel_{class_mod}::{class}Joiner::join(&mut list, conn, joiner).await?;
        let mut map = AHashMap::default();
        for row in list {
            if let Some(id) = RelFk{rel_name_pascal}::get_fk(&row._inner) {
                map.insert(id, Box::new(row));
            }
        }
        for val in self.iter_mut() {
            val.{rel_name} = Some(map.remove(&(&*val).into()));
        }
        Ok(())
    }", "") }@
@{- def.relations_many(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        if self.is_empty() { return Ok(()); }
        let union: Vec<_> = self.iter().map(|v| {
            let filter = RelFil{rel_name_pascal}::filter(v){additional_filter};
            rel_{class_mod}::{class}::query().filter(filter){order_and_limit}
        }).collect();
        use rel_{class_mod}::UnionBuilder;
        let mut list = union.select(conn, None, None, None).await?;
        rel_{class_mod}::{class}Joiner::join(&mut list, conn, joiner).await?;
        let mut map = AHashMap::default();
        for row in list {
            if let Some(id) = RelFk{rel_name_pascal}::get_fk(&row._inner){
                map.entry(id).or_insert_with(Vec::new).push(row);
            }
        }
        for val in self.iter_mut() {
            let mut l = map.remove(&(&*val).into()).unwrap_or_default();
            {list_sort}
            val.{rel_name} = Some(l);
        }
        Ok(())
    }", "") }@
}
@%- if !def.disable_update() %@

#[async_trait]
impl _@{ pascal_name }@Joiner for Vec<_@{ pascal_name }@Updater> {
@{- def.relations_belonging(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        let ids: FxHashSet<_> = self.iter().flat_map(RelPk{rel_name_pascal}::primary).collect();
        if ids.is_empty() { return Ok(()); }
        let mut list = rel_{class_mod}::{class}::find_many{with_trashed}(conn, ids.iter()).await?;
        rel_{class_mod}::{class}Joiner::join(&mut list, conn, joiner).await?;
        let map = rel_{class_mod}::{class}::list_to_map(list);
        for val in self.iter_mut() {
            if let Some(id) = RelPk{rel_name_pascal}::primary(val) {
                val.{rel_name} = Some(map.get(&id).map(|v| Box::new(v.clone())));
            } else {
                val.{rel_name} = Some(None);
            }
        }
        Ok(())
    }", "") }@
@{- def.relations_one(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        if self.is_empty() { return Ok(()); }
        let union: Vec<_> = self.iter().map(|v| {
            let filter = RelFil{rel_name_pascal}::filter(v){additional_filter};
            rel_{class_mod}::{class}::query().filter(filter)
        }).collect();
        use rel_{class_mod}::UnionBuilder;
        let mut list = union.select_for_update(conn).await?;
        rel_{class_mod}::{class}Joiner::join(&mut list, conn, joiner).await?;
        let mut map = AHashMap::default();
        for row in list {
            if let Some(id) = RelFk{rel_name_pascal}::get_fk(&row._data) {
                map.entry(id).or_insert_with(Vec::new).push(row);
            }
        }
        for val in self.iter_mut() {
            let mut l = map.remove(&(&*val).into()).unwrap_or_default();
            {list_sort_for_update}
            val.{rel_name} = Some(l);
        }
        Ok(())
    }", "") }@
@{- def.relations_many(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        if self.is_empty() { return Ok(()); }
        let union: Vec<_> = self.iter().map(|v| {
            let filter = RelFil{rel_name_pascal}::filter(v){additional_filter};
            rel_{class_mod}::{class}::query().filter(filter){order_and_limit}
        }).collect();
        use rel_{class_mod}::UnionBuilder;
        let mut list = union.select_for_update(conn).await?;
        rel_{class_mod}::{class}Joiner::join(&mut list, conn, joiner).await?;
        let mut map = AHashMap::default();
        for row in list {
            if let Some(id) = RelFk{rel_name_pascal}::get_fk(&row._data) {
                map.entry(id).or_insert_with(Vec::new).push(row);
            }
        }
        for val in self.iter_mut() {
            let mut l = map.remove(&(&*val).into()).unwrap_or_default();
            {list_sort_for_update}
            val.{rel_name} = Some(l);
        }
        Ok(())
    }", "") }@
}
@%- endif %@
@%- if !config.force_disable_cache %@

#[cfg(not(feature="cache_update_only"))]
impl CacheWrapper {
@{- def.relations_one_cache(false)|fmt_rel_join("
    async fn fetch_{raw_rel_name}_for_vec(vec: &mut [CacheWrapper], conn: &mut DbConn) -> Result<()> {
        if vec.is_empty() { return Ok(()); }
        let union: Vec<_> = vec.iter().map(|v| {
            let filter = RelFil{rel_name_pascal}::filter(v){additional_filter};
            rel_{class_mod}::{class}::query().filter(filter)
        }).collect();
        use rel_{class_mod}::_UnionBuilder;
        let list: Vec<Arc<_>> = union.__select_for_cache(conn).await?.into_iter().map(|v| v._wrapper).collect();
        let mut map = AHashMap::default();
        for row in list {
            if let Some(id) = RelFk{rel_name_pascal}::get_fk(&row._inner) {
                map.insert(id, row);
            }
        }
        for val in vec.iter_mut() {
            val.{rel_name} = map.remove(&(&*val).into());
        }
        Ok(())
    }", "") }@
@{- def.relations_many_cache(false)|fmt_rel_join("
    async fn fetch_{raw_rel_name}_for_vec(vec: &mut [CacheWrapper], conn: &mut DbConn) -> Result<()> {
        if vec.is_empty() { return Ok(()); }
        let union: Vec<_> = vec.iter().map(|v| {
            let filter = RelFil{rel_name_pascal}::filter(v){additional_filter};
            rel_{class_mod}::{class}::query().filter(filter){order_and_limit}
        }).collect();
        use rel_{class_mod}::_UnionBuilder;
        let list: Vec<Arc<_>> = union.__select_for_cache(conn).await?.into_iter().map(|v| v._wrapper).collect();
        let mut map = AHashMap::default();
        for row in list {
            if let Some(id) = RelFk{rel_name_pascal}::get_fk(&row._inner) {
                map.entry(id).or_insert_with(Vec::new).push(row);
            }
        }
        for val in vec.iter_mut() {
            let mut l = map.remove(&(&*val).into()).unwrap_or_default();
            {list_sort}
            val.{rel_name} = l;
        }
        Ok(())
    }", "") }@
}

#[async_trait]
impl _@{ pascal_name }@Joiner for Vec<_@{ pascal_name }@Cache> {
@{- def.relations_one_cache(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        if !matches!(joiner.as_ref().map(|v| v.has_some()), Some(true)) {
            return Ok(());
        }
        let ids: Vec<rel_{class_mod}::InnerPrimary> = self.iter().flat_map(|v| &v._wrapper.{rel_name}).map(|v| v.into()).collect();
        if ids.is_empty() { return Ok(()); }
        let mut list = rel_{class_mod}::{class}::find_many_from_cache{ignore_soft_delete}(conn, ids.iter()).await?;
        rel_{class_mod}::{class}Joiner::join(&mut list, conn, joiner).await?;
        let mut map: AHashMap<_, _> = list.into_iter().map(|v| (rel_{class_mod}::InnerPrimary::from(&v), v)).collect();
        for val in self.iter_mut() {
            if let Some(v) = &val._wrapper.{rel_name} {
                val.{rel_name} = Some(map.remove(&v.into()).map(Box::new));
            } else {
                val.{rel_name} = Some(None);
            }
        }
        Ok(())
    }", "") }@
@{- def.relations_one_uncached(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        if self.is_empty() { return Ok(()); }
        let union: Vec<_> = self.iter().map(|v| {
            let filter = RelFil{rel_name_pascal}::filter(v){additional_filter};
            rel_{class_mod}::{class}::query().filter(filter)
        }).collect();
        use rel_{class_mod}::UnionBuilder;
        let mut list = union.select(conn, None, None, None).await?;
        rel_{class_mod}::{class}Joiner::join(&mut list, conn, joiner).await?;
        let mut map = AHashMap::default();
        for row in list {
            if let Some(id) = RelFk{rel_name_pascal}::get_fk(&row._inner) {
                map.insert(id, Box::new(row));
            }
        }
        for val in self.iter_mut() {
            val.{rel_name} = Some(map.remove(&(&*val).into()));
        }
        Ok(())
    }", "") }@
@{- def.relations_many_cache(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        if !matches!(joiner.as_ref().map(|v| v.has_some()), Some(true)) {
            return Ok(());
        }
        let ids: Vec<rel_{class_mod}::InnerPrimary> = self.iter().flat_map(|v| &v._wrapper.{rel_name}).map(|v| v.into()).collect();
        if ids.is_empty() { return Ok(()); }
        let mut list = rel_{class_mod}::{class}::find_many_from_cache{ignore_soft_delete}(conn, ids.iter()).await?;
        rel_{class_mod}::{class}Joiner::join(&mut list, conn, joiner).await?;
        let mut map: AHashMap<_, _> = list.into_iter().map(|v| (rel_{class_mod}::InnerPrimary::from(&v), v)).collect();
        for val in self.iter_mut() {
            val.{rel_name} = Some(val._wrapper.{rel_name}.iter().flat_map(|v| map.remove(&v.into())).collect());
        }
        Ok(())
    }", "") }@
@{- def.relations_many_uncached(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        if self.is_empty() { return Ok(()); }
        let union: Vec<_> = self.iter().map(|v| {
            let filter = RelFil{rel_name_pascal}::filter(v){additional_filter};
            rel_{class_mod}::{class}::query().filter(filter){order_and_limit}
        }).collect();
        use rel_{class_mod}::UnionBuilder;
        let mut list = union.select(conn, None, None, None).await?;
        rel_{class_mod}::{class}Joiner::join(&mut list, conn, joiner).await?;
        let mut map = AHashMap::default();
        for row in list {
            if let Some(id) = RelFk{rel_name_pascal}::get_fk(&row._inner){
                map.entry(id).or_insert_with(Vec::new).push(row);
            }
        }
        for val in self.iter_mut() {
            let mut l = map.remove(&(&*val).into()).unwrap_or_default();
            {list_sort}
            val.{rel_name} = Some(l);
        }
        Ok(())
    }", "") }@
@{- def.relations_belonging_cache(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        let ids: FxHashSet<_> = self.iter().flat_map(RelPk{rel_name_pascal}::primary).collect();
        if ids.is_empty() { return Ok(()); }
        let mut list = rel_{class_mod}::{class}::find_many_from_cache{with_trashed}(conn, ids.iter()).await?;
        rel_{class_mod}::{class}Joiner::join(&mut list, conn, joiner).await?;
        let map = rel_{class_mod}::{class}::cache_list_to_map(list);
        for val in self.iter_mut() {
            if let Some(id) = RelPk{rel_name_pascal}::primary(val) {
                val.{rel_name} = Some(map.get(&id).map(|v| Box::new(v.clone())));
            } else {
                val.{rel_name} = Some(None);
            }
        }
        Ok(())
    }", "") }@
@{- def.relations_belonging_uncached(false)|fmt_rel_join("
    async fn join_{raw_rel_name}(&mut self, conn: &mut DbConn, joiner: Option<Box<join_{class_mod}::Joiner_>>) -> Result<()> {
        let ids: FxHashSet<_> = self.iter().flat_map(RelPk{rel_name_pascal}::primary).collect();
        if ids.is_empty() { return Ok(()); }
        let mut list = rel_{class_mod}::{class}::find_many{with_trashed}(conn, ids.iter()).await?;
        rel_{class_mod}::{class}Joiner::join(&mut list, conn, joiner).await?;
        let map = rel_{class_mod}::{class}::list_to_map(list);
        for val in self.iter_mut() {
            if let Some(id) = RelPk{rel_name_pascal}::primary(val) {
                val.{rel_name} = Some(map.get(&id).map(|v| Box::new(v.clone())));
            } else {
                val.{rel_name} = Some(None);
            }
        }
        Ok(())
    }", "") }@
}
@%- endif %@
@%- if config.excluded_from_domain %@

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub@{ visibility }@ enum Col_ {
@{ def.all_fields()|fmt_join("    {var},", "\n") }@
}
@%- else %@
pub(crate) use domain::models::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::_base::_@{ mod_name }@::Col_;
@%- endif %@
impl ColTr for Col_ {
    fn name(&self) -> &'static str {
        match self {
@{ def.all_fields()|fmt_join("            Col_::{var} => r#\"{col_esc}\"#,", "\n") }@
        }
    }
}
@%- if config.excluded_from_domain %@

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub@{ visibility }@ enum ColOne_ {
@{ def.all_fields_without_json()|fmt_join("    {var}({filter_type}),", "\n") }@
@%- for (index_name, index) in def.multi_index() %@
    @{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "{type}", ", ") }@),
@%- endfor %@
}
@%- else %@
pub(crate) use domain::models::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::_base::_@{ mod_name }@::ColOne_;
@%- endif %@
#[allow(clippy::match_single_binding)]
impl BindTr for ColOne_ {
    fn name(&self) -> &'static str {
        match self {
@{ def.all_fields_without_json()|fmt_join("            ColOne_::{var}(_) => r#\"{col_esc}\"#,", "\n") }@
@%- for (index_name, index) in def.multi_index() %@
            ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "_", ", ") }@) => r#"(@{ index.join_fields(def, "{col_esc}", ", ") }@)"#,
@%- endfor %@
            _ => unreachable!(),
        }
    }
    fn placeholder(&self) -> &'static str {
        match self {
@{ def.all_fields_without_json()|fmt_join("            ColOne_::{var}(_) => \"{placeholder}\",", "\n") }@
@%- for (index_name, index) in def.multi_index() %@
            ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "_", ", ") }@) => "(@{ index.join_fields(def, "{placeholder}", ", ") }@)",
@%- endfor %@
            _ => "?",
        }
    }
    fn query_as_bind<T>(
        self,
        query: QueryAs<'_, DbType, T, DbArguments>,
    ) -> QueryAs<'_, DbType, T, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
@{ def.all_fields_without_json()|fmt_join("            ColOne_::{var}(v) => query.bind(v{bind_as_for_filter}),", "\n") }@
@%- for (index_name, index) in def.multi_index() %@
            ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "v{index}", ", ") }@) => query@{ index.join_fields(def, ".bind(v{index}{bind_as_for_filter})", "") }@,
@%- endfor %@
            _ => unreachable!(),
        }
    }
    fn query_bind(
        self,
        query: Query<'_, DbType, DbArguments>,
    ) -> Query<'_, DbType, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
@{ def.all_fields_without_json()|fmt_join("            ColOne_::{var}(v) => query.bind(v{bind_as_for_filter}),", "\n") }@
@%- for (index_name, index) in def.multi_index() %@
            ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "v{index}", ", ") }@) => query@{ index.join_fields(def, ".bind(v{index}{bind_as_for_filter})", "") }@,
@%- endfor %@
            _ => unreachable!(),
        }
    }
}
@%- if config.excluded_from_domain %@

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Hash, Serialize)]
pub@{ visibility }@ enum ColKey_ {
    @{- def.unique_key()|fmt_index_col("
    {var}({filter_type}),", "") }@
}
@%- else %@
pub(crate) use domain::models::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::_base::_@{ mod_name }@::ColKey_;
@%- endif %@
#[allow(clippy::match_single_binding)]
impl BindTr for ColKey_ {
    fn name(&self) -> &'static str {
        match self {
            @{- def.unique_key()|fmt_index_col("
            ColKey_::{var}(_v) => r#\"{col_esc}\"#,", "") }@
            _ => unreachable!(),
        }
    }
    fn query_as_bind<T>(
        self,
        query: QueryAs<'_, DbType, T, DbArguments>,
    ) -> QueryAs<'_, DbType, T, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
            @{- def.unique_key()|fmt_index_col("
            ColKey_::{var}(v) => query.bind(v{bind_as_for_filter}),", "") }@
            _ => unreachable!(),
        }
    }
    fn query_bind(
        self,
        query: Query<'_, DbType, DbArguments>,
    ) -> Query<'_, DbType, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
            @{- def.unique_key()|fmt_index_col("
            ColKey_::{var}(v) => query.bind(v{bind_as_for_filter}),", "") }@
            _ => unreachable!(),
        }
    }
}
struct VecColKey(Vec<ColKey_>);
@%- if !config.force_disable_cache %@
impl HashVal for VecColKey {
    fn hash_val(&self, shard_id: ShardId) -> u128 {
        let mut hasher = FxHasher64::default();
        COL_KEY_TYPE_ID.hash(&mut hasher);
        shard_id.hash(&mut hasher);
        self.0.hash(&mut hasher);
        let hash = (hasher.finish() as u128) << 64;

        let mut hasher = AHasher::default();
        COL_KEY_TYPE_ID.hash(&mut hasher);
        shard_id.hash(&mut hasher);
        self.0.hash(&mut hasher);
        hash | (hasher.finish() as u128)
    }
}
@%- endif %@
@%- if config.excluded_from_domain %@

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub@{ visibility }@ enum ColMany_ {
@{ def.all_fields_without_json()|fmt_join("    {var}(Vec<{filter_type}>),", "\n") }@
@%- for (index_name, index) in def.multi_index() %@
    @{ index.join_fields(def, "{name}", "_") }@(Vec<(@{ index.join_fields(def, "{type}", ", ") }@)>),
@%- endfor %@
}
@%- else %@
pub(crate) use domain::models::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::_base::_@{ mod_name }@::ColMany_;
@%- endif %@
#[allow(clippy::match_single_binding)]
impl BindTr for ColMany_ {
    fn name(&self) -> &'static str {
        match self {
@{ def.all_fields_without_json()|fmt_join("            ColMany_::{var}(_) => r#\"{col_esc}\"#,", "\n") }@
@%- for (index_name, index) in def.multi_index() %@
            ColMany_::@{ index.join_fields(def, "{name}", "_") }@(_v) => r#"(@{ index.join_fields(def, "{col_esc}", ", ") }@)"#,
@%- endfor %@
            _ => unreachable!(),
        }
    }
    fn placeholder(&self) -> &'static str {
        match self {
@{ def.all_fields_without_json()|fmt_join("            ColMany_::{var}(_) => \"{placeholder}\",", "\n") }@
@%- for (index_name, index) in def.multi_index() %@
            ColMany_::@{ index.join_fields(def, "{name}", "_") }@(_v) => "(@{ index.join_fields(def, "{placeholder}", ", ") }@)",
@%- endfor %@
            _ => "?",
        }
    }
    fn len(&self) -> usize {
        match self {
@{ def.all_fields_without_json()|fmt_join("            ColMany_::{var}(v) => v.len(),", "\n") }@
@%- for (index_name, index) in def.multi_index() %@
            ColMany_::@{ index.join_fields(def, "{name}", "_") }@(v) => v.len(),
@%- endfor %@
            _ => unreachable!(),
        }
    }
    fn query_as_bind<T>(
        self,
        mut query: QueryAs<'_, DbType, T, DbArguments>,
    ) -> QueryAs<'_, DbType, T, DbArguments> {
        debug!("bind: {:?}", &self);
        // To improve build speed, do not use fold.
        match self {
@{ def.all_fields_without_json()|fmt_join("            ColMany_::{var}(v) => {for v in v { query = query.bind(v{bind_as_for_filter}); } query},", "\n") }@
@%- for (index_name, index) in def.multi_index() %@
            ColMany_::@{ index.join_fields(def, "{name}", "_") }@(v) => {for v in v { query = query@{ index.join_fields(def, ".bind(v.{index}{bind_as_for_filter})", "") }@; } query},
@%- endfor %@
            _ => unreachable!(),
        }
    }
    fn query_bind(
        self,
        mut query: Query<'_, DbType, DbArguments>,
    ) -> Query<'_, DbType, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
@{ def.all_fields_without_json()|fmt_join("            ColMany_::{var}(v) => {for v in v { query = query.bind(v{bind_as_for_filter}); } query},", "\n") }@
@%- for (index_name, index) in def.multi_index() %@
            ColMany_::@{ index.join_fields(def, "{name}", "_") }@(v) => {for v in v { query = query@{ index.join_fields(def, ".bind(v.{index}{bind_as_for_filter})", "") }@; } query},
@%- endfor %@
            _ => unreachable!(),
        }
    }
}
@%- if config.excluded_from_domain %@

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub@{ visibility }@ enum ColJson_ {
@{- def.all_fields_only_json()|fmt_join("
    {var}(Value),", "") }@
}
@%- else %@
pub(crate) use domain::models::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::_base::_@{ mod_name }@::ColJson_;
@%- endif %@
#[allow(clippy::match_single_binding)]
impl BindTr for ColJson_ {
    fn name(&self) -> &'static str {
        match self {
@{- def.all_fields_only_json()|fmt_join("
            ColJson_::{var}(_v) => r#\"{col_esc}\"#,", "") }@
            _ => unreachable!(),
        }
    }
    fn query_as_bind<T>(
        self,
        query: QueryAs<'_, DbType, T, DbArguments>,
    ) -> QueryAs<'_, DbType, T, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
@{- def.all_fields_only_json()|fmt_join("
            ColJson_::{var}(v) => query.bind(v{bind_as_for_filter}),", "") }@
            _ => unreachable!(),
        }
    }
    fn query_bind(
        self,
        query: Query<'_, DbType, DbArguments>,
    ) -> Query<'_, DbType, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
@{- def.all_fields_only_json()|fmt_join("
            ColJson_::{var}(v) => query.bind(v{bind_as_for_filter}),", "") }@
            _ => unreachable!(),
        }
    }
}
@%- if config.excluded_from_domain %@

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub@{ visibility }@ enum ColJsonArray_ {
@{- def.all_fields_only_json()|fmt_join("
    {var}(Vec<Value>),", "") }@
}
@%- else %@
pub(crate) use domain::models::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::_base::_@{ mod_name }@::ColJsonArray_;
@%- endif %@
#[allow(clippy::match_single_binding)]
impl BindTr for ColJsonArray_ {
    fn name(&self) -> &'static str {
        match self {
@{- def.all_fields_only_json()|fmt_join("
            ColJsonArray_::{var}(_v) => r#\"{col_esc}\"#,", "") }@
            _ => unreachable!(),
        }
    }
    fn query_as_bind<T>(
        self,
        query: QueryAs<'_, DbType, T, DbArguments>,
    ) -> QueryAs<'_, DbType, T, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
@{- def.all_fields_only_json()|fmt_join("
            ColJsonArray_::{var}(v) => query.bind(sqlx::types::Json(v{bind_as_for_filter})),", "") }@
            _ => unreachable!(),
        }
    }
    fn query_bind(
        self,
        query: Query<'_, DbType, DbArguments>,
    ) -> Query<'_, DbType, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
@{- def.all_fields_only_json()|fmt_join("
            ColJsonArray_::{var}(v) => query.bind(sqlx::types::Json(v{bind_as_for_filter})),", "") }@
            _ => unreachable!(),
        }
    }
}
impl BindArrayTr for ColJsonArray_ {
    fn query_as_each_bind<T>(
        self,
        mut query: QueryAs<'_, DbType, T, DbArguments>,
    ) -> QueryAs<'_, DbType, T, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
@{- def.all_fields_only_json()|fmt_join("
            ColJsonArray_::{var}(v) => {for v in v { query = query.bind(v{bind_as_for_filter}); } query},", "") }@
            _ => unreachable!(),
        }
    }
    fn query_each_bind(
        self,
        mut query: Query<'_, DbType, DbArguments>,
    ) -> Query<'_, DbType, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
@{- def.all_fields_only_json()|fmt_join("
            ColJsonArray_::{var}(v) => {for v in v { query = query.bind(v{bind_as_for_filter}); } query},", "") }@
            _ => unreachable!(),
        }
    }
}
@%- if config.excluded_from_domain %@

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub@{ visibility }@ enum ColGeo_ {
@{- def.all_fields_only_geo()|fmt_join("
    {var}(Value, u32),", "") }@
}
@%- else %@
pub(crate) use domain::models::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::_base::_@{ mod_name }@::ColGeo_;
@%- endif %@
#[allow(clippy::match_single_binding)]
impl BindTr for ColGeo_ {
    fn name(&self) -> &'static str {
        match self {
@{- def.all_fields_only_geo()|fmt_join("
            ColGeo_::{var}(_, _) => r#\"{col_esc}\"#,", "") }@
            _ => unreachable!(),
        }
    }
    fn query_as_bind<T>(
        self,
        query: QueryAs<'_, DbType, T, DbArguments>,
    ) -> QueryAs<'_, DbType, T, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
@{- def.all_fields_only_geo()|fmt_join("
            ColGeo_::{var}(v, srid) => query.bind(v{bind_as_for_filter}).bind(srid),", "") }@
            _ => unreachable!(),
        }
    }
    fn query_bind(
        self,
        query: Query<'_, DbType, DbArguments>,
    ) -> Query<'_, DbType, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
@{- def.all_fields_only_geo()|fmt_join("
            ColGeo_::{var}(v, srid) => query.bind(v{bind_as_for_filter}).bind(srid),", "") }@
            _ => unreachable!(),
        }
    }
}
@%- if config.excluded_from_domain %@

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub@{ visibility }@ enum ColGeoDistance_ {
@{- def.all_fields_only_geo()|fmt_join("
    {var}(Value, f64, u32),", "") }@
}
@%- else %@
pub(crate) use domain::models::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::_base::_@{ mod_name }@::ColGeoDistance_;
@%- endif %@
#[allow(clippy::match_single_binding)]
impl BindTr for ColGeoDistance_ {
    fn name(&self) -> &'static str {
        match self {
@{- def.all_fields_only_geo()|fmt_join("
            ColGeoDistance_::{var}(_, _, _) => r#\"{col_esc}\"#,", "") }@
            _ => unreachable!(),
        }
    }
    fn query_as_bind<T>(
        self,
        query: QueryAs<'_, DbType, T, DbArguments>,
    ) -> QueryAs<'_, DbType, T, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
@{- def.all_fields_only_geo()|fmt_join("
            ColGeoDistance_::{var}(v, d, srid) => query.bind(v.clone(){bind_as_for_filter}).bind(srid).bind(d).bind(v{bind_as_for_filter}).bind(srid).bind(d),", "") }@
            _ => unreachable!(),
        }
    }
    fn query_bind(
        self,
        query: Query<'_, DbType, DbArguments>,
    ) -> Query<'_, DbType, DbArguments> {
        debug!("bind: {:?}", &self);
        match self {
@{- def.all_fields_only_geo()|fmt_join("
            ColGeoDistance_::{var}(v, d, srid) => query.bind(v.clone(){bind_as_for_filter}).bind(srid).bind(d).bind(v{bind_as_for_filter}).bind(srid).bind(d),", "") }@
            _ => unreachable!(),
        }
    }
}
@%- if config.excluded_from_domain %@

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub@{ visibility }@ enum ColRel_ {
@{- def.relations_one_and_belonging(false)|fmt_rel_join("\n    {rel_name}(Option<Box<rel_{class_mod}::Filter_>>),", "") }@
@{- def.relations_many(false)|fmt_rel_join("\n    {rel_name}(Option<Box<rel_{class_mod}::Filter_>>),", "") }@
}
@%- else %@
pub(crate) use domain::models::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::_base::_@{ mod_name }@::ColRel_;
@%- endif %@
impl ColRelTr for ColRel_ {
    #[allow(unused_mut)]
    #[allow(clippy::ptr_arg)]
    fn write_rel(&self, buf: &mut String, idx: usize, without_key: bool) {
@%- if def.relations_one_and_belonging(false).len() + def.relations_many(false).len() > 0 %@
        match self {
@{- def.relations_belonging(false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => {
                if without_key {
                    write!(buf, r#\"SELECT {} FROM {table} as _t{} WHERE \"#, rel_{class_mod}::Primary::cols(), idx + 1).unwrap();
                } else {
                    write!(buf, r#\"SELECT * FROM {table} as _t{} WHERE {}={} AND \"#, idx + 1, rel_{class_mod}::Primary::cols_with_paren(), RelCol{rel_name_pascal}::cols_with_idx(idx)).unwrap();
                }
                let mut trash_mode = TrashMode::Not;
                if let Some(filter) = c {
                    filter.write(buf, idx + 1, &mut trash_mode);
                }
                if trash_mode == TrashMode::Not {
                    buf.push_str(rel_{class_mod}::NOT_TRASHED_SQL)
                } else if trash_mode == TrashMode::Only {
                    buf.push_str(rel_{class_mod}::ONLY_TRASHED_SQL)
                } else {
                    buf.push_str(rel_{class_mod}::TRASHED_SQL)
                }
                if buf.ends_with(\" AND \") {
                    buf.truncate(buf.len() - \" AND \".len());
                }
                if without_key && buf.ends_with(\" WHERE \") {
                    buf.truncate(buf.len() - \" WHERE \".len());
                }
            }", "") }@
@{- def.relations_one(false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => {
                if without_key {
                    write!(buf, r#\"SELECT {} FROM {table} as _t{} WHERE \"#, RelCol{rel_name_pascal}::cols(), idx + 1).unwrap();
                } else {
                    write!(buf, r#\"SELECT * FROM {table} as _t{} WHERE {}={} AND \"#, idx + 1, Primary::cols_with_idx(idx), RelCol{rel_name_pascal}::cols_with_paren()).unwrap();
                }
                let mut trash_mode = TrashMode::Not;
                if let Some(filter) = c {
                    filter.write(buf, idx + 1, &mut trash_mode);
                }
                if trash_mode == TrashMode::Not {
                    buf.push_str(rel_{class_mod}::NOT_TRASHED_SQL)
                } else if trash_mode == TrashMode::Only {
                    buf.push_str(rel_{class_mod}::ONLY_TRASHED_SQL)
                } else {
                    buf.push_str(rel_{class_mod}::TRASHED_SQL)
                }
                if buf.ends_with(\" AND \") {
                    buf.truncate(buf.len() - \" AND \".len());
                }
                if without_key && buf.ends_with(\" WHERE \") {
                    buf.truncate(buf.len() - \" WHERE \".len());
                }
            }", "") }@
@{- def.relations_many(false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => {
                if without_key {
                    write!(buf, r#\"SELECT {} FROM {table} as _t{} WHERE \"#, RelCol{rel_name_pascal}::cols(), idx + 1).unwrap();
                } else {
                    write!(buf, r#\"SELECT * FROM {table} as _t{} WHERE {}={} AND \"#, idx + 1, Primary::cols_with_idx(idx), RelCol{rel_name_pascal}::cols_with_paren()).unwrap();
                }
                let mut trash_mode = TrashMode::Not;
                if let Some(filter) = c {
                    filter.write(buf, idx + 1, &mut trash_mode);
                }
                if trash_mode == TrashMode::Not {
                    buf.push_str(rel_{class_mod}::NOT_TRASHED_SQL)
                } else if trash_mode == TrashMode::Only {
                    buf.push_str(rel_{class_mod}::ONLY_TRASHED_SQL)
                } else {
                    buf.push_str(rel_{class_mod}::TRASHED_SQL)
                }
                if buf.ends_with(\" AND \") {
                    buf.truncate(buf.len() - \" AND \".len());
                }
                if without_key && buf.ends_with(\" WHERE \") {
                    buf.truncate(buf.len() - \" WHERE \".len());
                }
            }", "") }@
        };
@%- endif %@
    }
    #[allow(unused_mut)]
    #[allow(clippy::ptr_arg)]
    fn write_key(&self, buf: &mut String) {
@%- if def.relations_one_and_belonging(false).len() + def.relations_many(false).len() > 0 %@
        match self {
@{- def.relations_belonging(false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => {
                buf.push_str(RelCol{rel_name_pascal}::cols());
            }", "") }@
@{- def.relations_one(false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => {
                buf.push_str(Primary::cols());
            }", "") }@
@{- def.relations_many(false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => {
                buf.push_str(Primary::cols());
            }", "") }@
        };
@%- endif %@
    }
    fn query_as_bind<T>(
        self,
        query: QueryAs<'_, DbType, T, DbArguments>,
    ) -> QueryAs<'_, DbType, T, DbArguments> {
@%- if def.relations_one_and_belonging(false).len() + def.relations_many(false).len() > 0 %@
        match self {
@{- def.relations_one_and_belonging(false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => {
                if let Some(filter) = c {
                    filter.query_as_bind(query)
                } else {
                    query
                }
            }", "") }@
@{- def.relations_many(false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => {
                if let Some(filter) = c {
                    filter.query_as_bind(query)
                } else {
                    query
                }
            }", "") }@
        }
@%- else %@
        query
@%- endif %@
    }
    fn query_bind(
        self,
        query: Query<'_, DbType, DbArguments>,
    ) -> Query<'_, DbType, DbArguments> {
@%- if def.relations_one_and_belonging(false).len() + def.relations_many(false).len() > 0 %@
        match self {
@{- def.relations_one_and_belonging(false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => {
                if let Some(filter) = c {
                    filter.query_bind(query)
                } else {
                    query
                }
            }", "") }@
@{- def.relations_many(false)|fmt_rel_join("
            ColRel_::{rel_name}(c) => {
                if let Some(filter) = c {
                    filter.query_bind(query)
                } else {
                    query
                }
            }", "") }@
        }
@%- else %@
        query
@%- endif %@
    }
}
@%- if config.excluded_from_domain %@

#[derive(Clone, Debug)]
pub@{ visibility }@ enum Filter_ {
    WithTrashed,
    OnlyTrashed,
    Match(Vec<Col_>, String),
    MatchBoolean(Vec<Col_>, String),
    MatchExpansion(Vec<Col_>, String),
    IsNull(Col_),
    IsNotNull(Col_),
    Eq(ColOne_),
    EqKey(ColKey_),
    NotEq(ColOne_),
    Gt(ColOne_),
    Gte(ColOne_),
    Lt(ColOne_),
    Lte(ColOne_),
    Like(ColOne_),
    AllBits(ColMany_),
    AnyBits(ColOne_),
    In(ColMany_),
    NotIn(ColMany_),
    MemberOf(ColJson_, Option<String>),
    Contains(ColJsonArray_, Option<String>),
    Overlaps(ColJsonArray_, Option<String>),
    JsonIn(ColJsonArray_, String),
    JsonContainsPath(ColJson_, String),
    JsonEq(ColJson_, String),
    JsonLt(ColJson_, String),
    JsonLte(ColJson_, String),
    JsonGt(ColJson_, String),
    JsonGte(ColJson_, String),
    Within(ColGeo_),
    Intersects(ColGeo_),
    Crosses(ColGeo_),
    DWithin(ColGeoDistance_),
    Not(Box<Filter_>),
    And(Vec<Filter_>),
    Or(Vec<Filter_>),
    Exists(ColRel_),
    NotExists(ColRel_),
    EqAny(ColRel_),
    NotAll(ColRel_),
    Raw(String),
    RawWithParam(String, Vec<String>),
    Boolean(bool),
}
impl Filter_ {
    pub@{ visibility }@ fn new_and() -> Filter_ {
        Filter_::And(vec![])
    }
    pub@{ visibility }@ fn new_or() -> Filter_ {
        Filter_::Or(vec![])
    }
    pub@{ visibility }@ fn and(mut self, filter: Filter_) -> Filter_ {
        match self {
            Filter_::And(ref mut v) => {
                v.push(filter);
                self
            },
            _ => Filter_::And(vec![self, filter]),
        }
    }
    pub@{ visibility }@ fn or(mut self, filter: Filter_) -> Filter_ {
        match self {
            Filter_::Or(ref mut v) => {
                v.push(filter);
                self
            },
            Filter_::And(ref v) if v.is_empty() => {
                Filter_::Or(vec![filter])
            },
            _ => Filter_::Or(vec![self, filter]),
        }
    }
    pub@{ visibility }@ fn when<F>(self, condition: bool, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        if condition {
            f(self)
        } else {
            self
        }
    }
    pub@{ visibility }@ fn if_let_some<T, F>(self, value: &Option<T>, f: F) -> Self
    where
        F: FnOnce(Self, &T) -> Self,
    {
        if let Some(v) = value {
            f(self, v)
        } else {
            self
        }
    }
}
@%- else %@
pub(crate) use domain::models::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::_base::_@{ mod_name }@::Filter_;
@%- endif %@
impl FilterTr for Filter_ {
    crate::misc::filter!(Data);
}
@% let filter_macro_name = "filter_{}_{}"|format(group_name, model_name) -%@
@% let model_path = "$crate::models::{}::_base::_{}"|format(group_name|to_var_name, mod_name) -%@
@%- if config.excluded_from_domain %@

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ filter_macro_name }@_null {
@%- for (col_name, column_def) in def.nullable() %@
    (@{ col_name }@) => (@{ model_path }@::Col_::@{ col_name|to_var_name }@);
@%- endfor %@
    () => (); // For empty case
}
pub@{ visibility }@ use @{ filter_macro_name }@_null as filter_null;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ filter_macro_name }@_text {
@%- for (col_name, column_def) in def.text() %@
    (@{ col_name }@) => (@{ model_path }@::Col_::@{ col_name|to_var_name }@);
@%- endfor %@
    () => (); // For empty case
}
pub@{ visibility }@ use @{ filter_macro_name }@_text as filter_text;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ filter_macro_name }@_one {
@%- for (col_name, column_def) in def.all_fields_without_json() %@
    (@{ col_name }@ $e:expr) => (@{ model_path }@::ColOne_::@{ col_name|to_var_name }@($e.clone().try_into()?));
@%- endfor %@
}
pub@{ visibility }@ use @{ filter_macro_name }@_one as filter_one;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ filter_macro_name }@_many {
@%- for (col_name, column_def) in def.all_fields_without_json() %@
    (@{ col_name }@ [$($e:expr),*]) => (@{ model_path }@::ColMany_::@{ col_name|to_var_name }@(vec![ $( $e.clone().try_into()? ),* ]));
    (@{ col_name }@ $e:expr) => (@{ model_path }@::ColMany_::@{ col_name|to_var_name }@($e.into_iter().map(|v| v.clone().try_into()).collect::<Result<Vec<_>, _>>()?));
@%- endfor %@
}
pub@{ visibility }@ use @{ filter_macro_name }@_many as filter_many;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ filter_macro_name }@_json {
@%- for (col_name, column_def) in def.all_fields_only_json() %@
    (@{ col_name }@ $e:expr) => (@{ model_path }@::ColJson_::@{ col_name|to_var_name }@($e.clone().try_into()?));
@%- endfor %@
    () => ();
}
pub@{ visibility }@ use @{ filter_macro_name }@_json as filter_json;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ filter_macro_name }@_json_array {
@%- for (col_name, column_def) in def.all_fields_only_json() %@
    (@{ col_name }@ $e:expr) => (@{ model_path }@::ColJsonArray_::@{ col_name|to_var_name }@($e.iter().map(|v| v.clone().try_into()).collect::<Result<Vec<_>, _>>()?));
@%- endfor %@
    () => ();
}
pub@{ visibility }@ use @{ filter_macro_name }@_json_array as filter_json_array;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ filter_macro_name }@_geo {
@%- for (col_name, column_def) in def.all_fields_only_geo() %@
    (@{ col_name }@ $e:expr, $s:expr) => (@{ model_path }@::ColGeo_::@{ col_name|to_var_name }@($e.clone().try_into()?, $s));
    (@{ col_name }@ $e:expr) => (@{ model_path }@::ColGeo_::@{ col_name|to_var_name }@($e.clone().try_into()?, @{ column_def.srid() }@));
@%- endfor %@
    () => ();
}
pub@{ visibility }@ use @{ filter_macro_name }@_geo as filter_geo;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ filter_macro_name }@_geo_distance {
@%- for (col_name, column_def) in def.all_fields_only_geo() %@
    (@{ col_name }@ $e:expr, $d:expr, $s:expr) => (@{ model_path }@::ColGeoDistance_::@{ col_name|to_var_name }@($e.clone().try_into()?, $d, $s));
    (@{ col_name }@ $e:expr, $d:expr) => (@{ model_path }@::ColGeoDistance_::@{ col_name|to_var_name }@($e.clone().try_into()?, $d, @{ column_def.srid() }@));
@%- endfor %@
    () => ();
}
pub@{ visibility }@ use @{ filter_macro_name }@_geo_distance as filter_geo_distance;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ filter_macro_name }@_rel {
@%- for (model_def, col_name, rel_def) in def.relations_one_and_belonging(false) %@
    (@{ col_name }@) => (@{ model_path }@::ColRel_::@{ col_name|to_var_name }@(None));
    (@{ col_name }@ $t:tt) => (@{ model_path }@::ColRel_::@{ col_name|to_var_name }@(Some(Box::new($crate::models::@{ rel_def.get_group_name()|snake|to_var_name }@::_base::_@{ rel_def.get_mod_name() }@::filter!($t)))));
@%- endfor %@
@%- for (model_def, col_name, rel_def) in def.relations_many(false) %@
    (@{ col_name }@) => (@{ model_path }@::ColRel_::@{ col_name|to_var_name }@(None));
    (@{ col_name }@ $t:tt) => (@{ model_path }@::ColRel_::@{ col_name|to_var_name }@(Some(Box::new($crate::models::@{ rel_def.get_group_name()|snake|to_var_name }@::_base::_@{ rel_def.get_mod_name() }@::filter!($t)))));
@%- endfor %@
    () => ();
}
pub@{ visibility }@ use @{ filter_macro_name }@_rel as filter_rel;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ filter_macro_name }@ {
    () => (@{ model_path }@::Filter_::new_and());
@%- for (index_name, index) in def.multi_index() %@
    ((@{ index.join_fields(def, "{name}", ", ") }@) = (@{ index.join_fields(def, "$e{index}:expr", ", ") }@)) => (@{ model_path }@::Filter_::Eq(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "$e{index}.clone().try_into()?", ", ") }@)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) > (@{ index.join_fields(def, "$e{index}:expr", ", ") }@)) => (@{ model_path }@::Filter_::Gt(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "$e{index}.clone().try_into()?", ", ") }@)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) >= (@{ index.join_fields(def, "$e{index}:expr", ", ") }@)) => (@{ model_path }@::Filter_::Gte(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "$e{index}.clone().try_into()?", ", ") }@)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) < (@{ index.join_fields(def, "$e{index}:expr", ", ") }@)) => (@{ model_path }@::Filter_::Lt(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "$e{index}.clone().try_into()?", ", ") }@)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) <= (@{ index.join_fields(def, "$e{index}:expr", ", ") }@)) => (@{ model_path }@::Filter_::Lte(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "$e{index}.clone().try_into()?", ", ") }@)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) = $e:expr) => (@{ model_path }@::Filter_::Eq(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "$e.{index}.clone().try_into()?", ", ") }@)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) IN $e:expr) => (@{ model_path }@::Filter_::In(@{ model_path }@::ColMany_::@{ index.join_fields(def, "{name}", "_") }@($e.into_iter().map(|v| (@{ index.join_fields(def, "v.{index}.clone()", ", ") }@).try_into()).collect::<Result<_, _>>()?)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) NOT IN $e:expr) => (@{ model_path }@::Filter_::NotIn(@{ model_path }@::ColMany_::@{ index.join_fields(def, "{name}", "_") }@($e.into_iter().map(|v| (@{ index.join_fields(def, "v.{index}.clone()", ", ") }@).try_into()).collect::<Result<_, _>>()?)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) > $e:expr) => (@{ model_path }@::Filter_::Gt(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "$e.{index}.clone().try_into()?", ", ") }@)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) >= $e:expr) => (@{ model_path }@::Filter_::Gte(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "$e.{index}.clone().try_into()?", ", ") }@)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) < $e:expr) => (@{ model_path }@::Filter_::Lt(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "$e.{index}.clone().try_into()?", ", ") }@)));
    ((@{ index.join_fields(def, "{name}", ", ") }@) <= $e:expr) => (@{ model_path }@::Filter_::Lte(@{ model_path }@::ColOne_::@{ index.join_fields(def, "{name}", "_") }@(@{ index.join_fields(def, "$e.{index}.clone().try_into()?", ", ") }@)));
@%- endfor %@
    (($($t:tt)*)) => (@{ model_path }@::filter!($($t)*));
    (NOT $t:tt) => (@{ model_path }@::Filter_::Not(Box::new(@{ model_path }@::filter!($t))));
    (WITH_TRASHED) => (@{ model_path }@::Filter_::WithTrashed);
    (ONLY_TRASHED) => (@{ model_path }@::Filter_::OnlyTrashed);
    (BOOLEAN $e:expr) => (@{ model_path }@::Filter_::Boolean($e));
    (RAW $e:expr) => (@{ model_path }@::Filter_::Raw($e.to_string()));
    (RAW $e:expr , [$($p:expr),*] ) => (@{ model_path }@::Filter_::RawWithParam($e.to_string(), vec![ $( $p.to_string() ),* ]));
    (RAW $e:expr , $p:expr ) => (@{ model_path }@::Filter_::RawWithParam($e.to_string(), $p.iter().map(|v| v.to_string()).collect()));
    (MATCH ( $($i:ident),+ ) AGAINST ($e:expr) IN BOOLEAN MODE) => (@{ model_path }@::Filter_::MatchBoolean(vec![ $( @{ model_path }@::filter_text!($i) ),* ], $e.to_string()));
    (MATCH ( $($i:ident),+ ) AGAINST ($e:expr) WITH QUERY EXPANSION) => (@{ model_path }@::Filter_::MatchExpansion(vec![ $( @{ model_path }@::filter_text!($i) ),* ], $e.to_string()));
    (MATCH ( $($i:ident),+ ) AGAINST ($e:expr)) => (@{ model_path }@::Filter_::Match(vec![ $( @{ model_path }@::filter_text!($i) ),* ], $e.to_string()));
    ($i:ident EXISTS) => (@{ model_path }@::Filter_::Exists(@{ model_path }@::filter_rel!($i)));
    ($i:ident EXISTS $t:tt) => (@{ model_path }@::Filter_::Exists(@{ model_path }@::filter_rel!($i $t)));
    ($i:ident NOT EXISTS) => (@{ model_path }@::Filter_::NotExists(@{ model_path }@::filter_rel!($i)));
    ($i:ident NOT EXISTS $t:tt) => (@{ model_path }@::Filter_::NotExists(@{ model_path }@::filter_rel!($i $t)));
    ($i:ident = ANY $t:tt) => (@{ model_path }@::Filter_::EqAny(@{ model_path }@::filter_rel!($i $t)));
    ($i:ident NOT ALL $t:tt) => (@{ model_path }@::Filter_::NotAll(@{ model_path }@::filter_rel!($i $t)));
    ($i:ident IS NULL) => (@{ model_path }@::Filter_::IsNull(@{ model_path }@::filter_null!($i)));
    ($i:ident IS NOT NULL) => (@{ model_path }@::Filter_::IsNotNull(@{ model_path }@::filter_null!($i)));
    ($i:ident = $e:expr) => (@{ model_path }@::Filter_::Eq(@{ model_path }@::filter_one!($i $e)));
    ($i:ident != $e:expr) => (@{ model_path }@::Filter_::NotEq(@{ model_path }@::filter_one!($i $e)));
    ($i:ident > $e:expr) => (@{ model_path }@::Filter_::Gt(@{ model_path }@::filter_one!($i $e)));
    ($i:ident >= $e:expr) => (@{ model_path }@::Filter_::Gte(@{ model_path }@::filter_one!($i $e)));
    ($i:ident < $e:expr) => (@{ model_path }@::Filter_::Lt(@{ model_path }@::filter_one!($i $e)));
    ($i:ident <= $e:expr) => (@{ model_path }@::Filter_::Lte(@{ model_path }@::filter_one!($i $e)));
    ($i:ident LIKE $e:expr) => (@{ model_path }@::Filter_::Like(@{ model_path }@::filter_one!($i $e)));
    ($i:ident ALL_BITS $e:expr) => (@{ model_path }@::Filter_::AllBits(@{ model_path }@::filter_many!($i [$e, $e])));
    ($i:ident ANY_BITS $e:expr) => (@{ model_path }@::Filter_::AnyBits(@{ model_path }@::filter_one!($i $e)));
    ($i:ident BETWEEN ($e1:expr, $e2:expr)) => (@{ model_path }@::filter!(($i >= $e1) AND ($i <= $e2)));
    ($i:ident RIGHT_OPEN ($e1:expr, $e2:expr)) => (@{ model_path }@::filter!(($i >= $e1) AND ($i < $e2)));
    ($i:ident IN ( $($e:expr),* )) => (@{ model_path }@::Filter_::In(@{ model_path }@::filter_many!($i [ $( $e ),* ])));
    ($i:ident IN $e:expr) => (@{ model_path }@::Filter_::In(@{ model_path }@::filter_many!($i $e)));
    ($i:ident NOT IN ( $($e:expr),* )) => (@{ model_path }@::Filter_::NotIn(@{ model_path }@::filter_many!($i [ $( $e ),* ])));
    ($i:ident NOT IN $e:expr) => (@{ model_path }@::Filter_::NotIn(@{ model_path }@::filter_many!($i $e)));
    ($i:ident HAS $e:expr) => (@{ model_path }@::Filter_::MemberOf(@{ model_path }@::filter_json!($i $e), None));
    ($i:ident -> ($p:expr) HAS $e:expr) => (@{ model_path }@::Filter_::MemberOf(@{ model_path }@::filter_json!($i $e), Some($p.to_string())));
    ($i:ident CONTAINS [ $($e:expr),* ]) => (@{ model_path }@::Filter_::Contains(@{ model_path }@::filter_json_array!($i vec![ $( $e ),* ]), None));
    ($i:ident CONTAINS $e:expr) => (@{ model_path }@::Filter_::Contains(@{ model_path }@::filter_json_array!($i $e), None));
    ($i:ident -> ($p:expr) CONTAINS [ $($e:expr),* ]) => (@{ model_path }@::Filter_::Contains(@{ model_path }@::filter_json_array!($i vec![ $( $e ),* ]), Some($p.to_string())));
    ($i:ident -> ($p:expr) CONTAINS $e:expr) => (@{ model_path }@::Filter_::Contains(@{ model_path }@::filter_json_array!($i $e), Some($p.to_string())));
    ($i:ident OVERLAPS [ $($e:expr),* ]) => (@{ model_path }@::Filter_::Overlaps(@{ model_path }@::filter_json_array!($i vec![ $( $e ),* ]), None));
    ($i:ident OVERLAPS $e:expr) => (@{ model_path }@::Filter_::Overlaps(@{ model_path }@::filter_json_array!($i $e), None));
    ($i:ident -> ($p:expr) OVERLAPS [ $($e:expr),* ]) => (@{ model_path }@::Filter_::Overlaps(@{ model_path }@::filter_json_array!($i vec![ $( $e ),* ]), Some($p.to_string())));
    ($i:ident -> ($p:expr) OVERLAPS $e:expr) => (@{ model_path }@::Filter_::Overlaps(@{ model_path }@::filter_json_array!($i $e), Some($p.to_string())));
    ($i:ident -> ($p:expr) IN [ $($e:expr),* ]) => (@{ model_path }@::Filter_::JsonIn(@{ model_path }@::filter_json_array!($i vec![ $( $e ),* ]), Some($p.to_string())));
    ($i:ident -> ($p:expr) IN $e:expr) => (@{ model_path }@::Filter_::JsonIn(@{ model_path }@::filter_json_array!($i $e), Some($p.to_string())));
    ($i:ident JSON_CONTAINS_PATH ($p:expr)) => (@{ model_path }@::Filter_::JsonContainsPath(@{ model_path }@::filter_json!($i 0), $p.to_string()));
    ($i:ident -> ($p:expr) = $e:expr) => (@{ model_path }@::Filter_::JsonEq(@{ model_path }@::filter_json!($i $e), $p.to_string()));
    ($i:ident -> ($p:expr) < $e:expr) => (@{ model_path }@::Filter_::JsonLt(@{ model_path }@::filter_json!($i $e), $p.to_string()));
    ($i:ident -> ($p:expr) <= $e:expr) => (@{ model_path }@::Filter_::JsonLte(@{ model_path }@::filter_json!($i $e), $p.to_string()));
    ($i:ident -> ($p:expr) > $e:expr) => (@{ model_path }@::Filter_::JsonGt(@{ model_path }@::filter_json!($i $e), $p.to_string()));
    ($i:ident -> ($p:expr) >= $e:expr) => (@{ model_path }@::Filter_::JsonGte(@{ model_path }@::filter_json!($i $e), $p.to_string()));
    ($i:ident WITHIN_WITH_SRID $e:expr, $s:expr) => (@{ model_path }@::Filter_::Within(@{ model_path }@::filter_geo!($i $e, $s)));
    ($i:ident WITHIN $e:expr) => (@{ model_path }@::Filter_::Within(@{ model_path }@::filter_geo!($i $e)));
    ($i:ident INTERSECTS_WITH_SRID $e:expr, $s:expr) => (@{ model_path }@::Filter_::Intersects(@{ model_path }@::filter_geo!($i $e, $s)));
    ($i:ident INTERSECTS $e:expr) => (@{ model_path }@::Filter_::Intersects(@{ model_path }@::filter_geo!($i $e)));
    ($i:ident CROSSES_WITH_SRID $e:expr, $s:expr) => (@{ model_path }@::Filter_::Crosses(@{ model_path }@::filter_geo!($i $e, $s)));
    ($i:ident CROSSES $e:expr) => (@{ model_path }@::Filter_::Crosses(@{ model_path }@::filter_geo!($i $e)));
    ($i:ident D_WITHIN_WITH_SRID $e:expr, $d:expr, $s:expr) => (@{ model_path }@::Filter_::DWithin(@{ model_path }@::filter_geo_distance!($i $e, $d, $s)));
    ($i:ident D_WITHIN $e:expr, $d:expr) => (@{ model_path }@::Filter_::DWithin(@{ model_path }@::filter_geo_distance!($i $e, $d)));
    ($t1:tt AND $($t2:tt)AND+) => (@{ model_path }@::Filter_::And(vec![ @{ model_path }@::filter!($t1), $( @{ model_path }@::filter!($t2) ),* ]));
    ($t1:tt OR $($t2:tt)OR+) => (@{ model_path }@::Filter_::Or(vec![ @{ model_path }@::filter!($t1), $( @{ model_path }@::filter!($t2) ),* ]));
}
pub@{ visibility }@ use @{ filter_macro_name }@ as filter;
@%- endif %@
@%- if config.excluded_from_domain %@

#[derive(Clone, Debug)]
pub@{ visibility }@ enum Order_ {
    Asc(Col_),
    Desc(Col_),
    IsNullAsc(Col_),
    IsNullDesc(Col_),
}
@%- else %@
pub(crate) use domain::models::@{ db|snake|to_var_name }@::@{ group_name|to_var_name }@::_base::_@{ mod_name }@::Order_;
@%- endif %@
impl OrderTr for Order_ {
    crate::misc::order!();
}
@%- if config.excluded_from_domain %@

@% let order_macro_name = "order_{}_{}"|format(group_name, model_name) -%@
@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ order_macro_name }@_col {
@%- for (col_name, column_def) in def.all_fields() %@
    (@{ col_name }@) => (@{ model_path }@::Col_::@{ col_name|to_var_name }@);
@%- endfor %@
}
pub@{ visibility }@ use @{ order_macro_name }@_col as order_by_col;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ order_macro_name }@_one {
    ($i:ident) => (@{ model_path }@::Order_::Asc(@{ model_path }@::order_by_col!($i)));
    ($i:ident ASC) => (@{ model_path }@::Order_::Asc(@{ model_path }@::order_by_col!($i)));
    ($i:ident DESC) => (@{ model_path }@::Order_::Desc(@{ model_path }@::order_by_col!($i)));
    ($i:ident IS NULL ASC) => (@{ model_path }@::Order_::IsNullAsc(@{ model_path }@::order_by_col!($i)));
    ($i:ident IS NULL DESC) => (@{ model_path }@::Order_::IsNullDesc(@{ model_path }@::order_by_col!($i)));
}
pub@{ visibility }@ use @{ order_macro_name }@_one as order_by_one;

@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! @{ order_macro_name }@ {
    ($($($i:ident)+),+) => (vec![$( @{ model_path }@::order_by_one!($($i)+)),+]);
}
pub@{ visibility }@ use @{ order_macro_name }@ as order;
@%- endif %@

#[derive(Default, sqlx::FromRow, senax_macros::SqlCol)]
struct Count {
    #[sql(query = "count(*)")]
    c: i64,
}
@%- if config.excluded_from_domain %@

#[derive(Debug, Clone, Default)]
pub@{ visibility }@ struct Joiner_ {
@{- def.relations()|fmt_rel_join("
    pub {rel_name}: Option<Box<join_{class_mod}::Joiner_>>,", "") }@
    pub _dummy: bool,
}
impl Joiner_ {
    #[allow(clippy::nonminimal_bool)]
    pub fn has_some(&self) -> bool {
        false
        @{- def.relations()|fmt_rel_join("
            || self.{rel_name}.is_some()", "") }@
    }
    pub fn merge(lhs: Option<Box<Self>>, rhs: Option<Box<Self>>) -> Option<Box<Self>> {
        if let Some(lhs) = lhs {
            if let Some(rhs) = rhs {
                Some(Box::new(Joiner_{
                    @{- def.relations()|fmt_rel_join("
                    {rel_name}: _model_::{class_mod_var}::Joiner_::merge(lhs.{rel_name}, rhs.{rel_name}),", "") }@
                    ..Default::default()
                }))
            } else {
                Some(lhs)
            }
        } else {
            rhs
        }
    }
}
@%- let fetch_macro_name = "{}_{}"|format(group_name, model_name) %@
@%- let base_path = "$crate::models::{}::{}::_base::_{}"|format(db|snake|to_var_name, group_name|to_var_name, mod_name) %@
@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! _join_@{ fetch_macro_name }@ {
@{- def.relations()|fmt_rel_join("
    ({rel_name}) => ($crate::models::{class_mod_var}::_{mod_name}::join!({}));
    ({rel_name}: $p:tt) => ($crate::models::{class_mod_var}::_{mod_name}::join!($p));", "") }@
    () => ();
}
pub@{ visibility }@ use _join_@{ fetch_macro_name }@ as _join;
@% if config.export_db_layer %@#[macro_export]@% else %@#[allow(unused_macros)]@% endif %@
macro_rules! join_@{ fetch_macro_name }@ {
    ({$($i:ident $(: $p:tt)?),*}) => (Some(Box::new(@{ model_path }@::Joiner_ {
        $($i: @{ base_path }@::_join!($i $(: $p)?),)*
        ..Default::default()
    })));
}
pub@{ visibility }@ use join_@{ fetch_macro_name }@ as join;
@%- endif %@

#[derive(Debug, Clone, Default)]
pub@{ visibility }@ struct QueryBuilder {
    filter: Option<Filter_>,
    order: Option<Vec<Order_>>,
    limit: Option<usize>,
    offset: Option<usize>,
    skip_locked: bool,
    trash_mode: TrashMode,
    raw_query: String,
    bind: Vec<BindValue>,
    joiner: Option<Box<Joiner_>>,
}

impl QueryBuilder {
    pub@{ visibility }@ fn filter(mut self, filter: Filter_) -> Self {
        self.filter = Some(filter);
        self
    }
    pub@{ visibility }@ fn order_by(mut self, order: Vec<Order_>) -> Self {
        self.order = Some(order);
        self
    }
    pub@{ visibility }@ fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
    pub@{ visibility }@ fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
    pub@{ visibility }@ fn skip_locked(mut self) -> Self {
        self.skip_locked = true;
        self
    }
    @%- if def.is_soft_delete() %@
    pub@{ visibility }@ fn with_trashed(mut self) -> Self {
        self.trash_mode = TrashMode::With;
        self
    }
    pub@{ visibility }@ fn only_trashed(mut self) -> Self {
        self.trash_mode = TrashMode::Only;
        self
    }
    @%- endif %@
    pub@{ visibility }@ fn append_raw_query(mut self, query: &str) -> Self {
        self.raw_query.push_str(query);
        self
    }
    /// bind for raw_query
    pub@{ visibility }@ fn bind<T: Into<BindValue>>(mut self, value: T) -> Self {
        self.bind.push(value.into());
        self
    }
    pub@{ visibility }@ fn when<F>(self, condition: bool, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        if condition {
            f(self)
        } else {
            self
        }
    }
    pub@{ visibility }@ fn if_let_some<T, F>(self, value: &Option<T>, f: F) -> Self
    where
        F: FnOnce(Self, &T) -> Self,
    {
        if let Some(v) = value {
            f(self, v)
        } else {
            self
        }
    }
    pub fn join(mut self, joiner: Option<Box<Joiner_>>) -> Self {
        self.joiner = Joiner_::merge(self.joiner, joiner);
        self
    }
    async fn _select<T>(self, conn: &mut DbConn) -> Result<Vec<T>>
    where
        T: for<'r> sqlx::FromRow<'r, <DbType as sqlx::Database>::Row> + SqlColumns + Send + Sync + Unpin,
    {
        let sql = self._sql(T::_sql_cols(), false);
        let mut query = sqlx::query_as::<_, T>(&sql);
        let _span = debug_span!("query", sql = &query.sql());
        query = self._bind(query);
        let result = crate::misc::fetch!(conn, query, fetch_all);
        Ok(result)
    }

    fn _sql(&self, sql_cols: &str, for_update: bool) -> String {
        let mut sql = format!(
            r#"SELECT {} FROM @{ table_name|db_esc }@ as _t1 {} {} {}"#,
            sql_cols,
            Filter_::write_where(
                &self.filter,
                self.trash_mode,
                TRASHED_SQL,
                NOT_TRASHED_SQL,
                ONLY_TRASHED_SQL
            ),
            &self.raw_query,
            Order_::write_order(&self.order),
        );
        if let Some(limit) = self.limit {
            write!(sql, " limit {}", limit).unwrap();
        }
        if let Some(offset) = self.offset {
            write!(sql, " offset {}", offset).unwrap();
        }
        if for_update {
            if self.skip_locked {
                write!(sql, " FOR UPDATE SKIP LOCKED").unwrap();
            } else {
                write!(sql, " FOR UPDATE").unwrap();
            }
        }
        sql
    }

    fn _bind<T>(self, mut query: QueryAs<DbType, T, DbArguments>) -> QueryAs<DbType, T, DbArguments> {
        if let Some(c) = self.filter {
            debug!("filter: {:?}", &c);
            query = c.query_as_bind(query);
        }
        for value in self.bind.into_iter() {
            debug!("bind: {:?}", &value);
            query = match value {
                BindValue::Bool(v) => query.bind(v),
                BindValue::Enum(v) => query.bind(v),
                BindValue::Number(v) => query.bind(v),
                BindValue::String(v) => query.bind(v),
                BindValue::DateTime(v) => query.bind(v),
                BindValue::Date(v) => query.bind(v),
                BindValue::Time(v) => query.bind(v),
                BindValue::Blob(v) => query.bind(v),
                BindValue::Json(v) => query.bind(v),
                BindValue::Uuid(v) => query.bind(v),
                BindValue::BinaryUuid(v) => query.bind(v),
            };
        }
        query
    }

    async fn _select_stream<'a, T: 'a>(self, conn: &mut DbConn) -> Result<mpsc::Receiver<T>>
    where
        T: for<'r> sqlx::FromRow<'r, <DbType as sqlx::Database>::Row>
            + SqlColumns
            + Send
            + Sync
            + Unpin
            + 'static,
    {
        let sql = self._sql(T::_sql_cols(), false);
        let (tx, rx) = mpsc::channel(1000);
        let mut executor = conn.acquire_replica().await?;
        tokio::spawn(async move {
            let mut query = sqlx::query_as::<_, T>(&sql);
            let _span = debug_span!("query", sql = &query.sql());
            query = self._bind(query);
            let mut result = query.fetch(executor.as_mut());
            while let Some(v) = result.try_next().await.unwrap_or_else(|e| {
                warn!("{}", e);
                None
            }) {
                if let Err(e) = tx.send(v).await {
                    warn!("{}", e);
                    break;
                }
            }
        });
        Ok(rx)
    }
    @%- if def.use_cache() %@

    #[cfg(not(feature="cache_update_only"))]
    async fn _select_from_cache(self, conn: &mut DbConn) -> Result<Vec<_@{ pascal_name }@Cache>> {
        let mut sql = format!(
            r#"SELECT @{ def.primaries()|fmt_join("{col_query}", ", ") }@ FROM @{ table_name|db_esc }@ as _t1 {} {} {}"#,
            Filter_::write_where(&self.filter, self.trash_mode, TRASHED_SQL, NOT_TRASHED_SQL, ONLY_TRASHED_SQL),
            &self.raw_query,
            Order_::write_order(&self.order),
        );
        if let Some(limit) = self.limit {
            write!(sql, " limit {}", limit)?;
        }
        if let Some(offset) = self.offset {
            write!(sql, " offset {}", offset)?;
        }
        let mut query = sqlx::query_as::<_, InnerPrimary>(&sql);
        let _span = debug_span!("query", sql = &query.sql());
        query = self._bind(query);
        let result = crate::misc::fetch!(conn, query, fetch_all);
        let ids: Vec<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@> = result.iter().map(|id| id.into()).collect();
        conn.release_cache_tx();
        let mut list = _@{ pascal_name }@::find_many_from_cache(conn, ids).await?;
        list.join(conn, self.joiner).await?;
        let mut map = _@{ pascal_name }@::cache_list_to_map(list);
        let list: Vec<_@{ pascal_name }@Cache> = result
            .iter()
            .flat_map(|id| map.remove(&id.into()))
            .collect();
        Ok(list)
    }
    @%- endif %@
    @%- if !def.disable_update() %@

    pub@{ visibility }@ async fn select_for_update(mut self, conn: &mut DbConn) -> Result<Vec<_@{ pascal_name }@Updater>> {
        let sql = self._sql(Data::_sql_cols(), true);
        let mut query = sqlx::query_as::<_, Data>(&sql);
        let _span = debug_span!("query", sql = &query.sql());
        let joiner = self.joiner.take();
        query = self._bind(query);
        let result = if conn.wo_tx() {
            query.fetch_all(conn.acquire_source().await?.as_mut()).await?
        } else {
            query.fetch_all(conn.get_tx().await?.as_mut()).await?
        };
        let mut list: Vec<_Updater_> = result
            .into_iter()
            .map(_Updater_::from)
            .collect();
        list.join(conn, joiner).await?;
        Ok(list)
    }
    @%- endif %@

    pub@{ visibility }@ async fn select(mut self, conn: &mut DbConn) -> Result<Vec<_@{ pascal_name }@>> {
        let joiner = self.joiner.take();
        let result: Vec<Data> = self._select(conn).await?;
        #[allow(unused_mut)]
        let mut list: Vec<_@{ pascal_name }@> = result.into_iter().map(_@{ pascal_name }@::from).collect();
        list.join(conn, joiner).await?;
        Ok(list)
    }

    pub@{ visibility }@ async fn select_one(mut self, conn: &mut DbConn) -> Result<Option<_@{ pascal_name }@>> {
        self.limit = Some(1);
        let mut list = Self::select(self, conn).await?;
        Ok(list.pop())
    }

    pub@{ visibility }@ async fn select_for<T>(self, conn: &mut DbConn) -> Result<Vec<T>>
    where
        T: for<'r> sqlx::FromRow<'r, <DbType as sqlx::Database>::Row> + SqlColumns + Send + Sync + Unpin,
    {
        self._select(conn).await
    }

    pub@{ visibility }@ async fn select_one_for<T>(mut self, conn: &mut DbConn) -> Result<Option<T>>
    where
        T: for<'r> sqlx::FromRow<'r, <DbType as sqlx::Database>::Row> + SqlColumns + Send + Sync + Unpin,
    {
        self.limit = Some(1);
        let mut list = Self::select_for(self, conn).await?;
        Ok(list.pop())
    }
    @%- if !config.force_disable_cache %@

    #[cfg(not(feature="cache_update_only"))]
    pub(crate) async fn __select_for_cache(self, conn: &mut DbConn) -> Result<Vec<_@{ pascal_name }@Cache>> {
        let result: Vec<CacheData> = self._select(conn).await?;
        let time = MSec::now();
        let list = result.into_iter().map(|v| Arc::new(CacheWrapper::_from_inner(v, conn.shard_id(), time)).into()).collect();
        Ok(list)
    }
    @%- endif %@
    @%- if def.use_cache() %@

    #[cfg(feature="cache_update_only")]
    pub@{ visibility }@ async fn select_from_cache(self, conn: &mut DbConn) -> Result<Vec<_@{ pascal_name }@Cache>> {
        unimplemented!("cache_update_only feature disables fetching from cache.")
    }

    #[cfg(not(feature="cache_update_only"))]
    pub@{ visibility }@ async fn select_from_cache(self, conn: &mut DbConn) -> Result<Vec<_@{ pascal_name }@Cache>> {
        self._select_from_cache(conn).await
    }
    @%- endif %@

    pub@{ visibility }@ async fn count(self, conn: &mut DbConn) -> Result<i64> {
        let result: Count = self.select_one_for(conn).await?.unwrap_or_default();
        Ok(result.c)
    }

    pub@{ visibility }@ async fn select_stream(self, conn: &mut DbConn) -> Result<impl Stream<Item = _@{ pascal_name }@>> {
        let mut rx: mpsc::Receiver<Data> = self._select_stream(conn).await?;
        Ok(async_stream::stream! {
            while let Some(v) = rx.recv().await {
                yield  _@{ pascal_name }@::from(v);
            }
        })
    }

    pub@{ visibility }@ async fn select_stream_for<T>(self, conn: &mut DbConn) -> Result<impl Stream<Item = T>>
    where
        T: for<'r> sqlx::FromRow<'r, <DbType as sqlx::Database>::Row> + SqlColumns + Send + Sync + Unpin + 'static,
    {
        let mut rx: mpsc::Receiver<T> = self._select_stream(conn).await?;
        Ok(async_stream::stream! {
            while let Some(v) = rx.recv().await {
                yield  v;
            }
        })
    }
    @%- if !def.disable_update() || def.soft_delete().is_some() %@

    #[allow(unused_mut)]
    pub@{ visibility }@ async fn update(self, conn: &mut DbConn, mut obj: _@{ pascal_name }@Updater) -> Result<u64> {
        @%- if def.updated_at_conf().is_some() %@
        if obj._op.@{ ConfigDef::updated_at()|to_var_name }@ == Op::None {
            obj.mut_@{ ConfigDef::updated_at() }@().set(@{(def.updated_at_conf().unwrap() == Timestampable::RealTime)|if_then_else_ref("SystemTime::now()","conn.time()")}@.into());
        }
        @%- endif %@
        let mut vec: Vec<String> = Vec::new();
        @{- def.non_primaries()|fmt_join_cache_or_not("
        assign_sql_no_cache_update!(obj, vec, {var}, r#\"{col_esc}\"#, {may_null}, \"{placeholder}\");", "
        assign_sql_no_cache_update!(obj, vec, {var}, r#\"{col_esc}\"#, {may_null}, \"{placeholder}\");", "") }@
        let mut sql = format!(
            r#"UPDATE @{ table_name|db_esc }@ as _t1 SET {} {} {} {}"#,
            &vec.join(","),
            Filter_::write_where(
                &self.filter,
                self.trash_mode,
                TRASHED_SQL,
                NOT_TRASHED_SQL,
                ONLY_TRASHED_SQL
            ),
            &self.raw_query,
            Order_::write_order(&self.order)
        );
        if let Some(limit) = self.limit {
            write!(sql, " limit {}", limit)?;
        }
        let mut query = sqlx::query(&sql);
        let _span = debug_span!("query", sql = &query.sql());
        @{- def.non_primaries()|fmt_join("
        for _n in 0..obj._op.{var}.get_bind_num({may_null}) {
            query = query.bind(obj._update.{var}{bind_as});
        }","") }@
        info!(target: "db_update::@{ db|snake }@::@{ group_name }@::@{ mod_name }@", op = "update_with_filter", filter = format!("{:?}", &self.filter), ctx = conn.ctx_no(); "{}", &obj);
        debug!("{:?}", &obj);
        if let Some(c) = self.filter {
            query = c.query_bind(query);
        }
        for value in self.bind.into_iter() {
            debug!("bind: {:?}", &value);
            query = match value {
                BindValue::Bool(v) => query.bind(v),
                BindValue::Enum(v) => query.bind(v),
                BindValue::Number(v) => query.bind(v),
                BindValue::String(v) => query.bind(v),
                BindValue::DateTime(v) => query.bind(v),
                BindValue::Date(v) => query.bind(v),
                BindValue::Time(v) => query.bind(v),
                BindValue::Blob(v) => query.bind(v),
                BindValue::Json(v) => query.bind(v),
                BindValue::Uuid(v) => query.bind(v),
                BindValue::BinaryUuid(v) => query.bind(v),
            };
        }
        let result = if conn.wo_tx() {
            query.execute(conn.acquire_source().await?.as_mut()).await?
        } else {
            query.execute(conn.get_tx().await?.as_mut()).await?
        };
        if !conn.clear_whole_cache && (USE_CACHE || USE_CACHE_ALL || USE_UPDATE_NOTICE) {
            conn.push_cache_op(CacheOp::InvalidateAll.wrap()).await?;
        }
        Ok(result.rows_affected())
    }
    @%- endif %@

    #[allow(unused_mut)]
    pub@{ visibility }@ async fn delete(self, conn: &mut DbConn) -> Result<u64> {
        @{- def.soft_delete_tpl2("
        self.force_delete(conn).await","
        let mut obj = _{pascal_name}::updater();
        obj.mut_deleted_at().set(Some({val}.into()));
        self.update(conn, obj).await","
        let mut obj = _{pascal_name}::updater();
        obj.mut_deleted().set(true);
        self.update(conn, obj).await","
        let mut obj = _{pascal_name}::updater();
        let deleted = cmp::max(1, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as u32);
        obj.mut_deleted().set(deleted);
        self.update(conn, obj).await")}@
    }

    #[allow(unused_mut)]
    pub@{ visibility }@ async fn force_delete(self, conn: &mut DbConn) -> Result<u64> {
        @%- if def.on_delete_list.is_empty() %@
        let mut sql = format!(
            r#"DELETE FROM @{ table_name|db_esc }@ as _t1 {} {} {}"#,
            Filter_::write_where(
                &self.filter,
                self.trash_mode,
                TRASHED_SQL,
                NOT_TRASHED_SQL,
                ONLY_TRASHED_SQL
            ),
            &self.raw_query,
            Order_::write_order(&self.order)
        );
        if let Some(limit) = self.limit {
            write!(sql, " limit {}", limit)?;
        }
        let mut query = sqlx::query(&sql);
        let _span = debug_span!("query", sql = &query.sql());
        info!(target: "db_update::@{ db|snake }@::@{ group_name }@::@{ mod_name }@", op = "delete_with_filter", filter = format!("{:?}", &self.filter), ctx = conn.ctx_no(); "");
        if let Some(c) = self.filter {
            query = c.query_bind(query);
        }
        for value in self.bind.into_iter() {
            debug!("bind: {:?}", &value);
            query = match value {
                BindValue::Bool(v) => query.bind(v),
                BindValue::Enum(v) => query.bind(v),
                BindValue::Number(v) => query.bind(v),
                BindValue::String(v) => query.bind(v),
                BindValue::DateTime(v) => query.bind(v),
                BindValue::Date(v) => query.bind(v),
                BindValue::Time(v) => query.bind(v),
                BindValue::Blob(v) => query.bind(v),
                BindValue::Json(v) => query.bind(v),
                BindValue::Uuid(v) => query.bind(v),
                BindValue::BinaryUuid(v) => query.bind(v),
            };
        }
        let result = if conn.wo_tx() {
            query.execute(conn.acquire_source().await?.as_mut()).await?
        } else {
            query.execute(conn.get_tx().await?.as_mut()).await?
        };
        if !conn.clear_whole_cache && (USE_CACHE || USE_CACHE_ALL || USE_UPDATE_NOTICE) {
            conn.push_cache_op(CacheOp::InvalidateAll.wrap()).await?;
        }
        Ok(result.rows_affected())
        @%- else %@
        let mut sql = format!(
            r#"SELECT @{ def.primaries()|fmt_join("{col_query}", ", ") }@ FROM @{ table_name|db_esc }@ as _t1 {} {} {}"#,
            Filter_::write_where(&self.filter, self.trash_mode, TRASHED_SQL, NOT_TRASHED_SQL, ONLY_TRASHED_SQL),
            &self.raw_query,
            Order_::write_order(&self.order),
        );
        if let Some(limit) = self.limit {
            write!(sql, " limit {}", limit)?;
        }
        if let Some(offset) = self.offset {
            write!(sql, " offset {}", offset)?;
        }
        if self.skip_locked {
            write!(sql, " FOR UPDATE SKIP LOCKED").unwrap();
        } else {
            write!(sql, " FOR UPDATE").unwrap();
        }
        let mut query = sqlx::query_as::<_, InnerPrimary>(&sql);
        let _span = debug_span!("query", sql = &query.sql());
        query = self._bind(query);
        let result = crate::misc::fetch!(conn, query, fetch_all);
        let ids: Vec<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@> = result.iter().map(|id| id.into()).collect();
        _@{ pascal_name }@::force_delete_by_ids(conn, ids).await
        @%- endif %@
    }
}

#[async_trait]
pub@{ visibility }@ trait UnionBuilder {
    async fn select(self, conn: &mut DbConn, order: Option<Vec<Order_>>, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<_@{ pascal_name }@>>;
    async fn select_for_update(self, conn: &mut DbConn) -> Result<Vec<_@{ pascal_name }@Updater>>;
}
@%- if !config.force_disable_cache %@

#[cfg(not(feature="cache_update_only"))]
#[async_trait]
pub(crate) trait _UnionBuilder {
    async fn __select_for_cache(self, conn: &mut DbConn) -> Result<Vec<_@{ pascal_name }@Cache>>;
}
@%- endif %@

async fn _union<T>(
    mut list: Vec<QueryBuilder>,
    conn: &mut DbConn,
    order: Option<Vec<Order_>>,
    limit: Option<usize>,
    offset: Option<usize>,
    for_update: bool,
) -> Result<Vec<T>>
where
    T: for<'r> sqlx::FromRow<'r, <DbType as sqlx::Database>::Row>
        + SqlColumns
        + Send
        + Sync
        + Unpin,
{
    if order.is_none() && limit.is_none() && offset.is_none() {
        let mut result = Vec::new();
        loop {
            let chunk: Vec<_> = list.drain(list.len().saturating_sub(crate::UNION_LIMIT)..).collect();
            if chunk.is_empty() {
                break;
            }
            let mut sql = chunk
                .iter()
                .map(|v| format!("({})", v._sql(T::_sql_cols(), for_update)))
                .collect::<Vec<_>>()
                .join(" UNION ALL ");
            let mut query = sqlx::query_as::<_, T>(&sql);
            let _span = debug_span!("query", sql = &query.sql());
            for builder in chunk {
                query = builder._bind(query);
            }
            result.append(&mut crate::misc::fetch!(conn, query, fetch_all));
        }
        Ok(result)
    } else {
        let mut sql = list
            .iter()
            .map(|v| format!("({})", v._sql(T::_sql_cols(), for_update)))
            .collect::<Vec<_>>()
            .join(" UNION ");
        write!(sql, " {}", Order_::write_order(&order))?;
        if let Some(limit) = limit {
            write!(sql, " limit {}", limit)?;
        }
        if let Some(offset) = offset {
            write!(sql, " offset {}", offset)?;
        }
        let mut query = sqlx::query_as::<_, T>(&sql);
        let _span = debug_span!("query", sql = &query.sql());
        for builder in list {
            query = builder._bind(query);
        }
        let result = crate::misc::fetch!(conn, query, fetch_all);
        Ok(result)
    }
}

#[async_trait]
impl UnionBuilder for Vec<QueryBuilder> {
    async fn select(mut self, conn: &mut DbConn, order: Option<Vec<Order_>>, limit: Option<usize>, offset: Option<usize>) -> Result<Vec<_@{ pascal_name }@>> {
        if self.is_empty() {
            return Ok(Vec::new());
        }
        let result: Vec<Data> = _union(self, conn, order, limit, offset, false).await?;
        Ok(result.into_iter().map(_@{ pascal_name }@::from).collect())
    }
    async fn select_for_update(mut self, conn: &mut DbConn) -> Result<Vec<_@{ pascal_name }@Updater>> {
        if self.is_empty() {
            return Ok(Vec::new());
        }
        let result: Vec<Data> = _union(self, conn, None, None, None, true).await?;
        Ok(result.into_iter().map(_@{ pascal_name }@Updater::from).collect())
    }
}
@%- if !config.force_disable_cache %@

#[cfg(not(feature="cache_update_only"))]
#[async_trait]
impl _UnionBuilder for Vec<QueryBuilder> {
    async fn __select_for_cache(self, conn: &mut DbConn) -> Result<Vec<_@{ pascal_name }@Cache>> {
        let result: Vec<CacheData> = _union(self, conn, None, None, None, false).await?;
        let time = MSec::now();
        let list = result.into_iter().map(|v| Arc::new(CacheWrapper::_from_inner(v, conn.shard_id(), time)).into()).collect();
        Ok(list)
    }
}
@%- endif %@

@% for (name, column_def) in def.id() -%@
impl std::ops::Deref for @{ id_name }@ {
    type Target = @{ column_def.get_deref_type(false) }@;
    fn deref(&self) -> &@{ column_def.get_deref_type(false) }@ {
        &self.0
    }
}

impl @{ id_name }@ {
    pub@{ visibility }@ fn inner(&self) -> @{ column_def.get_inner_type(false, false) }@ {
        self.0@{ column_def.clone_str() }@
    }
@%- if def.primaries().len() == 1 %@
    pub@{ visibility }@ async fn fetch(&self, conn: &mut DbConn) -> Result<Option<_@{ pascal_name }@>> {
        _@{ pascal_name }@::find_optional(conn, self, None).await
    }
@%- if def.is_soft_delete() %@
    pub@{ visibility }@ async fn fetch_with_trashed(&self, conn: &mut DbConn) -> Result<Option<_@{ pascal_name }@>> {
        _@{ pascal_name }@::find_optional_with_trashed(conn, self, None).await
    }
@%- endif %@
@%- if def.use_cache() %@
    #[cfg(feature="cache_update_only")]
    pub@{ visibility }@ async fn fetch_from_cache(&self, conn: &DbConn) -> Result<Option<_@{ pascal_name }@Cache>> {
        unimplemented!("cache_update_only feature disables fetching from cache.")
    }
    #[cfg(not(feature="cache_update_only"))]
    pub@{ visibility }@ async fn fetch_from_cache(&self, conn: &DbConn) -> Result<Option<_@{ pascal_name }@Cache>> {
        _@{ pascal_name }@::find_optional_from_cache(conn, self).await
    }
@%- if def.is_soft_delete() %@
    #[cfg(feature="cache_update_only")]
    pub@{ visibility }@ async fn fetch_from_cache_with_trashed(&self, conn: &DbConn) -> Result<Option<_@{ pascal_name }@Cache>> {
        unimplemented!("cache_update_only feature disables fetching from cache.")
    }
    #[cfg(not(feature="cache_update_only"))]
    pub@{ visibility }@ async fn fetch_from_cache_with_trashed(&self, conn: &DbConn) -> Result<Option<_@{ pascal_name }@Cache>> {
        _@{ pascal_name }@::find_optional_from_cache_with_trashed(conn, self).await
    }
@%- endif %@
@%- endif %@
@%- if !def.disable_update() %@
    pub@{ visibility }@ async fn fetch_for_update(&self, conn: &mut DbConn) -> Result<_@{ pascal_name }@Updater> {
        _@{ pascal_name }@::find_for_update(conn, self, None).await
    }
@%- if def.is_soft_delete() %@
    pub@{ visibility }@ async fn fetch_for_update_with_trashed(&self, conn: &mut DbConn) -> Result<_@{ pascal_name }@Updater> {
        _@{ pascal_name }@::find_for_update_with_trashed(conn, self, None).await
    }
@%- endif %@
    pub@{ visibility }@ fn updater(&self) -> _@{ pascal_name }@Updater {
        _@{ pascal_name }@Updater {
            _data: Data {
                @{ name }@: self.inner(),
                ..Data::default()
            },
            _update: Data::default(),
            _is_new: false,
            _do_delete: false,
            _upsert: false,
            _is_loaded: false,
            _op: OpData::default(),
@{- def.relations_one(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_many(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
        }
    }
@%- endif %@
@%- endif %@
}

#[async_trait]
pub@{ visibility }@ trait @{ id_name }@Fetcher {
@%- if def.use_cache() %@
    async fn fetch_from_cache(&self, conn: &DbConn) -> Result<Option<_@{ pascal_name }@Cache>>;
    @%- if def.is_soft_delete() %@
    async fn fetch_from_cache_with_trashed(&self, conn: &DbConn) -> Result<Option<_@{ pascal_name }@Cache>>;
    @%- endif %@
@%- endif %@
}
@%- if def.primaries().len() == 1 && def.use_cache() %@

#[async_trait]
impl @{ id_name }@Fetcher for Option<@{ id_name }@> {
@%- if def.use_cache() %@
    #[cfg(feature="cache_update_only")]
    async fn fetch_from_cache(&self, conn: &DbConn) -> Result<Option<_@{ pascal_name }@Cache>> {
        unimplemented!("cache_update_only feature disables fetching from cache.")
    }
    #[cfg(not(feature="cache_update_only"))]
    async fn fetch_from_cache(&self, conn: &DbConn) -> Result<Option<_@{ pascal_name }@Cache>> {
        if let Some(id) = self {
            _@{ pascal_name }@::find_optional_from_cache(conn, id).await
        } else {
            Ok(None)
        }
    }
    @%- if def.is_soft_delete() %@
    #[cfg(feature="cache_update_only")]
    async fn fetch_from_cache_with_trashed(&self, conn: &DbConn) -> Result<Option<_@{ pascal_name }@Cache>> {
        unimplemented!("cache_update_only feature disables fetching from cache.")
    }
    #[cfg(not(feature="cache_update_only"))]
    async fn fetch_from_cache_with_trashed(&self, conn: &DbConn) -> Result<Option<_@{ pascal_name }@Cache>> {
        if let Some(id) = self {
            _@{ pascal_name }@::find_from_cache_with_trashed(conn, id)
                .await
                .map(Some)
        } else {
            Ok(None)
        }
    }
    @%- endif %@
@%- endif %@
}
@%- endif %@

impl From<@{ column_def.get_inner_type(false, false) }@> for @{ id_name }@ {
    fn from(id: @{ column_def.get_inner_type(false, false) }@) -> Self {
        Self(id)
    }
}
impl From<&@{ column_def.get_inner_type(false, false) }@> for @{ id_name }@ {
    fn from(id: &@{ column_def.get_inner_type(false, false) }@) -> Self {
        Self(id.clone())
    }
}
impl From<@{ id_name }@> for @{ column_def.get_inner_type(false, false) }@ {
    fn from(id: @{ id_name }@) -> Self {
        id.0
    }
}
@%- if column_def.get_inner_type(true, false) != column_def.get_inner_type(false, false)%@
impl From<@{ column_def.get_inner_type(true, false) }@> for @{ id_name }@ {
    fn from(id: @{ column_def.get_inner_type(true, false) }@) -> Self {
        Self(id.into())
    }
}
impl From<&@{ column_def.get_inner_type(true, false) }@> for @{ id_name }@ {
    fn from(id: &@{ column_def.get_inner_type(true, false) }@) -> Self {
        Self(id.clone().into())
    }
}
impl From<@{ id_name }@> for @{ column_def.get_inner_type(true, false) }@ {
    fn from(id: @{ id_name }@) -> Self {
        id.0.as_ref().clone()
    }
}
impl From<@{ def.primaries()|fmt_join_with_paren("{raw_inner}", ", ") }@> for Primary {
    fn from(id: @{ def.primaries()|fmt_join_with_paren("{raw_inner}", ", ") }@) -> Self {
        @% if def.primaries().len() == 1 %@Self(id.into())@% else %@Self(@{ def.primaries()|fmt_join("id.{index}.into()", ", ") }@)@% endif %@
    }
}
@%- endif %@
impl From<&@{ id_name }@> for @{ id_name }@ {
    fn from(id: &@{ id_name }@) -> Self {
        Self(id.inner())
    }
}
@%- endfor %@
impl From<&InnerPrimary> for @{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@ {
    fn from(id: &InnerPrimary) -> Self {
        @{ def.primaries()|fmt_join_with_paren("id.{index}{clone}.into()", ", ") }@
    }
}
impl From<@{ def.primaries()|fmt_join_with_paren("{outer_ref}", ", ") }@> for Primary {
    fn from(id: @{ def.primaries()|fmt_join_with_paren("{outer_ref}", ", ") }@) -> Self {
        @% if def.primaries().len() == 1 -%@
        Self(id.to_owned().into())
        @%- else -%@
        Self(@{ def.primaries()|fmt_join("id.{index}.to_owned().into()", ", ") }@)
        @%- endif %@
    }
}
impl From<&Primary> for Primary {
    fn from(id: &Primary) -> Self {
        id.clone()
    }
}
impl From<&Primary> for InnerPrimary {
    fn from(id: &Primary) -> Self {
        Self(@{ def.primaries()|fmt_join("id.{index}{clone}.into()", ", ") }@)
    }
}
impl From<&InnerPrimary> for Primary {
    fn from(id: &InnerPrimary) -> Self {
        Self(@{ def.primaries()|fmt_join("id.{index}{clone}.into()", ", ") }@)
    }
}
@%- if def.primaries()|fmt_join_with_paren("{outer_ref}", ", ") != def.primaries()|fmt_join_with_paren("{inner}", ", ") %@
impl From<@{ def.primaries()|fmt_join_with_paren("{inner}", ", ") }@> for Primary {
    fn from(id: @{ def.primaries()|fmt_join_with_paren("{inner}", ", ") }@) -> Self {
        @% if def.primaries().len() == 1 %@Self(id.into())@% else %@Self(@{ def.primaries()|fmt_join("id.{index}.into()", ", ") }@)@% endif %@
    }
}
@%- endif %@
@%- if def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") != def.primaries()|fmt_join_with_paren("{inner}", ", ") %@
impl From<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@> for Primary {
    fn from(id: @{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@) -> Self {
        @% if def.primaries().len() == 1 %@Self(id)@% else %@Self(@{ def.primaries()|fmt_join("id.{index}", ", ") }@)@% endif %@
    }
}
@%- endif %@
@%- if def.primaries().len() > 1 %@
impl From<&@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@> for Primary {
    fn from(id: &@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@) -> Self {
        Self(@{ def.primaries()|fmt_join("id.{index}{clone}", ", ") }@)
    }
}
@%- endif %@
impl From<&_@{ pascal_name }@> for Primary {
    fn from(obj: &_@{ pascal_name }@) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._inner.{var}{clone}.into()", ", ") }@)
    }
}
impl From<&_@{ pascal_name }@> for InnerPrimary {
    fn from(obj: &_@{ pascal_name }@) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._inner.{var}{clone}", ", ") }@)
    }
}

impl From<&Data> for Primary {
    fn from(obj: &Data) -> Self {
        Self(@{ def.primaries()|fmt_join("obj.{var}{clone}.into()", ", ") }@)
    }
}
impl From<&Data> for InnerPrimary {
    fn from(obj: &Data) -> Self {
        Self(@{ def.primaries()|fmt_join("obj.{var}{clone}", ", ") }@)
    }
}
@%- if !config.force_disable_cache %@

impl From<&CacheData> for InnerPrimary {
    fn from(obj: &CacheData) -> Self {
        Self(@{ def.primaries()|fmt_join("obj.{var}{clone}", ", ") }@)
    }
}

impl From<&_@{ pascal_name }@Cache> for Primary {
    fn from(obj: &_@{ pascal_name }@Cache) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._wrapper._inner.{var}{clone}.into()", ", ") }@)
    }
}
impl From<&_@{ pascal_name }@Cache> for InnerPrimary {
    fn from(obj: &_@{ pascal_name }@Cache) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._wrapper._inner.{var}{clone}", ", ") }@)
    }
}

impl From<&Arc<CacheWrapper>> for InnerPrimary {
    fn from(obj: &Arc<CacheWrapper>) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._inner.{var}{clone}", ", ") }@)
    }
}
impl From<&CacheWrapper> for Primary {
    fn from(obj: &CacheWrapper) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._inner.{var}{clone}.into()", ", ") }@)
    }
}
@%- endif %@

impl From<&_@{ pascal_name }@Updater> for Primary {
    fn from(obj: &_@{ pascal_name }@Updater) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._data.{var}{clone}.into()", ", ") }@)
    }
}

impl From<&_@{ pascal_name }@Updater> for InnerPrimary {
    fn from(obj: &_@{ pascal_name }@Updater) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._data.{var}{clone}", ", ") }@)
    }
}

impl From<&mut _@{ pascal_name }@Updater> for InnerPrimary {
    fn from(obj: &mut _@{ pascal_name }@Updater) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._data.{var}{clone}", ", ") }@)
    }
}

fn id_to_string(id: &InnerPrimary) -> String {
    format!("@{ def.primaries()|fmt_join("{col}={disp}", ", ") }@"@{ def.primaries()|fmt_join(", id.{index}", "") }@)
}

impl fmt::Display for InnerPrimary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{ def.primaries()|fmt_join("{col}={disp}", ", ") }@"@{ def.primaries()|fmt_join(", self.{index}", "") }@)
    }
}

#[allow(clippy::useless_format)]
fn primaries_to_str(s: &[InnerPrimary]) -> String {
    let v = s.iter().fold(Vec::new(), |mut i, p| {
        i.push(format!("@{ def.primaries()|fmt_join_with_paren("{disp}", ", ") }@"@{ def.primaries()|fmt_join(", p.{index}", "") }@));
        i
    });
    format!("@{ def.primaries()|fmt_join_with_paren("{col}", ", ") }@={}", v.join(","))
}

impl _@{ pascal_name }@Getter for _@{ pascal_name }@ {
    @{- def.all_fields()|fmt_join("
    fn _{raw_var}(&self) -> {outer} {
        {convert_outer_prefix}self._inner.{var}{clone_for_outer}{convert_outer}
    }", "") }@
    @{- def.relations_one_and_belonging(false)|fmt_rel_join("
    fn _{raw_rel_name}(&self) -> Option<&rel_{class_mod}::{class}> {
        self.{rel_name}.as_ref().expect(\"{rel_name} is not loaded\").as_ref().map(|b| &**b)
    }", "") }@
    @{- def.relations_many(false)|fmt_rel_join("
    fn _{raw_rel_name}(&self) -> &Vec<rel_{class_mod}::{class}> {
        self.{rel_name}.as_ref().expect(\"{rel_name} is not loaded\")
    }", "") }@
}

@%- for parent in def.parents() %@
impl crate::models::@{ parent.group_name|to_var_name }@::_base::_@{ parent.name }@::_@{ parent.name|pascal }@Getter for _@{ pascal_name }@ {
    @{- parent.primaries()|fmt_join("
    fn _{raw_var}(&self) -> &{inner} {
        &self._inner.{var}
    }", "") }@
    @{- parent.non_primaries()|fmt_join("
    fn _{raw_var}(&self) -> {outer} {
        {convert_outer_prefix}self._inner.{var}{clone_for_outer}{convert_outer}
    }", "") }@
    @{- parent.relations_one_and_belonging(false)|fmt_rel_join("
    fn _{raw_rel_name}(&self) -> Option<&rel_{class_mod}::{class}> {
        self.{rel_name}.as_ref().expect(\"{rel_name} is not loaded\").as_ref().map(|b| &**b)
    }", "") }@
    @{- parent.relations_many(false)|fmt_rel_join("
    fn _{raw_rel_name}(&self) -> &Vec<rel_{class_mod}::{class}> {
        self.{rel_name}.as_ref().expect(\"{rel_name} is not loaded\").as_ref()
    }", "") }@
}
@%- endfor %@
@%- if !config.force_disable_cache %@

static CACHE_WRAPPER_AVG: AtomicUsize = AtomicUsize::new(0);
static CACHE_WRAPPER_AVG_NUM: AtomicUsize = AtomicUsize::new(0);

impl CacheVal for CacheWrapper {
    fn _size(&self) -> u32 {
        let mut size = calc_mem_size(std::mem::size_of::<Self>());
        @{- def.cache_cols_not_null_sized()|fmt_join("
        size += self._inner.{var}._size();", "") }@
        @{- def.cache_cols_null_sized()|fmt_join("
        size += self._inner.{var}.as_ref().map(|v| v._size()).unwrap_or(0);", "") }@
        @{- def.relations_one_cache(false)|fmt_rel_join("
        size += self.{rel_name}.as_ref().map(|v| v._size() as usize).unwrap_or(0);", "") }@
        @{- def.relations_many_cache(false)|fmt_rel_join("
        size += self.{rel_name}.iter().fold(0, |i, v| i + v._size() as usize);", "") }@
        size.try_into().unwrap_or(u32::MAX)
    }
    fn _type_id(&self) -> u64 {
        Self::__type_id()
    }
    fn __type_id() -> u64 {
        CACHE_TYPE_ID
    }
    fn _shard_id(&self) -> ShardId {
        self._shard_id
    }
    fn _time(&self) -> MSec {
        self._time
    }
    fn _estimate() -> usize {
        CACHE_WRAPPER_AVG.load(Ordering::Relaxed)
    }
    fn _encode(&self) -> Result<Vec<u8>> {
        let mut buf = Vec::new();
        ciborium::into_writer(self, &mut buf)?;
        let vec = encode_all(buf.as_slice(), 1)?;
        let num = CACHE_WRAPPER_AVG_NUM.load(Ordering::Relaxed);
        let ave = (CACHE_WRAPPER_AVG.load(Ordering::Relaxed) * num + vec.len()) / num.saturating_add(1);
        CACHE_WRAPPER_AVG_NUM.store(num.saturating_add(1), Ordering::Relaxed);
        CACHE_WRAPPER_AVG.store(ave, Ordering::Relaxed);
        Ok(vec)
    }
    fn _decode(v: &[u8]) -> Result<Self> {
        Ok(ciborium::from_reader(decode_all(v)?.as_slice())?)
    }
}

impl CacheWrapper {
    @{- def.cache_cols()|fmt_join("
    fn _{raw_var}(&self) -> {outer} {
        {convert_outer_prefix}self._inner.{var}{clone_for_outer}{convert_outer}
    }", "") }@
    @{- def.relations_one_cache(false)|fmt_rel_join("
    fn _{raw_rel_name}(&self) -> Option<&Arc<rel_{class_mod}::CacheWrapper>> {
        self.{rel_name}.as_ref()
    }", "") }@
    @{- def.relations_many_cache(false)|fmt_rel_join("
    fn _{raw_rel_name}(&self) -> &Vec<Arc<rel_{class_mod}::CacheWrapper>> {
        self.{rel_name}.as_ref()
    }", "") }@
}

impl _@{ pascal_name }@Cache {
    @{- def.cache_cols()|fmt_join("
{label}{comment}    pub fn _{raw_var}(&self) -> {outer} {
        self._wrapper._{raw_var}()
    }", "") }@
    @{- def.relations_one_cache(false)|fmt_rel_join("
{label}{comment}    pub fn _{raw_rel_name}(&self) -> Option<rel_{class_mod}::{class}Cache> {
        if let Some(v) = &self.{rel_name} {
            v.as_ref().map(|v| (**v).clone())
        } else {
            self._wrapper._{raw_rel_name}().map(|v| v.clone().into())
        }
    }", "") }@
    @{- def.relations_one_uncached(false)|fmt_rel_join("
{label}{comment}    pub fn _{raw_rel_name}(&self) -> Option<rel_{class_mod}::{class}> {
        self.{rel_name}.as_ref().expect(\"{rel_name} is not loaded\").as_ref().map(|v| (**v).clone())
    }", "") }@
    @{- def.relations_many_cache(false)|fmt_rel_join("
{label}{comment}    pub fn _{raw_rel_name}(&self) -> Vec<rel_{class_mod}::{class}Cache> {
        if let Some(v) = &self.{rel_name} {
            v.to_vec()
        } else {
            self._wrapper._{raw_rel_name}().iter().map(|v| v.clone().into()).collect()
        }
    }", "") }@
    @{- def.relations_many_uncached(false)|fmt_rel_join("
{label}{comment}    pub fn _{raw_rel_name}(&self) -> Vec<rel_{class_mod}::{class}> {
        self.{rel_name}.as_ref().expect(\"{rel_name} is not loaded\").to_vec()
    }", "") }@
    @{- def.relations_belonging_cache(false)|fmt_rel_join("
{label}{comment}    pub fn _{raw_rel_name}(&self) -> Option<rel_{class_mod}::{class}Cache> {
        self.{rel_name}.as_ref().expect(\"{rel_name} is not loaded\").as_ref().map(|b| *b.clone())
    }", "") }@
    @{- def.relations_belonging_uncached(false)|fmt_rel_join("
{label}{comment}    pub fn _{raw_rel_name}(&self) -> Option<rel_{class_mod}::{class}> {
        self.{rel_name}.as_ref().expect(\"{rel_name} is not loaded\").as_ref().map(|b| *b.clone())
    }", "") }@
    pub@{ visibility }@ async fn invalidate_cache<T>(conn: &DbConn, id: T) -> Result<()>
    where
        T: Into<Primary>,
    {
        if USE_CACHE || USE_CACHE_ALL {
            @%- if def.act_as_job_queue() %@
            @%- else if def.use_clear_whole_cache() %@
            let sync = DbConn::inc_cache_sync(conn.shard_id()).await?;
            let mut sync_map = FxHashMap::default();
            sync_map.insert(conn.shard_id(), sync);
            CacheMsg(vec![crate::CacheOp::_AllClear], sync_map)
                .do_send()
                .await;
            @%- else %@
            let id: InnerPrimary = (&id.into()).into();
            let sync = DbConn::inc_cache_sync(conn.shard_id()).await?;
            let mut sync_map = FxHashMap::default();
            sync_map.insert(conn.shard_id(), sync);
            CacheMsg(vec![CacheOp::Invalidate{id, shard_id: conn.shard_id()}.wrap()], sync_map)
                .do_send()
                .await;
            @%- endif %@
        }
        Ok(())
    }
    pub(crate) fn __invalidate_cache_op<T>(conn: &DbConn, id: T) -> crate::CacheOp
    where
        T: Into<Primary>,
    {
        @%- if def.act_as_job_queue() || def.use_clear_whole_cache() %@
        CacheOp::None.wrap()
        @%- else %@
        if USE_CACHE || USE_CACHE_ALL {
            let id: InnerPrimary = (&id.into()).into();
            CacheOp::Invalidate{id, shard_id: conn.shard_id()}.wrap()
        } else {
            CacheOp::None.wrap()
        }
        @%- endif %@
    }
    @%- if def.use_cache() %@
    #[cfg(feature="cache_update_only")]
    pub@{ visibility }@ async fn find_self(&self, conn: &DbConn) -> Result<Self> {
        unimplemented!("cache_update_only feature disables fetching from cache.")
    }
    #[cfg(not(feature="cache_update_only"))]
    pub@{ visibility }@ async fn find_self(&self, conn: &DbConn) -> Result<Self> {
        @%- if def.is_soft_delete() %@
        _@{ pascal_name }@::find_from_cache_with_trashed(conn, Primary::from(self)).await
        @%- else %@
        _@{ pascal_name }@::find_from_cache(conn, Primary::from(self)).await
        @%- endif %@
    }
    @%- endif %@
}
@%- endif %@

impl Updater for _@{ pascal_name }@Updater {
    fn is_new(&self) -> bool {
        self._is_new
    }
    fn has_been_deleted(&self) -> bool {
        @{ def.soft_delete_tpl("false","self._data.deleted_at.is_some()","self._data.deleted != 0")}@
    }
    fn mark_for_delete(&mut self) {
        self._do_delete = true;
    }
    fn mark_for_delete_and_return_self(mut self) -> Self {
        self._do_delete = true;
        self
    }
    fn unmark_for_delete(&mut self) {
        self._do_delete = false;
    }
    fn will_be_deleted(&self) -> bool {
        self._do_delete
    }
    fn mark_for_upsert(&mut self) {
        self._upsert = true;
    }
    fn is_updated(&self) -> bool {
        false
        @%- if !def.disable_update() %@
        @{- def.non_primaries()|fmt_join("
        || self._op.{var} != Op::None && self._op.{var} != Op::Skip", "") }@
        @%- endif %@
    }
    // fn __eq(&self, updater: &Self) -> bool {
    //     true
    //     @%- if !def.disable_update() %@
    //     @{- def.for_cmp()|fmt_join("
    //     && self._data.{var} == updater._data.{var}", "") }@
    //     @%- endif %@
    // }
    // fn __set(&mut self, updater: Self) {
    //     @%- if !def.disable_update() %@
    //     @{- def.for_cmp()|fmt_join("
    //     self._op.{var} = Op::Set;
    //     self._data.{var} = updater._data.{var}.clone();
    //     self._update.{var} = updater._data.{var};", "") }@
    //     @%- endif %@
    // }
    fn overwrite_except_skip(&mut self, updater: Self) {
        self.overwrite_with(updater, false)
    }
    fn overwrite_only_set(&mut self, updater: Self) {
        self.overwrite_with(updater, true)
    }
    fn overwrite_with(&mut self, updater: Self, set_only: bool) {
        @%- if !def.disable_update() %@
        @{- def.for_cmp()|fmt_join("
        if (if set_only { updater._op.{var} == Op::Set } else { updater._op.{var} != Op::Skip }) && self._data.{var} != updater._data.{var} {
            self._op.{var} = Op::Set;
            self._data.{var} = updater._data.{var}.clone();
            self._update.{var} = updater._data.{var};
        }", "") }@
        @%- endif %@
    }
}

#[allow(non_snake_case)]
impl _@{ pascal_name }@Updater {
@{- def.all_fields()|fmt_join("
{label}{comment}    pub fn _{raw_var}(&self) -> {outer} {
        {convert_outer_prefix}self._data.{var}{clone_for_outer}{convert_outer}
    }", "") }@
@{- def.primaries()|fmt_join("
{label}{comment}    pub fn mut_{raw_var}(&self) -> Accessor{accessor_with_type} {
        Accessor{accessor} {
            val: &self._data.{var},
            _phantom: Default::default(),
        }
    }", "") }@
@{- def.non_primaries()|fmt_join("
{label}{comment}    pub fn mut_{raw_var}(&mut self) -> Accessor{accessor_with_type} {
        Accessor{accessor} {
            op: &mut self._op.{var},
            val: &mut self._data.{var},
            update: &mut self._update.{var},
            _phantom: Default::default(),
        }
    }", "") }@
@{- def.relations_one(false)|fmt_rel_join("
{label}{comment}    pub fn mut_{raw_rel_name}(&mut self) -> AccessorHasOne<rel_{class_mod}::{class}Updater> {
        AccessorHasOne {
            name: \"{rel_name}\",
            val: &mut self.{rel_name},
        }
    }", "") }@
@{- def.relations_many(false)|fmt_rel_join("
{label}{comment}    pub fn mut_{raw_rel_name}(&mut self) -> AccessorHasMany<rel_{class_mod}::{class}Updater> {
        AccessorHasMany {
            name: \"{rel_name}\",
            val: &mut self.{rel_name},
        }
    }", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("
    fn _{raw_rel_name}(&self) -> Option<&rel_{class_mod}::{class}> {
        self.{rel_name}.as_ref().expect(\"{rel_name} is not loaded\").as_ref().map(|b| &**b)
    }", "") }@
    pub(crate) fn __validate(&self) -> Result<()> {
        self._data.validate()?;
@{- def.relations_one_and_many(false)|fmt_rel_join("
        if let Some(v) = self.{rel_name}.as_ref() {
            for v in v.iter() {
                v.__validate()?;
            }
        }", "") }@
        Ok(())
    }
    @%- if config.use_sequence %@
    #[allow(clippy::unnecessary_cast)]
    #[allow(clippy::only_used_in_recursion)]
    #[async_recursion::async_recursion]
    pub(crate) async fn __set_default_value(&mut self, conn: &mut DbConn) -> Result<()>
    @%- else %@
    pub(crate) fn __set_default_value(&mut self, conn: &DbConn)
    @%- endif %@
    {
        if self.is_new() {
            @{- def.auto_seq()|fmt_join("
            if self._data.{var} == 0 {
                self._data.{var} = conn.sequence(1).await? as {inner};
            }", "") }@
            @{- def.auto_uuid()|fmt_join("
            if self._data.{var}.is_nil() {
                if let Some(uuid_node) = crate::UUID_NODE.get() {
                    self._data.{var} = uuid::Uuid::now_v6(uuid_node);
                } else {
                    self._data.{var} = uuid::Uuid::now_v7();
                }
            }", "") }@
            @%- if def.created_at_conf().is_some() %@
            if self._op.@{ ConfigDef::created_at()|to_var_name }@ == Op::None {
                self._data.@{ ConfigDef::created_at()|to_var_name }@ = @{(def.created_at_conf().unwrap() == Timestampable::RealTime)|if_then_else_ref("SystemTime::now()","conn.time()")}@.into();
            }
            @%- endif %@
            @%- if def.updated_at_conf().is_some() %@
            if self._op.@{ ConfigDef::updated_at()|to_var_name }@ == Op::None {
                self._data.@{ ConfigDef::updated_at()|to_var_name }@ = @{(def.updated_at_conf().unwrap() == Timestampable::RealTime)|if_then_else_ref("SystemTime::now()","conn.time()")}@.into();
            }
            @%- endif %@
            @%- if def.versioned %@
            self._data.@{ version_col }@ = 1;
            @%- endif %@
            @{ def.inheritance_set() }@
        }
        @%- if def.updated_at_conf().is_some() %@
        if (self.is_updated() || self.will_be_deleted()) && self._op.@{ ConfigDef::updated_at()|to_var_name }@ == Op::None {
            self.mut_@{ ConfigDef::updated_at() }@().set(@{(def.updated_at_conf().unwrap() == Timestampable::RealTime)|if_then_else_ref("SystemTime::now()","conn.time()")}@.into());
        }
        @%- endif %@
        @%- if config.use_sequence %@
@{- def.relations_one_and_many(false)|fmt_rel_join("
        if let Some(v) = self.{rel_name}.as_mut() {
            for v in v.iter_mut() {
                RelCol{rel_name_pascal}::set_op_none(&mut v._op);
                v.__set_default_value(conn).await?;
            }
        }", "") }@
        Ok(())
        @%- else %@
@{- def.relations_one_and_many(false)|fmt_rel_join("
        if let Some(v) = self.{rel_name}.as_mut() {
            for v in v.iter_mut() {
                RelCol{rel_name_pascal}::set_op_none(&mut v._op);
                v.__set_default_value(conn);
            }
        }", "") }@
        @%- endif %@
    }

    #[allow(clippy::only_used_in_recursion)]
    pub(crate) fn __set_overwrite_extra_value(&mut self, conn: &mut DbConn)
    {
        if self.will_be_deleted() {
            @{- def.soft_delete_tpl2("
            panic!(\"DELETE is not supported.\");","
            self.mut_deleted_at().set(Some({val}.into()));","
            self.mut_deleted().set(true);","
            let deleted = cmp::max(1, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as u32);
            self.mut_deleted().set(deleted);")}@
        }
        @%- if def.versioned %@
        if !self.is_new() {
            let version = self._@{ ConfigDef::version() }@().wrapping_add(1);
            self.mut_@{ ConfigDef::version() }@().set(version);
        }
        @%- endif %@
        @{- def.relations_one_and_many(false)|fmt_rel_join("
        if let Some(v) = self.{rel_name}.as_mut() {
            for v in v.iter_mut() {
                v.__set_overwrite_extra_value(conn);
            }
        }", "") }@
    }
}

impl fmt::Display for _@{ pascal_name }@Updater {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_new() {
            write!(f, "{{INSERT: {}}}", &self._data)?;
        } else {
            write!(f, "{{UPDATE: {{")?;
            @%- if !def.disable_update() %@
            @{- def.all_except_secret()|fmt_join("
            Accessor{accessor_with_sep_type}::_write_update(f, \"{comma}\", \"{raw_var}\", self._op.{var}, &self._update.{var})?;", "") }@
            @%- endif %@
            write!(f, "}}}}")?;
        }
        Ok(())
    }
}

impl From<Data> for _@{ pascal_name }@ {
    fn from(_inner: Data) -> Self {
        Self {
            _inner,
@{- def.relations_one_and_belonging(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_many(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
        }
    }
}
impl From<Data> for _@{ pascal_name }@Updater {
    fn from(_data: Data) -> Self {
        Self {
            _data,
            _update: Data::default(),
            _is_new: false,
            _do_delete: false,
            _upsert: false,
            _is_loaded: true,
            _op: OpData::default(),
@{- def.relations_one(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_many(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
        }
    }
}
@%- if !config.force_disable_cache %@

impl From<Arc<CacheWrapper>> for _@{ pascal_name }@Cache {
    fn from(wrapper: Arc<CacheWrapper>) -> Self {
        Self {
            _wrapper: wrapper,
@{- def.relations_one_cache(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_one_uncached(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_many_cache(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_many_uncached(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_belonging_cache(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_belonging_uncached(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
        }
    }
}

impl CacheWrapper {
    fn _from_inner(inner: CacheData, shard_id: ShardId, time: MSec) -> Self {
        Self {
            _inner: inner,
            _shard_id: shard_id,
            _time: time,
@{- def.relations_one_cache(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_many_cache(false)|fmt_rel_join("\n            {rel_name}: Vec::new(),", "") }@
        }
    }
    pub(crate) fn _from_data(data: Data, shard_id: ShardId, time: MSec) -> Self {
        Self {
            _inner: CacheData {
@{- def.cache_cols()|fmt_join("\n                {var}: data.{var},", "") }@
            },
            _shard_id: shard_id,
            _time: time,
@{- def.relations_one_cache(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_many_cache(false)|fmt_rel_join("\n            {rel_name}: Vec::new(),", "") }@
        }
    }
}
@%- endif %@

impl Serialize for _@{ pascal_name }@ {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        #[allow(unused_mut)]
        let mut len = @{ def.serializable().len() }@;
        @{- def.relations_one_and_belonging(false)|fmt_rel_join("
        if self.{rel_name}.is_some() {
            len += 1;
        }", "") }@
        @{- def.relations_many(false)|fmt_rel_join("
        if self.{rel_name}.is_some() {
            len += 1;
        }", "") }@
        let mut state = serializer.serialize_struct("@{ pascal_name }@", len)?;
        @{- def.serializable()|fmt_join("
        state.serialize_field(\"{var}\", &(self._inner.{var}{convert_serialize}))?;", "") }@
        @{- def.relations_one_and_belonging(false)|fmt_rel_join("
        if self.{rel_name}.is_some() {
            state.serialize_field(\"{rel_name}\", &self.{rel_name})?;
        }", "") }@
        @{- def.relations_many(false)|fmt_rel_join("
        if self.{rel_name}.is_some() {
            state.serialize_field(\"{rel_name}\", &self.{rel_name})?;
        }", "") }@
        state.end()
    }
}
@%- if !config.force_disable_cache %@

impl Serialize for _@{ pascal_name }@Cache {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = @{ def.serializable_cache().len() + def.relations_one_cache(false).len() + def.relations_belonging_cache(false).len() + def.relations_many_cache(false).len() }@;
        let mut state = serializer.serialize_struct("@{ pascal_name }@", len)?;
        @{- def.serializable_cache()|fmt_join("
        state.serialize_field(\"{var}\", &(self._wrapper._inner.{var}{convert_serialize}))?;", "") }@
        @{- def.relations_one_cache(false)|fmt_rel_join("
        state.serialize_field(\"{rel_name}\", &self._{raw_rel_name}())?;", "") }@
        @{- def.relations_many_cache(false)|fmt_rel_join("
        state.serialize_field(\"{rel_name}\", &self._{raw_rel_name}())?;", "") }@
        @{- def.relations_belonging_cache(false)|fmt_rel_join("
        if self.{rel_name}.is_some() {
            state.serialize_field(\"{rel_name}\", &self.{rel_name})?;
        }", "") }@
        state.end()
    }
}
@%- endif %@

impl _@{ pascal_name }@ {
    pub@{ visibility }@ fn updater() -> _@{ pascal_name }@Updater {
        _@{ pascal_name }@Updater {
            _data: Data::default(),
            _update: Data::default(),
            _is_new: false,
            _do_delete: false,
            _upsert: false,
            _is_loaded: false,
            _op: OpData::default(),
@{- def.relations_one(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_many(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
        }
    }

    pub@{ visibility }@ fn updater_of<T>(id: T) -> _@{ pascal_name }@Updater
    where
        T: Into<Primary>,
    {
        let id: InnerPrimary = (&id.into()).into();
        _@{ pascal_name }@Updater {
            _data: Data {
                @{- def.primaries()|fmt_join("
                {var}: id.{index}{raw_to_inner},", "") }@
                ..Data::default()
            },
            _update: Data::default(),
            _is_new: false,
            _do_delete: false,
            _upsert: false,
            _is_loaded: false,
            _op: OpData::default(),
@{- def.relations_one(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_many(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
        }
    }
@%- for parent in def.downcast_aggregation() %@

    pub@{ visibility }@ fn downcast_from(base: &crate::models::@{ parent.group_name|to_var_name }@::_base::_@{ parent.name }@::_@{ parent.name|pascal }@) -> Option<_@{ pascal_name }@> {
        if base._inner.@{ def.inheritance_check() }@ {
            let clone = base._inner.clone();
            Some(Self {
                _inner: Data {
                    @{- def.all_fields()|fmt_join("
                    {var}: clone.{var},", "") }@
                },
@{- def.relations_one_and_belonging(false)|fmt_rel_join("\n                {rel_name}: None,", "") }@
@{- def.relations_many(false)|fmt_rel_join("\n                {rel_name}: None,", "") }@
            })
        } else {
            None
        }
    }
@%- endfor %@
@%- for parent in def.downcast_simple() %@

    pub@{ visibility }@ fn downcast_from(base: &crate::models::@{ parent.group_name|to_var_name }@::_base::_@{ parent.name }@::_@{ parent.name|pascal }@) -> _@{ pascal_name }@ {
        let clone = base._inner.clone();
        Self {
            _inner: Data {
                @{- def.all_fields()|fmt_join("
                {var}: clone.{var},", "") }@
            },
@{- def.relations_one_and_belonging(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
@{- def.relations_many(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
        }
    }
@%- endfor %@
@%- if def.use_all_row_cache() %@
@%- if def.use_filtered_row_cache() %@

    #[cfg(feature="cache_update_only")]
    pub@{ visibility }@ async fn find_all_from_cache(
        conn: &DbConn,
        filter: Option<Filter_>,
        order: Option<Vec<Order_>>,
        limit: Option<usize>,
    ) -> Result<Arc<Vec<_@{ pascal_name }@Cache>>> {
        unimplemented!("cache_update_only feature disables fetching from cache.")
    }
    #[cfg(not(feature="cache_update_only"))]
    pub@{ visibility }@ async fn find_all_from_cache(
        conn: &DbConn,
        filter: Option<Filter_>,
        order: Option<Vec<Order_>>,
        limit: Option<usize>,
    ) -> Result<Arc<Vec<_@{ pascal_name }@Cache>>> {
        let shard_id = conn.shard_id();
        if let Some(arc) = CACHE_ALL.get().unwrap()[shard_id as usize].load_full() {
            return Ok(arc);
        }
        let _guard = BULK_FETCH_SEMAPHORE.get().unwrap()[shard_id as usize].acquire().await?;
        if let Some(arc) = CACHE_ALL.get().unwrap()[shard_id as usize].load_full() {
            return Ok(arc);
        }
        let mut conn = DbConn::_new(shard_id);
        conn.begin_cache_tx().await?;
        let mut sql = format!(
            r#"SELECT {} FROM @{ table_name|db_esc }@ as _t1 {} {}"#,
            CacheData::_sql_cols(),
            Filter_::write_where(
                &filter,
                TrashMode::Not,
                TRASHED_SQL,
                NOT_TRASHED_SQL,
                ONLY_TRASHED_SQL
            ),
            Order_::write_order(&order)
        );
        if let Some(limit) = limit {
            write!(sql, " limit {}", limit)?;
        }
        let mut query = sqlx::query_as::<_, CacheData>(&sql);
        let _span = debug_span!("query", sql = &query.sql());
        if let Some(c) = filter {
            query = c.query_as_bind(query);
        }
        let result = crate::misc::fetch!(conn, query, fetch_all);
        let time = MSec::now();
        let mut list: Vec<CacheWrapper> = result.into_iter().map(|data| CacheWrapper::_from_inner(data, shard_id, time)).collect();
        @{- def.relations_in_cache()|fmt_rel_join("\n        CacheWrapper::fetch_{raw_rel_name}_for_vec(&mut list, &mut conn).await?;", "") }@
        let list: Vec<_@{ pascal_name }@Cache> = list.into_iter().map(|v| Arc::new(v).into()).collect();
        let arc = Arc::new(list);
        let sync = CACHE_RESET_SYNC_ALL.get().unwrap()[shard_id as usize].lock().await;
        if *sync <= conn.cache_sync() {
            CACHE_ALL.get().unwrap()[shard_id as usize].swap(Some(Arc::clone(&arc)));
        }
        Ok(arc)
    }
@%- else %@

    #[cfg(feature="cache_update_only")]
    pub@{ visibility }@ async fn find_all_from_cache(
        conn: &DbConn,
        order: Option<Vec<Order_>>,
    ) -> Result<Arc<Vec<_@{ pascal_name }@Cache>>> {
        unimplemented!("cache_update_only feature disables fetching from cache.")
    }
    #[cfg(not(feature="cache_update_only"))]
    pub@{ visibility }@ async fn find_all_from_cache(
        conn: &DbConn,
        order: Option<Vec<Order_>>,
    ) -> Result<Arc<Vec<_@{ pascal_name }@Cache>>> {
        let shard_id = conn.shard_id();
        if let Some(arc) = CACHE_ALL.get().unwrap()[shard_id as usize].load_full() {
            return Ok(arc);
        }
        let _guard = BULK_FETCH_SEMAPHORE.get().unwrap()[shard_id as usize].acquire().await?;
        if let Some(arc) = CACHE_ALL.get().unwrap()[shard_id as usize].load_full() {
            return Ok(arc);
        }
        let mut conn = DbConn::_new(shard_id);
        conn.begin_cache_tx().await?;
        let mut sql = format!(
            r#"SELECT {} FROM @{ table_name|db_esc }@ as _t1 {} {}"#,
            CacheData::_sql_cols(),
            Filter_::write_where(
                &None,
                TrashMode::Not,
                TRASHED_SQL,
                NOT_TRASHED_SQL,
                ONLY_TRASHED_SQL
            ),
            Order_::write_order(&order)
        );
        let mut query = sqlx::query_as::<_, CacheData>(&sql);
        let _span = debug_span!("query", sql = &query.sql());
        let result = crate::misc::fetch!(conn, query, fetch_all);
        let time = MSec::now();
        #[allow(clippy::needless_collect)]
        let mut list: Vec<CacheWrapper> = result.into_iter().map(|data| CacheWrapper::_from_inner(data, shard_id, time)).collect();
        @{- def.relations_in_cache()|fmt_rel_join("\n        CacheWrapper::fetch_{raw_rel_name}_for_vec(&mut list, &mut conn).await?;", "") }@
        let list: Vec<_@{ pascal_name }@Cache> = list.into_iter().map(|v| Arc::new(v).into()).collect();
        let arc = Arc::new(list);
        let sync = CACHE_RESET_SYNC_ALL.get().unwrap()[shard_id as usize].lock().await;
        if *sync <= conn.cache_sync() {
            CACHE_ALL.get().unwrap()[shard_id as usize].swap(Some(Arc::clone(&arc)));
        }
        Ok(arc)
    }
@%- endif %@
@%- endif %@

    pub@{ visibility }@ fn query() -> QueryBuilder {
        QueryBuilder::default()
    }
    @%- if !config.force_disable_cache %@

    pub@{ visibility }@ async fn clear_cache() -> Result<()> {
        @%- if def.act_as_job_queue() %@
        @%- else if def.use_clear_whole_cache() %@
        let sync_map = DbConn::inc_all_cache_sync().await?;
        CacheMsg(vec![crate::CacheOp::_AllClear], sync_map)
            .do_send()
            .await;
        @%- else %@
        let sync_map = DbConn::inc_all_cache_sync().await?;
        CacheMsg(vec![CacheOp::InvalidateAll.wrap()], sync_map)
            .do_send()
            .await;
        @%- endif %@
        Ok(())
    }
    @%- endif %@

    async fn __find_many(
        conn: &mut DbConn,
        ids: &[InnerPrimary],
        with_trashed: bool,
    ) -> Result<Vec<Data>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let id_chunks = ids.chunks(IN_CONDITION_LIMIT);
        let mut list = Vec::with_capacity(ids.len());
        for ids in id_chunks {
            @%- if def.dummy_always_present() %@
            let mut v = ids
                .iter()
                .map(|id| {
                    let id = id.clone();
                    Data {
                        @{- def.primaries()|fmt_join("
                        {var}: id.{index}{raw_to_inner},", "") }@
                        ..Default::default()
                    }
                })
                .collect();
            @%- else %@
            let mut v = Self::___find_many(conn, ids, with_trashed).await?;
            @%- endif %@
            list.append(&mut v);
        }
        Ok(list)
    }
@% if def.not_optimized_tuple() %@
    fn check_id(id: impl std::fmt::Display) -> Result<String> {
        let id = id.to_string();
        for c in id.as_bytes().iter() {
            if *c == '\\' as u32 as u8
                || *c == '\'' as u32 as u8
                || *c == '"' as u32 as u8
                || *c < '!' as u32 as u8
                || *c > '~' as u32 as u8
            {
                return Err(anyhow!("invalid id!"));
            }
        }
        Ok(id)
    }

    async fn ___find_many<T>(conn: &mut DbConn, ids: &[InnerPrimary], with_trashed: bool) -> Result<Vec<T>>
    where
        T: for<'r> sqlx::FromRow<'r, <DbType as sqlx::Database>::Row> + SqlColumns + Send + Sync + Unpin,
    {
        use futures::TryStreamExt;
        use sqlx::Executor;
        let mut sql = String::new();
        for id in ids {
            write!(sql, 
                r#"SELECT {} FROM @{ table_name|db_esc }@ WHERE {}@{ def.primaries()|fmt_join("{col_esc}='{}'", " AND ") }@;"#,
                T::_sql_cols(),
                if with_trashed { TRASHED_SQL } else { NOT_TRASHED_SQL },
                @{ def.primaries()|fmt_join("check_id(id.{index})?", ", ") }@
            )?;
        }
        let mut list = Vec::new();
        if conn.has_tx() {
            let mut stream = conn.get_tx().await?.as_mut().fetch_many(&*sql);
            while let Some(result) = stream.try_next().await? {
                if let Some(row) = result.right() {
                    list.push(T::from_row(&row)?);
                }
            }
        } else if conn.has_read_tx() {
            let mut stream = conn.get_read_tx().await?.fetch_many(&*sql);
            while let Some(result) = stream.try_next().await? {
                if let Some(row) = result.right() {
                    list.push(T::from_row(&row)?);
                }
            }
        } else {
            let replica = conn.get_replica_conn().await?;
            let mut stream = replica.fetch_many(&*sql);
            while let Some(result) = stream.try_next().await? {
                if let Some(row) = result.right() {
                    list.push(T::from_row(&row)?);
                }
            }
        };
        Ok(list)
    }
@%- else %@
    #[allow(clippy::needless_borrow)]
    async fn ___find_many<T>(conn: &mut DbConn, ids: &[InnerPrimary], with_trashed: bool) -> Result<Vec<T>>
    where
        T: for<'r> sqlx::FromRow<'r, <DbType as sqlx::Database>::Row> + SqlColumns + Send + Sync + Unpin,
    {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let q = "@{ def.primaries()|fmt_join_with_paren("{placeholder}", ",") }@,".repeat(ids.len());
        let sql = format!(
            r#"SELECT {} FROM @{ table_name|db_esc }@ WHERE {}@{ def.primaries()|fmt_join_with_paren("{col_esc}", ",") }@ in ({});"#,
            T::_sql_cols(),
            if with_trashed { TRASHED_SQL } else { NOT_TRASHED_SQL },
            &q[0..q.len() - 1]
        );
        let mut query = sqlx::query_as::<_, T>(&sql);
        let _span = debug_span!("query", sql = &query.sql());
        for id in ids {
            @{- def.primaries()|fmt_join("
            query = query.bind(id.{index}{bind_as});", "") }@
        }
        let result = crate::misc::fetch!(conn, query, fetch_all);
        Ok(result)
    }
@%- endif %@
@%- if def.use_cache() %@

    #[cfg(not(feature="cache_update_only"))]
    pub(crate) async fn __find_many_for_cache<I, T>(conn: &mut DbConn, ids: I) -> Result<FxHashMap<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@, _@{ pascal_name }@Cache>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Primary>,
    {
        let ids: Vec<InnerPrimary> = ids.into_iter().map(|id| (&id.into()).into()).collect();
        let list = Self::___find_many_for_cache(conn, &ids).await?;
        let map = list.into_iter()@{- def.soft_delete_tpl("",".filter(|data| data._inner.deleted_at.is_none())",".filter(|data| data._inner.deleted == 0)")}@.fold(FxHashMap::default(), |mut map, v| {
            map.insert(@{ def.primaries()|fmt_join_with_paren("v._inner.{var}{clone}.into()", ", ") }@, Arc::new(v).into());
            map
        });
        Ok(map)
    }

    #[cfg(not(feature="cache_update_only"))]
    async fn ___find_many_for_cache(
        conn: &mut DbConn,
        ids: &[InnerPrimary],
    ) -> Result<Vec<CacheWrapper>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let id_chunks = ids.chunks(IN_CONDITION_LIMIT);
        let mut result: Vec<CacheData> = Vec::with_capacity(ids.len());
        for ids in id_chunks {
            @%- if def.dummy_always_present() %@
            let mut v = ids
                .iter()
                .map(|id| {
                    let id = id.clone();
                    CacheData {
                        @{- def.primaries()|fmt_join("
                        {var}: id.{index}{raw_to_inner},", "") }@
                        ..Default::default()
                    }
                })
                .collect();
            @%- else %@
            let mut v = Self::___find_many(conn, ids, true).await?;
            @%- endif %@
            result.append(&mut v);
        }
        let time = MSec::now();
        let list = result.into_iter().map(|v| CacheWrapper::_from_inner(v, conn.shard_id(), time)).collect();
        Ok(list)
    }
    @%- endif %@

    pub@{ visibility }@ async fn find<T>(conn: &mut DbConn, id: T, filter: Option<Filter_>) -> Result<_@{ pascal_name }@>
    where
        T: Into<Primary>,
    {
        let id: Primary = id.into();
        Self::find_optional(conn, id.clone(), filter)
            .await?
            .with_context(|| err::RowNotFound::new("@{ table_name }@", id_to_string(&(&id).into())))
    }
    @%- if def.is_soft_delete() %@

    pub@{ visibility }@ async fn find_with_trashed<T>(conn: &mut DbConn, id: T, filter: Option<Filter_>) -> Result<_@{ pascal_name }@>
    where
        T: Into<Primary>,
    {
        let id: Primary = id.into();
        Self::find_optional_with_trashed(conn, id.clone(), filter)
            .await?
            .with_context(|| err::RowNotFound::new("@{ table_name }@", id_to_string(&(&id).into())))
    }
    @%- endif %@
    @%- if def.use_cache() %@

    #[cfg(feature="cache_update_only")]
    pub@{ visibility }@ async fn find_from_cache<T>(conn: &DbConn, id: T) -> Result<_@{ pascal_name }@Cache>
    where
        T: Into<Primary>,
    {
        unimplemented!("cache_update_only feature disables fetching from cache.")
    }
    #[cfg(not(feature="cache_update_only"))]
    pub@{ visibility }@ async fn find_from_cache<T>(conn: &DbConn, id: T) -> Result<_@{ pascal_name }@Cache>
    where
        T: Into<Primary>,
    {
        let id: InnerPrimary = (&id.into()).into();
        Self::__find_optional_from_cache(conn, id.clone())
            .await?
@{- def.soft_delete_tpl("","
            .filter(|data| data._wrapper._inner.deleted_at.is_none())","
            .filter(|data| data._wrapper._inner.deleted == 0)")}@
            .with_context(|| err::RowNotFound::new("@{ table_name }@", id_to_string(&id)))
    }
    @%- if def.is_soft_delete() %@

    #[cfg(feature="cache_update_only")]
    pub@{ visibility }@ async fn find_from_cache_with_trashed<T>(conn: &DbConn, id: T) -> Result<_@{ pascal_name }@Cache>
    where
        T: Into<Primary>,
    {
        unimplemented!("cache_update_only feature disables fetching from cache.")
    }
    #[cfg(not(feature="cache_update_only"))]
    pub@{ visibility }@ async fn find_from_cache_with_trashed<T>(conn: &DbConn, id: T) -> Result<_@{ pascal_name }@Cache>
    where
        T: Into<Primary>,
    {
        let id: InnerPrimary = (&id.into()).into();
        Self::__find_optional_from_cache(conn, id.clone())
            .await?
            .with_context(|| err::RowNotFound::new("@{ table_name }@", id_to_string(&id)))
    }
    @%- endif %@
    @%- endif %@

    pub@{ visibility }@ fn list_to_map(list: Vec<_@{ pascal_name }@>) -> AHashMap<Primary, _@{ pascal_name }@> {
        let mut map = AHashMap::default();
        for v in list {
            map.insert((&v).into(), v);
        }
        map
    }
    @%- if !config.force_disable_cache %@

    pub@{ visibility }@ fn cache_list_to_map(list: Vec<_@{ pascal_name }@Cache>) -> AHashMap<Primary, _@{ pascal_name }@Cache> {
        let mut map = AHashMap::default();
        for v in list {
            map.insert((&v).into(), v);
        }
        map
    }
    @%- endif %@

    pub@{ visibility }@ fn updater_list_to_map(list: Vec<_@{ pascal_name }@Updater>) -> AHashMap<Primary, _@{ pascal_name }@Updater> {
        let mut map = AHashMap::default();
        for v in list {
            map.insert((&v).into(), v);
        }
        map
    }

    pub@{ visibility }@ async fn find_many<I, T>(conn: &mut DbConn, ids: I) -> Result<Vec<_@{ pascal_name }@>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Primary>,
    {
        let ids: Vec<InnerPrimary> = ids.into_iter().map(|id| (&id.into()).into()).collect();
        Ok(Self::__find_many(conn, &ids, false).await?.into_iter().map(|v| v.into()).collect())
    }
    @%- if def.is_soft_delete() %@

    pub@{ visibility }@ async fn find_many_with_trashed<I, T>(conn: &mut DbConn, ids: I) -> Result<Vec<_@{ pascal_name }@>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Primary>,
    {
        let ids: Vec<InnerPrimary> = ids.into_iter().map(|id| (&id.into()).into()).collect();
        Ok(Self::__find_many(conn, &ids, true).await?.into_iter().map(|v| v.into()).collect())
    }
    @%- endif %@
    @%- if def.use_cache() %@

    #[cfg(feature="cache_update_only")]
    pub@{ visibility }@ async fn find_many_from_cache<I, T>(conn: &DbConn, ids: I) -> Result<Vec<_@{ pascal_name }@Cache>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Primary>,
    {
        unimplemented!("cache_update_only feature disables fetching from cache.")
    }
    #[cfg(not(feature="cache_update_only"))]
    pub@{ visibility }@ async fn find_many_from_cache<I, T>(conn: &DbConn, ids: I) -> Result<Vec<_@{ pascal_name }@Cache>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Primary>,
    {
        let ids: Vec<InnerPrimary> = ids.into_iter().map(|id| (&id.into()).into()).collect();
        Ok(Self::__find_many_from_cache(conn, ids).await?.into_iter()@{ def.soft_delete_tpl("",".filter(|data| data._wrapper._inner.deleted_at.is_none())",".filter(|data| data._wrapper._inner.deleted == 0)")}@.collect())
    }
    @%- if def.is_soft_delete() %@

    #[cfg(feature="cache_update_only")]
    pub@{ visibility }@ async fn find_many_from_cache_with_trashed<I, T>(conn: &DbConn, ids: I) -> Result<Vec<_@{ pascal_name }@Cache>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Primary>,
    {
        unimplemented!("cache_update_only feature disables fetching from cache.")
    }
    #[cfg(not(feature="cache_update_only"))]
    pub@{ visibility }@ async fn find_many_from_cache_with_trashed<I, T>(conn: &DbConn, ids: I) -> Result<Vec<_@{ pascal_name }@Cache>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Primary>,
    {
        let ids: Vec<InnerPrimary> = ids.into_iter().map(|id| (&id.into()).into()).collect();
        Self::__find_many_from_cache(conn, ids).await
    }
    @%- endif %@

    #[cfg(not(feature="cache_update_only"))]
    async fn __find_many_from_cache<I, T>(conn: &DbConn, ids: I) -> Result<Vec<_@{ pascal_name }@Cache>>
    where
        I: IntoIterator<Item = T>,
        T: Into<InnerPrimary>,
    {
        let shard_id = conn.shard_id();
        let ids: Vec<_> = ids.into_iter().map(|id| PrimaryHasher(id.into(), shard_id)).collect();
        Self::___find_many_from_cache(conn, ids).await
    }
    #[cfg(not(feature="cache_update_only"))]
    #[allow(clippy::collapsible_if)]
    async fn ___find_many_from_cache(conn: &DbConn, ids: Vec<PrimaryHasher>) -> Result<Vec<_@{ pascal_name }@Cache>> {
        let mut list: Vec<_@{ pascal_name }@Cache> = Vec::new();
        let mut rest_ids = Vec::new();
        let shard_id = conn.shard_id();
        let mut conn = DbConn::_new(shard_id);
        let cache_map = Cache::get_many::<CacheWrapper>(&ids.iter().map(|id| id.hash_val(shard_id)).collect(), shard_id, USE_FAST_CACHE).await;
        for id in ids {
            if let Some(obj) = cache_map.get(&id.hash_val(shard_id)).filter(|o| InnerPrimary::from(*o) == id.0) {
                list.push(obj.clone().into());
            } else {
                rest_ids.push(id);
            }
        }
        if !rest_ids.is_empty() {
            for id in rest_ids.iter() {
                BULK_FETCH_QUEUE.get().unwrap()[shard_id as usize].push(id.0.clone());
            }
            let _guard = BULK_FETCH_SEMAPHORE.get().unwrap()[shard_id as usize].acquire().await?;
            let mut rest_ids2 = FxHashSet::with_capacity_and_hasher(rest_ids.len(), Default::default());
            for id in rest_ids.into_iter() {
                if let Some(obj) = Cache::get_from_memory::<CacheWrapper>(&id, shard_id, USE_FAST_CACHE).await.filter(|o| InnerPrimary::from(o) == id.0) {
                    list.push(obj.into());
                } else {
                    rest_ids2.insert(id);
                }
            }
            if !rest_ids2.is_empty() {
                conn.begin_cache_tx().await?;
                let mut ids = FxHashSet::with_capacity_and_hasher(
                    BULK_FETCH_QUEUE.get().unwrap()[shard_id as usize].len() + rest_ids2.len(),
                    Default::default()
                );
                for id in &rest_ids2 {
                    ids.insert(id.0.clone());
                }
                while let Some(x) = BULK_FETCH_QUEUE.get().unwrap()[shard_id as usize].pop() {
                    ids.insert(x);
                }
                let ids: Vec<InnerPrimary> = ids.drain().collect();
                #[allow(unused_mut)]
                let mut result = Self::___find_many_for_cache(&mut conn, &ids).await?;
@{- def.relations_in_cache()|fmt_rel_join("\n                CacheWrapper::fetch_{raw_rel_name}_for_vec(&mut result, &mut conn).await?;", "") }@
                let _lock = crate::models::CACHE_UPDATE_LOCK.read().await;
                for v in result.into_iter() {
                    let arc = Arc::new(v);
                    let id = PrimaryHasher(InnerPrimary::from(&arc), shard_id);
                    let sync = CACHE_RESET_SYNC.get().unwrap()[shard_id as usize].read().await;
                    if *sync <= conn.cache_sync() {
                        if Cache::get_from_memory::<CacheWrapper>(&id, shard_id, USE_FAST_CACHE).await.filter(|o| InnerPrimary::from(o) == id.0).is_none() {
                            @%- if def.versioned %@
                            let vw = VersionWrapper {
                                id: id.0.clone(),
                                shard_id,
                                time: MSec::default(),
                                version: 0,
                            };
                            if let Some(ver) = Cache::get_version::<VersionWrapper>(&vw, shard_id).await.filter(|o| o.id == id.0) {
                                if arc._inner.@{ version_col }@.greater_equal(ver.version) {
                                    Cache::insert_long(&id, arc.clone(), USE_FAST_CACHE).await;
                                }
                            } else {
                                let cs = CacheSyncWrapper {
                                    id: id.0.clone(),
                                    shard_id,
                                    time: MSec::default(),
                                    sync: 0,
                                };
                                if let Some(cs) = Cache::get_version::<CacheSyncWrapper>(&cs, shard_id).await.filter(|o| o.id == id.0) {
                                    if cs.sync <= conn.cache_sync() {
                                        Cache::insert_long(&id, arc.clone(), USE_FAST_CACHE).await;
                                    }
                                } else {
                                    Cache::insert_long(&id, arc.clone(), USE_FAST_CACHE).await;
                                }
                            }
                            @%- else %@
                            let cs = CacheSyncWrapper {
                                id: id.0.clone(),
                                shard_id,
                                time: MSec::default(),
                                sync: 0,
                            };
                            if let Some(cs) = Cache::get_version::<CacheSyncWrapper>(&cs, shard_id).await.filter(|o| o.id == id.0) {
                                if cs.sync <= conn.cache_sync() {
                                    Cache::insert_long(&id, arc.clone(), USE_FAST_CACHE).await;
                                }
                            } else {
                                Cache::insert_long(&id, arc.clone(), USE_FAST_CACHE).await;
                            }
                            @%- endif %@
                        }
                    }
                    if rest_ids2.contains(&id) {
                        list.push(arc.into());
                    }
                }
            }
        }
        conn.release_cache_tx();
        Ok(list)
    }
    @%- else %@
    @%- if !config.force_disable_cache %@

    pub@{ visibility }@ async fn find_many_from_cache<I, T>(conn: &DbConn, ids: I) -> Result<Vec<_@{ pascal_name }@Cache>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Primary>,
    {
        unimplemented!("@{ table_name }@ does not support caching.")
    }
    @%- if def.is_soft_delete() %@

    pub@{ visibility }@ async fn find_many_from_cache_with_trashed<I, T>(conn: &DbConn, ids: I) -> Result<Vec<_@{ pascal_name }@Cache>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Primary>,
    {
        unimplemented!("@{ table_name }@ does not support caching.")
    }
    @%- endif %@
    @%- endif %@
    @%- endif %@

    pub@{ visibility }@ async fn find_optional<T>(conn: &mut DbConn, id: T, filter: Option<Filter_>) -> Result<Option<_@{ pascal_name }@>>
    where
        T: Into<Primary>,
    {
        let id: InnerPrimary = (&id.into()).into();
        @%- if def.dummy_always_present() %@
        let data: Option<Data> = Some(Data {
            @{- def.primaries()|fmt_join("
            {var}: id.{index}{raw_to_inner},", "") }@
            ..Default::default()
        });
        @%- else %@
        let data: Option<Data> = Self::__find_optional(conn, id, TrashMode::Not, filter).await?;
        @%- endif %@
        Ok(data.map(_@{ pascal_name }@::from))
    }
    @%- if !def.disable_update() %@

    pub@{ visibility }@ async fn find_optional_for_update<T>(conn: &mut DbConn, id: T, filter: Option<Filter_>) -> Result<Option<_@{ pascal_name }@Updater>>
    where
        T: Into<Primary>,
    {
        let id: InnerPrimary = (&id.into()).into();
        let result = Self::__find_for_update(conn, &id, TrashMode::Not, filter).await?;
        Ok(result.map(_Updater_::from))
    }
    @%- endif %@
    @%- if def.is_soft_delete() %@

    pub@{ visibility }@ async fn find_optional_with_trashed<T>(conn: &mut DbConn, id: T, filter: Option<Filter_>) -> Result<Option<_@{ pascal_name }@>>
    where
        T: Into<Primary>,
    {
        let id: InnerPrimary = (&id.into()).into();
        @%- if def.dummy_always_present() %@
        let data: Option<Data> = Some(Data {
            @{- def.primaries()|fmt_join("
            {var}: id.{index}{raw_to_inner},", "") }@
            ..Default::default()
        });
        @%- else %@
        let data: Option<Data> = Self::__find_optional(conn, id, TrashMode::With, filter).await?;
        @%- endif %@
        Ok(data.map(_@{ pascal_name }@::from))
    }
    @%- endif %@
    @%- if def.use_cache() %@

    // #[cfg(not(feature="cache_update_only"))]
    // pub(crate) async fn __find_optional_for_cache<T>(conn: &mut DbConn, id: T) -> Result<Option<_@{ pascal_name }@Cache>>
    // where
    //     T: Into<Primary>,
    // {
    //     let id: InnerPrimary = (&id.into()).into();
    //     let data: Option<CacheData> = Self::__find_optional(conn, id, TrashMode::With, None).await?;
    //     Ok(data.map(|v| Arc::new(CacheWrapper::_from_inner(v, conn.shard_id(), MSec::now())).into()))
    // }
    @%- endif %@

    #[allow(clippy::needless_borrow)]
    async fn __find_optional<T>(conn: &mut DbConn, id: InnerPrimary, trash_mode: TrashMode, filter: Option<Filter_>) -> Result<Option<T>>
    where
        T: for<'r> sqlx::FromRow<'r, <DbType as sqlx::Database>::Row> + SqlColumns + Send + Sync + Unpin,
    {
        let mut filter_str = Filter_::write_where(&filter, trash_mode, TRASHED_SQL, NOT_TRASHED_SQL, ONLY_TRASHED_SQL);
        if filter_str.is_empty() {
            filter_str = "WHERE".to_string();
        } else {
            filter_str.push_str(" AND ");
        }
        let sql = format!(r#"SELECT {} FROM @{ table_name|db_esc }@ as _t1 {filter_str} @{ def.inheritance_cond(" AND ") }@@{ def.primaries()|fmt_join("{col_esc}={placeholder}", " AND ") }@"#, T::_sql_cols());
        let mut query = sqlx::query_as::<_, T>(&sql);
        let _span = debug_span!("query", sql = &query.sql());
        if let Some(c) = filter {
            query = c.query_as_bind(query);
        }
        @{- def.primaries()|fmt_join("
        query = query.bind(id.{index}{bind_as});", "") }@
        Ok(crate::misc::fetch!(conn, query, fetch_optional))
    }
    @%- if def.use_cache() %@

    #[cfg(feature="cache_update_only")]
    #[allow(clippy::needless_question_mark)]
    pub@{ visibility }@ async fn find_optional_from_cache<T>(conn: &DbConn, id: T) -> Result<Option<_@{ pascal_name }@Cache>>
    where
        T: Into<Primary>,
    {
        unimplemented!("cache_update_only feature disables fetching from cache.")
    }
    #[cfg(not(feature="cache_update_only"))]
    #[allow(clippy::needless_question_mark)]
    pub@{ visibility }@ async fn find_optional_from_cache<T>(conn: &DbConn, id: T) -> Result<Option<_@{ pascal_name }@Cache>>
    where
        T: Into<Primary>,
    {
        let id: InnerPrimary = (&id.into()).into();
        Ok(Self::__find_optional_from_cache(conn, id).await?@{- def.soft_delete_tpl("",".filter(|data| data._wrapper._inner.deleted_at.is_none())",".filter(|data| data._wrapper._inner.deleted == 0)")}@)
    }
    @%- if def.is_soft_delete() %@

    #[cfg(feature="cache_update_only")]
    pub@{ visibility }@ async fn find_optional_from_cache_with_trashed<T>(conn: &DbConn, id: T) -> Result<Option<_@{ pascal_name }@Cache>>
    where
        T: Into<Primary>,
    {
        unimplemented!("cache_update_only feature disables fetching from cache.")
    }
    #[cfg(not(feature="cache_update_only"))]
    pub@{ visibility }@ async fn find_optional_from_cache_with_trashed<T>(conn: &DbConn, id: T) -> Result<Option<_@{ pascal_name }@Cache>>
    where
        T: Into<Primary>,
    {
        let id: InnerPrimary = (&id.into()).into();
        Self::__find_optional_from_cache(conn, id).await
    }
    @%- endif %@

    #[cfg(not(feature="cache_update_only"))]
    async fn __find_optional_from_cache<T>(conn: &DbConn, id: T) -> Result<Option<_@{ pascal_name }@Cache>>
    where
        T: Into<InnerPrimary>,
    {
        let id: InnerPrimary = id.into();
        let mut result = Self::__find_many_from_cache(conn, [id]).await?;
        Ok(result.pop())
    }
    @%- else %@
    @%- if !config.force_disable_cache %@

    pub(crate) async fn find_optional_from_cache<T>(conn: &DbConn, id: T) -> Result<Option<_@{ pascal_name }@Cache>>
    where
        T: Into<Primary>,
    {
        unimplemented!("@{ table_name }@ does not support caching.")
    }
    @%- if def.is_soft_delete() %@

    pub@{ visibility }@ async fn find_optional_from_cache_with_trashed<T>(conn: &DbConn, id: T) -> Result<Option<_@{ pascal_name }@Cache>>
    where
        T: Into<Primary>,
    {
        unimplemented!("@{ table_name }@ does not support caching.")
    }
    @%- endif %@
    @%- endif %@
    @%- endif %@
    @%- if !def.disable_update() %@

    pub@{ visibility }@ async fn find_for_update<T>(conn: &mut DbConn, id: T, filter: Option<Filter_>) -> Result<_@{ pascal_name }@Updater>
    where
        T: Into<Primary>,
    {
        let id: InnerPrimary = (&id.into()).into();
        let result = Self::__find_for_update(conn, &id, TrashMode::Not, filter).await?@{ def.soft_delete_tpl("",".filter(|data| data.deleted_at.is_none())",".filter(|data| data.deleted == 0)")}@;
        let data = result.with_context(|| err::RowNotFound::new("@{ table_name }@", id.to_string()))?;
        Ok(_Updater_::from(data))
    }
    @%- endif %@
    @%- if !def.disable_update() && def.is_soft_delete() %@

    pub@{ visibility }@ async fn find_for_update_with_trashed<T>(conn: &mut DbConn, id: T, filter: Option<Filter_>) -> Result<_@{ pascal_name }@Updater>
    where
        T: Into<Primary>,
    {
        let id: InnerPrimary = (&id.into()).into();
        let result = Self::__find_for_update(conn, &id, TrashMode::With, filter).await?;
        let data = result.with_context(|| err::RowNotFound::new("@{ table_name }@", id.to_string()))?;
        Ok(_Updater_::from(data))
    }
    @%- endif %@
@%- if !def.disable_update() %@

    #[allow(clippy::needless_borrow)]
    async fn __find_for_update(conn: &mut DbConn, id: &InnerPrimary, trash_mode: TrashMode, filter: Option<Filter_>) -> Result<Option<Data>> {
        let mut filter_str = Filter_::write_where(&filter, trash_mode, TRASHED_SQL, NOT_TRASHED_SQL, ONLY_TRASHED_SQL);
        if filter_str.is_empty() {
            filter_str = "WHERE".to_string();
        } else {
            filter_str.push_str(" AND ");
        }
        let sql = format!(r#"SELECT {} FROM @{ table_name|db_esc }@ as _t1 {filter_str} @{ def.inheritance_cond(" AND ") }@@{ def.primaries()|fmt_join("{col_esc}={placeholder}", " AND ") }@ FOR UPDATE"#, Data::_sql_cols());
        let mut query = sqlx::query_as::<_, Data>(&sql);
        let _span = debug_span!("query", sql = &query.sql());
        if let Some(c) = filter {
            query = c.query_as_bind(query);
        }
        @{- def.primaries()|fmt_join("
        query = query.bind(id.{index}{bind_as});", "") }@
        if conn.wo_tx() {
            Ok(query.fetch_optional(conn.acquire_source().await?.as_mut()).await?)
        } else {
            Ok(query.fetch_optional(conn.get_tx().await?.as_mut()).await?)
        }
    }

    pub@{ visibility }@ async fn find_many_for_update<I, T>(conn: &mut DbConn, ids: I) -> Result<Vec<_@{ pascal_name }@Updater>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Primary>,
    {
        Self::__find_many_for_update(conn, ids, false).await
    }

    pub@{ visibility }@ async fn find_many_for_update_with_trashed<I, T>(conn: &mut DbConn, ids: I) -> Result<Vec<_@{ pascal_name }@Updater>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Primary>,
    {
        Self::__find_many_for_update(conn, ids, true).await
    }

    #[allow(clippy::needless_borrow)]
    async fn __find_many_for_update<I, T>(conn: &mut DbConn, ids: I, with_trashed: bool) -> Result<Vec<_@{ pascal_name }@Updater>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Primary>,
    {
        let ids: Vec<InnerPrimary> = ids.into_iter().map(|id| (&id.into()).into()).collect();
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let mut list: Vec<_Updater_> = Vec::with_capacity(ids.len());
        let id_chunks = ids.chunks(IN_CONDITION_LIMIT);
        for ids in id_chunks {
            let q = "@{ def.primaries()|fmt_join_with_paren("{placeholder}", ",") }@,".repeat(ids.len());
            let sql = format!(
                r#"SELECT {} FROM @{ table_name|db_esc }@ WHERE {}@{ def.primaries()|fmt_join_with_paren("{col_esc}", ",") }@ in ({}) FOR UPDATE;"#,
                Data::_sql_cols(),
                if with_trashed { TRASHED_SQL } else { NOT_TRASHED_SQL },
                &q[0..q.len() - 1],
            );
            let mut query = sqlx::query_as::<_, Data>(&sql);
            let _span = debug_span!("query", sql = &query.sql());
            for id in ids {
                @{- def.primaries()|fmt_join("
                query = query.bind(id.{index}{bind_as});", "") }@
            }
            let result = if conn.wo_tx() {
                query.fetch_all(conn.acquire_source().await?.as_mut()).await?
            } else {
                query.fetch_all(conn.get_tx().await?.as_mut()).await?
            };
            result
                .into_iter()
                .map(_Updater_::from)
                .for_each(|obj| list.push(obj));
        }
        Ok(list)
    }
@%- endif %@
@%- for (index_name, index) in def.unique_index() %@

    pub@{ visibility }@ async fn find_by_@{ index_name }@<@{ index.fields(index_name, def)|fmt_index_col("T{index}", ", ") }@>(conn: &mut DbConn, @{ index.fields(index_name, def)|fmt_index_col("_{name}: T{index}", ", ") }@) -> Result<_@{ pascal_name }@>
    where
    @{- index.fields(index_name, def)|fmt_index_col("
        T{index}: Into<{filter_type}>,", "") }@
    {
        @{- index.fields(index_name, def)|fmt_index_col("
        let val{index}: {filter_type} = _{name}.into();", "") }@
        let filter = Filter_::And(vec![@{- index.fields(index_name, def)|fmt_index_col("Filter_::EqKey(ColKey_::{var}(val{index}.clone().into()))", ", ") }@]);
        Self::query().filter(filter).select(conn).await?.pop()
            .with_context(|| err::RowNotFound::new("@{ table_name }@", format!("@{ index.fields(index_name, def)|fmt_index_col("{col_name}={}", ", ") }@", @{ index.fields(index_name, def)|fmt_index_col("val{index}", ", ") }@)))
    }
@%- endfor %@
@%- if def.use_cache() %@
@%- for (index_name, index) in def.unique_index() %@

    #[cfg(feature="cache_update_only")]
    pub@{ visibility }@ async fn find_by_@{ index_name }@_from_cache<@{ index.fields(index_name, def)|fmt_index_col("T{index}", ", ") }@>(conn: &DbConn, @{ index.fields(index_name, def)|fmt_index_col("_{name}: T{index}", ", ") }@) -> Result<_@{ pascal_name }@Cache>
    where
    @{- index.fields(index_name, def)|fmt_index_col("
        T{index}: Into<{filter_type}>,", "") }@
    {
        unimplemented!("cache_update_only feature disables fetching from cache.")
    }
    #[cfg(not(feature="cache_update_only"))]
    pub@{ visibility }@ async fn find_by_@{ index_name }@_from_cache<@{ index.fields(index_name, def)|fmt_index_col("T{index}", ", ") }@>(conn: &DbConn, @{ index.fields(index_name, def)|fmt_index_col("_{name}: T{index}", ", ") }@) -> Result<_@{ pascal_name }@Cache>
    where
    @{- index.fields(index_name, def)|fmt_index_col("
        T{index}: Into<{filter_type}>,", "") }@
    {
        @{- index.fields(index_name, def)|fmt_index_col("
        let val{index}: {filter_type} = _{name}.into();", "") }@
        let key = VecColKey(vec![@{- index.fields(index_name, def)|fmt_index_col("ColKey_::{var}(val{index}.clone().into())", ", ") }@]);
        if let Some(id) = Cache::get::<PrimaryWrapper>(&key, conn.shard_id(), true).await {
            if let Some(obj) = Self::find_optional_from_cache(conn, &id.0).await? {
                if @{ index.fields(index_name, def)|fmt_index_col_not_null_or_null("obj._{raw_var}() == val{index}", "matches!(obj._{raw_var}(), Some(v) if v == val{index})", " && ") }@ {
                    return Ok(obj);
                }
            }
        }
        let filter = Filter_::And(vec![@{- index.fields(index_name, def)|fmt_index_col("Filter_::EqKey(ColKey_::{var}(val{index}.clone().into()))", ", ") }@]);
        let mut conn = DbConn::_new(conn.shard_id());
        conn.begin_cache_tx().await?;
        let obj = Self::query().filter(filter).select_from_cache(&mut conn).await?.pop()
            .with_context(|| err::RowNotFound::new("@{ table_name }@", format!("@{ index.fields(index_name, def)|fmt_index_col("{col_name}={}", ", ") }@", @{ index.fields(index_name, def)|fmt_index_col("val{index}", ", ") }@)))?;
        let id = PrimaryWrapper(InnerPrimary::from(&obj), conn.shard_id(), MSec::now());
        Cache::insert_long(&key, Arc::new(id), true).await;
        Ok(obj)
    }
@%- endfor %@
@%- if !def.act_as_job_queue() && !def.use_clear_whole_cache() %@

    #[cfg(feature="cache_update_only")]
    pub@{ visibility }@ async fn insert_dummy_cache(conn: &DbConn, obj: _@{ pascal_name }@Updater) -> Result<()> {
        Ok(())
    }
    #[cfg(not(feature="cache_update_only"))]
    pub@{ visibility }@ async fn insert_dummy_cache(conn: &DbConn, obj: _@{ pascal_name }@Updater) -> Result<()> {
        let _lock = crate::models::CACHE_UPDATE_LOCK.write().await;
        let cache_msg = CacheOp::Insert {
            shard_id: conn.shard_id(),
            data: obj._data,
@{- def.relations_one_cache(false)|fmt_rel_join("\n            _{rel_name}: None,", "") }@
@{- def.relations_many_cache(false)|fmt_rel_join("\n            _{rel_name}: None,", "") }@
        };
        let mut sync_map = FxHashMap::default();
        sync_map.insert(conn.shard_id(), 0);
        cache_msg.handle_cache_msg(Arc::new(sync_map)).await;
        Ok(())
    }
@%- endif %@
@%- endif %@

    pub@{ visibility }@ async fn save(conn: &mut DbConn, mut obj: _@{ pascal_name }@Updater) -> Result<Option<_@{ pascal_name }@>> {
        obj.__validate()?;
        obj.__set_default_value(conn)@% if config.use_sequence %@.await?@% endif %@;
        Self::__save(conn, obj).await
    }

    async fn __save(conn: &mut DbConn, obj: _@{ pascal_name }@Updater) -> Result<Option<_@{ pascal_name }@>> {
        @%- if !config.force_disable_cache && !def.use_clear_whole_cache() && def.cache_owners.len() > 0 %@
        if !conn.clear_whole_cache {
            @{- def.cache_owners|fmt_cache_owners("
            if let Some(v) = crate::models::{mod}::RelFk{rel_name_pascal}::get_fk(&obj._data) {
                conn.push_cache_op(crate::models::{mod}::_{model_name}Cache::__invalidate_cache_op(conn, v)).await?;
            }") }@
        }
        @%- endif %@
        let (obj, cache_msg) = Self::___save(conn, obj).await?;
        @%- if !config.force_disable_cache %@
        @%- if def.act_as_job_queue() %@
        if let Some(cache_msg) = cache_msg {
            conn.push_cache_op(cache_msg.wrap()).await?;
        }
        @%- else if def.use_clear_whole_cache() %@
        conn.clear_whole_cache = true;
        @%- else %@
        if !conn.clear_whole_cache && (USE_CACHE || USE_CACHE_ALL || USE_UPDATE_NOTICE) {
            if let Some(cache_msg) = cache_msg {
                @%- if def.disable_insert_cache_propagation %@
                let internal = matches!(cache_msg, CacheOp::Insert { .. });
                conn.push_cache_op_to(cache_msg.wrap(), internal).await?;
                @%- else %@
                conn.push_cache_op(cache_msg.wrap()).await?;
                @%- endif %@
            }
        }
        @%- endif %@
        @%- endif %@
        Ok(obj)
    }

    #[allow(clippy::type_complexity)]
    pub(crate) fn ___save(
        conn: &mut DbConn,
        obj: _@{ pascal_name }@Updater,
    ) -> BoxFuture<'_, Result<(Option<_@{ pascal_name }@>, Option<CacheOp>)>>
    {
        async move {
            @%- if !def.disable_update() %@
            if obj.will_be_deleted() {
                if obj.is_new() || obj.has_been_deleted() {
                    return Ok((None, None));
                }
                let cache_msg = Self::__delete(conn, obj).await?;
                return Ok((None, cache_msg));
            }
            if obj.is_new() && obj._upsert {
                let (obj, cache_msg) = Self::__save_upsert(conn, obj).await?;
                Ok((Some(obj), cache_msg))
            } else if obj.is_new() {
                let (obj, cache_msg) = Self::__save_insert(conn, obj).await?;
                Ok((Some(obj), Some(cache_msg)))
            } else {
                let (obj, cache_msg) = Self::__save_update(conn, obj).await?;
                Ok((Some(obj), cache_msg))
            }
            @%- else %@
            if obj.is_new() {
                let (obj, cache_msg) = Self::__save_insert(conn, obj).await?;
                Ok((Some(obj), Some(cache_msg)))
            } else {
                anyhow::bail!("Update is disabled.");
            }
            @%- endif %@
        }.boxed()
    }

    #[allow(clippy::unnecessary_cast)]
    async fn __save_insert(conn: &mut DbConn, mut obj: _@{ pascal_name }@Updater) -> Result<(_@{ pascal_name }@, CacheOp)> {
        let sql = r#"INSERT INTO @{ table_name|db_esc }@ 
            (@{ def.all_fields()|fmt_join("{col_esc}", ",") }@) 
            VALUES (@{ def.all_fields()|fmt_join("{placeholder}", ",") }@)"#;
        let query = query_bind(sql, &obj._data);
        let _span = debug_span!("query", sql = &query.sql());
        let result = if conn.wo_tx() {
            query.execute(conn.acquire_source().await?.as_mut()).await?
        } else {
            query.execute(conn.get_tx().await?.as_mut()).await?
        };
@{- def.auto_inc()|fmt_join("
        if obj._data.{var} == 0 {
            obj._data.{var} = result.last_insert_id() as {inner};
        }", "") }@
        info!(target: "db_update::@{ db|snake }@::@{ group_name }@::@{ mod_name }@", op = "insert", ctx = conn.ctx_no(); "{}", &obj);
        debug!("{:?}", &obj);
        let mut obj2: _@{ pascal_name }@ = obj._data.clone().into();
        let mut update_cache = true;

        @{- def.non_primaries()|fmt_join_cache_or_not("", "
        obj._data.{var} = Default::default();", "") }@
        @%- if def.act_as_job_queue() %@
        let cache_msg = CacheOp::Queued;
        @{- def.relations_one(false)|fmt_rel_join("\n        save_{rel_name}(conn, &mut obj2, obj.{rel_name}, &mut update_cache).await?;", "") }@
        @{- def.relations_many(false)|fmt_rel_join("\n        save_{rel_name}(conn, &mut obj2, obj.{rel_name}, &mut update_cache).await?;", "") }@
        @%- else if !config.force_disable_cache && !def.use_clear_whole_cache() %@
        let cache_msg = CacheOp::Insert {
            shard_id: conn.shard_id(),
            data: obj._data,
@{- def.relations_one_cache(false)|fmt_rel_join("\n            _{rel_name}: save_{rel_name}(conn, &mut obj2, obj.{rel_name}, &mut update_cache).await?,", "") }@
@{- def.relations_many_cache(false)|fmt_rel_join("\n            _{rel_name}: save_{rel_name}(conn, &mut obj2, obj.{rel_name}, &mut update_cache).await?,", "") }@
        };
@{- def.relations_one_uncached(false)|fmt_rel_join("\n        save_{rel_name}(conn, &mut obj2, obj.{rel_name}, &mut update_cache).await?;", "") }@
@{- def.relations_many_uncached(false)|fmt_rel_join("\n        save_{rel_name}(conn, &mut obj2, obj.{rel_name}, &mut update_cache).await?;", "") }@
        @%- else %@
        let cache_msg = CacheOp::None;
        @{- def.relations_one(false)|fmt_rel_join("\n        save_{rel_name}(conn, &mut obj2, obj.{rel_name}, &mut update_cache).await?;", "") }@
        @{- def.relations_many(false)|fmt_rel_join("\n        save_{rel_name}(conn, &mut obj2, obj.{rel_name}, &mut update_cache).await?;", "") }@
        @%- endif %@
        Ok((obj2, cache_msg))
    }
    @%- if !def.disable_update() %@

    #[allow(unused_mut)]
    #[allow(unused_assignments)]
    #[allow(clippy::needless_borrow)]
    #[allow(clippy::unnecessary_cast)]
    async fn __save_update(conn: &mut DbConn, mut obj: _@{ pascal_name }@Updater) -> Result<(_@{ pascal_name }@, Option<CacheOp>)> {
        let id = InnerPrimary::from(&obj);
        let mut update_cache = false; // To distinguish from updates that do not require cache updates
        if obj.is_updated() {
            let mut vec: Vec<String> = Vec::new();
            @{- def.non_primaries_wo_read_only(false)|fmt_join_cache_or_not("
            assign_sql!(obj, vec, {var}, r#\"{col_esc}\"#, {may_null}, update_cache, \"{placeholder}\");", "
            assign_sql_no_cache_update!(obj, vec, {var}, r#\"{col_esc}\"#, {may_null}, \"{placeholder}\");", "") }@
            @%- if def.versioned %@
            vec.push(r#"\"@{ version_col }@\" = LAST_INSERT_ID(IF(\"@{ version_col }@\" < 4294967295, \"@{ version_col }@\" + 1, 0))"#.to_string());
            @%- endif %@
            @%- if def.counting.is_some() %@
            vec.push(r#"\"@{ def.get_counting_col() }@\" = LAST_INSERT_ID(\"@{ def.get_counting_col() }@\")"#.to_string());
            @%- endif %@
            @%- if def.versioned %@
            let sql = format!(r#"UPDATE @{ table_name|db_esc }@ SET {} WHERE @{ def.inheritance_cond(" AND ") }@@{ def.primaries()|fmt_join("{col_esc}={placeholder}", " AND ") }@ AND \"@{ version_col }@\"=?"#, &vec.join(","));
            @%- else %@
            let sql = format!(r#"UPDATE @{ table_name|db_esc }@ SET {} WHERE @{ def.inheritance_cond(" AND ") }@@{ def.primaries()|fmt_join("{col_esc}={placeholder}", " AND ") }@"#, &vec.join(","));
            @%- endif %@
            let mut query = sqlx::query(&sql);
            let _span = debug_span!("query", sql = &query.sql());
            @{- def.non_primaries_wo_read_only(false)|fmt_join("
            for _n in 0..obj._op.{var}.get_bind_num({may_null}) {
                query = query.bind(obj._update.{var}{bind_as});
            }","") }@
            query = query@{ def.primaries()|fmt_join(".bind(id.{index}{bind_as})", "") }@;
            @%- if def.versioned %@
            query = query.bind(&obj._data.@{ version_col }@);
            @%- endif %@
            info!(target: "db_update::@{ db|snake }@::@{ group_name }@::@{ mod_name }@", op = "update", ctx = conn.ctx_no(); "{}", &obj);
            debug!("{:?}", &obj);
            let result = if conn.wo_tx() {
                query.execute(conn.acquire_source().await?.as_mut()).await?
            } else {
                query.execute(conn.get_tx().await?.as_mut()).await?
            };
            if result.rows_affected() == 0 {
                anyhow::bail!(err::RowNotFound::new("@{ table_name }@", id.to_string()));
            }
            @%- if def.versioned %@
            obj.mut_@{ version_col }@().set(result.last_insert_id() as u32);
            @%- endif %@
            @%- if def.counting.is_some() %@
            if obj._op.@{ def.get_counting() }@ == Op::Add {
                obj._op.@{ def.get_counting() }@ = Op::Max;
                obj._update.@{ def.get_counting() }@ = result.last_insert_id().try_into().unwrap_or(@{ def.get_counting_type() }@::MAX);
            }
            @%- endif %@
        }
        let mut obj2: _@{ pascal_name }@ = obj._data.into();
        @{- def.non_primaries()|fmt_join_cache_or_not("", "
        obj._op.{var} = Op::None;
        obj._update.{var} = Default::default();", "") }@
        @%- if !config.force_disable_cache && !def.use_clear_whole_cache() && !def.act_as_job_queue() %@
        let mut cache_msg = Some(CacheOp::Update {
            id,
            shard_id: conn.shard_id(),
            update:obj._update,
            op: obj._op,
@{- def.relations_one_cache(false)|fmt_rel_join("\n            _{rel_name}: save_{rel_name}(conn, &mut obj2, obj.{rel_name}, &mut update_cache).await?,", "") }@
@{- def.relations_many_cache(false)|fmt_rel_join("\n            _{rel_name}: save_{rel_name}(conn, &mut obj2, obj.{rel_name}, &mut update_cache).await?,", "") }@
        });
        if !update_cache {
            cache_msg = None;
        }
@{- def.relations_one_uncached(false)|fmt_rel_join("\n        save_{rel_name}(conn, &mut obj2, obj.{rel_name}, &mut update_cache).await?;", "") }@
@{- def.relations_many_uncached(false)|fmt_rel_join("\n        save_{rel_name}(conn, &mut obj2, obj.{rel_name}, &mut update_cache).await?;", "") }@
        @%- else %@
        let cache_msg = None;
@{- def.relations_one(false)|fmt_rel_join("\n        save_{rel_name}(conn, &mut obj2, obj.{rel_name}, &mut update_cache).await?;", "") }@
@{- def.relations_many(false)|fmt_rel_join("\n        save_{rel_name}(conn, &mut obj2, obj.{rel_name}, &mut update_cache).await?;", "") }@
        @%- endif %@
        Ok((obj2, cache_msg))
    }
    @%- endif %@
    @%- if !def.disable_update() %@

    fn assign_non_primaries(obj: &_@{ pascal_name }@Updater) -> (Vec<String>, bool) {
        let mut update_cache = false;
        let mut vec: Vec<String> = Vec::new();
        @{- def.non_primaries()|fmt_join_cache_or_not("
        assign_sql!(obj, vec, {var}, r#\"{col_esc}\"#, {may_null}, update_cache, \"{placeholder}\");", "
        assign_sql_no_cache_update!(obj, vec, {var}, r#\"{col_esc}\"#, {may_null}, \"{placeholder}\");", "") }@
        (vec, update_cache)
    }

    fn bind_non_primaries<'a>(obj: &'a _@{ pascal_name }@Updater, mut query: Query<'a, DbType, DbArguments>, _sql: &'a str) -> Query<'a, DbType, DbArguments> {
        @{- def.non_primaries()|fmt_join("
        for _n in 0..obj._op.{var}.get_bind_num({may_null}) {
            query = query.bind(obj._update.{var}{bind_as});
        }","") }@
        query
    }

    #[allow(unused_assignments)]
    #[allow(clippy::unnecessary_cast)]
    async fn __save_upsert(conn: &mut DbConn, mut obj: _@{ pascal_name }@Updater) -> Result<(_@{ pascal_name }@, Option<CacheOp>)> {
        let (mut vec, _) = Self::assign_non_primaries(&obj);
        @%- if def.versioned %@
        vec.push(r#"\"@{ version_col }@\" = LAST_INSERT_ID(IF(\"@{ version_col }@\" < 4294967295, \"@{ version_col }@\" + 1, 0))"#.to_string());
        @%- endif %@
        @%- if def.counting.is_some() %@
        vec.push(r#"\"@{ def.get_counting_col() }@\" = LAST_INSERT_ID(\"@{ def.get_counting_col() }@\")"#.to_string());
        @%- endif %@
        let sql = format!(r#"INSERT INTO @{ table_name|db_esc }@ 
            (@{ def.all_fields()|fmt_join("{col_esc}", ",") }@) 
            VALUES (@{ def.all_fields()|fmt_join("{placeholder}", ",") }@) ON DUPLICATE KEY UPDATE {}"#, &vec.join(","));
        let query = query_bind(&sql, &obj._data);
        let _span = debug_span!("query", sql = &query.sql());
        let query = Self::bind_non_primaries(&obj, query, &sql);
        let result = if conn.wo_tx() {
            query.execute(conn.acquire_source().await?.as_mut()).await?
        } else {
            query.execute(conn.get_tx().await?.as_mut()).await?
        };
        info!(target: "db_update::@{ db|snake }@::@{ group_name }@::@{ mod_name }@", op = "upsert", ctx = conn.ctx_no(); "{}", &obj);
        debug!("{:?}", &obj);
        if result.rows_affected() == 1 {
            @{- def.auto_inc()|fmt_join("
            if obj._data.{var} == 0 {
                obj._data.{var} = result.last_insert_id() as {inner};
            }", "") }@
            let mut obj2: _@{ pascal_name }@ = obj._data.clone().into();
            @%- if !config.force_disable_cache && !def.use_clear_whole_cache() && !def.act_as_job_queue() %@
            let cache_msg = Some(CacheOp::Insert {
                shard_id: conn.shard_id(),
                data: obj._data,
@{- def.relations_one_cache(false)|fmt_rel_join("\n                _{rel_name}: None,", "") }@
@{- def.relations_many_cache(false)|fmt_rel_join("\n                _{rel_name}: None,", "") }@
            });
            @%- else %@
            let cache_msg = None;
            @%- endif %@
            Ok((obj2, cache_msg))
        } else if result.rows_affected() == 2 {
            @%- if def.versioned %@
            obj.mut_@{ version_col }@().set(result.last_insert_id() as u32);
            @%- endif %@
            @%- if def.counting.is_some() %@
            if obj._op.@{ def.get_counting() }@ == Op::Add {
                obj._op.@{ def.get_counting() }@ = Op::Max;
                obj._update.@{ def.get_counting() }@ = result.last_insert_id().try_into().unwrap_or(@{ def.get_counting_type() }@::MAX);
            }
            @%- endif %@
            let id = InnerPrimary::from(&obj);
            let mut obj2: _@{ pascal_name }@ = obj._data.into();
            @%- if !config.force_disable_cache && !def.use_clear_whole_cache() && !def.act_as_job_queue() %@
            let mut cache_msg = Some(CacheOp::Update {
                id,
                shard_id: conn.shard_id(),
                update:obj._update,
                op: obj._op,
@{- def.relations_one_cache(false)|fmt_rel_join("\n                _{rel_name}: None,", "") }@
@{- def.relations_many_cache(false)|fmt_rel_join("\n                _{rel_name}: None,", "") }@
            });
            @%- else %@
            let cache_msg = None;
            @%- endif %@
            Ok((obj2, cache_msg))
        } else {
            let mut obj2: _@{ pascal_name }@ = obj._data.into();
            Ok((obj2, None))
        }
    }
    @%- endif %@
    @%- if !def.disable_update() %@

    /// update_many ignores versions.
    pub@{ visibility }@ async fn update_many<I, T>(conn: &mut DbConn, ids: I, mut updater: _@{ pascal_name }@Updater) -> Result<u64>
    where
        I: IntoIterator<Item = T>,
        T: Into<Primary>,
    {
        let ids: Vec<InnerPrimary> = ids.into_iter().map(|id| (&id.into()).into()).collect();
        @%- if def.updated_at_conf().is_some() %@
        if updater._op.@{ ConfigDef::updated_at()|to_var_name }@ == Op::None {
            updater.mut_@{ ConfigDef::updated_at() }@().set(@{(def.updated_at_conf().unwrap() == Timestampable::RealTime)|if_then_else_ref("SystemTime::now()","conn.time()")}@.into());
        }
        @%- endif %@
        Self::__update_many(conn, ids, updater).await
    }
    @%- endif %@
    @%- if !def.disable_update() %@

    #[allow(unused_mut)]
    async fn __update_many(conn: &mut DbConn, ids: Vec<InnerPrimary>, mut obj: _Updater_) -> Result<u64> {
        if !obj.is_updated() {
            return Ok(0);
        }
        if ids.is_empty() {
            return Ok(0);
        }
        let mut rows_affected = 0;
        for ids in ids.chunks(IN_CONDITION_LIMIT) {
            rows_affected += Self::___update_many(conn, ids, &obj).await?;
        }
        info!(target: "db_update::@{ db|snake }@::@{ group_name }@::@{ mod_name }@", op = "update_many", ctx = conn.ctx_no(), ids = primaries_to_str(&ids); "{}", &obj);
        debug!("{:?}", &obj);
        @%- if !config.force_disable_cache %@
        @%- if def.act_as_job_queue() %@
        @%- else if def.use_clear_whole_cache() %@
        conn.clear_whole_cache = true;
        @%- else %@
        if !conn.clear_whole_cache && (USE_CACHE || USE_CACHE_ALL || USE_UPDATE_NOTICE) {
            @{- def.non_primaries()|fmt_join_cache_or_not("", "
            obj._op.{var} = Op::None;", "") }@
            let mut data_list = Vec::new();
            @%- if def.non_primaries_addable().len() > 0 %@
            let has_add = false
                @{- def.non_primaries_addable()|fmt_join_cache_or_not("
                || obj._op.{var} == Op::Add || obj._op.{var} == Op::Sub", "", "") }@;
            if has_add {
                for ids in ids.chunks(IN_CONDITION_LIMIT) {
                    Self::find_many_for_update(conn, ids.iter()).await?.into_iter()
                        .for_each(|v| {
                            let mut data = v._data;
                            @{- def.non_primaries_addable()|fmt_join_cache_or_not("
                            if obj._op.{var} != Op::Add && obj._op.{var} != Op::Sub {
                                data.{var} = obj._update.{var}.clone();
                            }", "", "") }@;
                            @{- def.non_primaries()|fmt_join_cache_or_not("", "
                            data.{var} = Default::default();", "") }@
                            data_list.push(data);
                        });
                }
            }
            @%- endif %@
            @{- def.non_primaries_addable()|fmt_join_cache_or_not("
            if obj._op.{var} == Op::Add {
                obj._op.{var} = Op::Max;
            } else if obj._op.{var} == Op::Sub {
                obj._op.{var} = Op::Min;
            }", "", "") }@
            @{- def.non_primaries()|fmt_join_cache_or_not("", "
            obj._update.{var} = Default::default();", "") }@
            let cache_msg = CacheOp::UpdateMany {
                ids,
                shard_id: conn.shard_id(),
                update: obj._update,
                data_list,
                op: obj._op,
            };
            conn.push_cache_op(cache_msg.wrap()).await?;
        }
        @%- endif %@
        @%- endif %@
        Ok(rows_affected)
    }
    @%- endif %@
    @%- if !def.disable_update() %@

    #[allow(unused_assignments)]
    #[allow(unused_mut)]
    #[allow(clippy::needless_borrow)]
    async fn ___update_many(conn: &mut DbConn, ids: &[InnerPrimary], obj: &_Updater_) -> Result<u64> {
        if ids.is_empty() {
            return Ok(0);
        }
        let (mut vec, _) = Self::assign_non_primaries(obj);
        let q = "@{ def.primaries()|fmt_join_with_paren("{placeholder}", ",") }@,".repeat(ids.len());
        let sql = format!(r#"UPDATE @{ table_name|db_esc }@ SET {} WHERE @{ def.inheritance_cond(" AND ") }@@{ def.primaries()|fmt_join_with_paren("{col_esc}", ",") }@ in ({})"#, &vec.join(","), &q[0..q.len() - 1]);
        let query = sqlx::query(&sql);
        let _span = debug_span!("query", sql = &query.sql());
        let mut query = Self::bind_non_primaries(&obj, query, &sql);
        for id in ids {
            @{- def.primaries()|fmt_join("
            query = query.bind(id.{index}{bind_as});", "") }@
        }
        let result = if conn.wo_tx() {
            query.execute(conn.acquire_source().await?.as_mut()).await?
        } else {
            query.execute(conn.get_tx().await?.as_mut()).await?
        };
        Ok(result.rows_affected())
    }
    @%- endif %@

    /// insert_ignore does not return an error if the error is negligible.
    /// insert_ignore does not save the related tables.
    #[allow(clippy::unnecessary_cast)]
    pub@{ visibility }@ async fn insert_ignore(conn: &mut DbConn, mut obj: _@{ pascal_name }@Updater) -> Result<Option<_@{ pascal_name }@Updater>> {
        obj.__validate()?;
        ensure!(obj.is_new(), "The obj is not new.");
        obj.__set_default_value(conn)@% if config.use_sequence %@.await?@% endif %@;
        let sql = r#"INSERT IGNORE INTO @{ table_name|db_esc }@ (@{ def.all_fields()|fmt_join("{col_esc}", ",") }@) 
            VALUES (@{ def.all_fields()|fmt_join("{placeholder}", ",") }@)"#;
        let query = query_bind(sql, &obj._data);
        let _span = debug_span!("query", sql = &query.sql());
        let result = if conn.wo_tx() {
            query.execute(conn.acquire_source().await?.as_mut()).await?
        } else {
            query.execute(conn.get_tx().await?.as_mut()).await?
        };
        if result.rows_affected() == 0 {
            return Ok(None);
        }
@{- def.auto_inc()|fmt_join("
        if obj._data.{var} == 0 {
            obj._data.{var} = result.last_insert_id() as {inner};
        }", "") }@
        info!(target: "db_update::@{ db|snake }@::@{ group_name }@::@{ mod_name }@", op = "insert_ignore", ctx = conn.ctx_no(); "{}", &obj);
        debug!("{:?}", &obj);
        obj._is_new = false;
        obj._op = OpData::default();
        @%- if !config.force_disable_cache %@
        @%- if def.act_as_job_queue() %@
        conn.push_cache_op(CacheOp::Queued.wrap()).await?;
        @%- else if def.use_clear_whole_cache() %@
        conn.clear_whole_cache = true;
        @%- else %@
        if !conn.clear_whole_cache && (USE_CACHE || USE_CACHE_ALL || USE_UPDATE_NOTICE) {
            let cache_msg = CacheOp::Insert {
                shard_id: conn.shard_id(),
                data: obj._data.clone(),
@{- def.relations_one_cache(false)|fmt_rel_join("\n                _{rel_name}: None,", "") }@
@{- def.relations_many_cache(false)|fmt_rel_join("\n                _{rel_name}: None,", "") }@
            };
            conn.push_cache_op(cache_msg.wrap()).await?;
        }
        @%- endif %@
        @%- endif %@
        Ok(Some(obj))
    }
    @%- if def.use_insert_delayed() %@

    /// If insert_delayed is used, the data will be collectively registered later.
    pub@{ visibility }@ async fn insert_delayed(conn: &mut DbConn, mut obj: _@{ pascal_name }@Updater) -> Result<()> {
        ensure!(obj.is_new(), "The obj is not new.");
        obj.__validate()?;
        obj.__set_default_value(conn)@% if config.use_sequence %@.await?@% endif %@;
        info!(target: "db_update::@{ db|snake }@::@{ group_name }@::@{ mod_name }@", op = "insert_delayed", ctx = conn.ctx_no(); "{}", &obj);
        debug!("{:?}", &obj);
        conn.push_callback(Box::new(|| {
            async move {
                INSERT_DELAYED_QUEUE.push(obj.into());
                if !crate::is_test_mode() {
                    DelayedActor::handle(DelayedMsg::InsertFromMemory);
                } else {
                    handle_delayed_msg_insert_from_memory().await;
                }
            }.boxed()
        })).await;
        Ok(())
    }
    @%- endif %@
    @%- if def.use_save_delayed() %@

    // The data will be updated collectively later.
    // save_delayed does not support relational tables.
    pub@{ visibility }@ async fn save_delayed(conn: &mut DbConn, mut obj: _@{ pascal_name }@Updater) -> Result<()> {
        obj.__validate()?;
        obj.__set_default_value(conn)@% if config.use_sequence %@.await?@% endif %@;
        if obj.will_be_deleted() {
            obj._op = OpData::default();
        }
@{- def.relations_one(false)|fmt_rel_join("\n        obj.{rel_name} = None;", "") }@
@{- def.relations_many(false)|fmt_rel_join("\n        obj.{rel_name} = None;", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("\n        obj.{rel_name} = None;", "") }@
        let shard_id = conn.shard_id() as usize;
        conn.push_callback(Box::new(move || {
            async move {
                SAVE_DELAYED_QUEUE.get().unwrap()[shard_id].push(obj);
                if !crate::is_test_mode() {
                    DelayedActor::handle(DelayedMsg::Save);
                } else {
                    handle_delayed_msg_save().await;
                }
            }.boxed()
        })).await;
        Ok(())
    }
    @%- endif %@
    @%- if def.use_update_delayed() %@

    // The data will be updated collectively later.
    // update_delayed does not support relational tables and version.
    pub@{ visibility }@ async fn update_delayed(conn: &mut DbConn, mut obj: _@{ pascal_name }@Updater) -> Result<()> {
        if obj.is_new() {
            panic!("INSERT is not supported.");
        }
        @%- if def.soft_delete().is_none() %@
        if obj.will_be_deleted() {
            panic!("DELETE is not supported.");
        }
        @%- endif %@
        obj.__validate()?;
@{- def.relations_one(false)|fmt_rel_join("\n        obj.{rel_name} = None;", "") }@
@{- def.relations_many(false)|fmt_rel_join("\n        obj.{rel_name} = None;", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("\n        obj.{rel_name} = None;", "") }@
        let shard_id = conn.shard_id() as usize;
        conn.push_callback(Box::new(move || {
            async move {
                UPDATE_DELAYED_QUEUE.get().unwrap()[shard_id].push(obj);
                if !crate::is_test_mode() {
                    DelayedActor::handle(DelayedMsg::Update);
                } else {
                    handle_delayed_msg_update().await;
                }
            }.boxed()
        })).await;
        Ok(())
    }
    @%- endif %@
    @%- if def.use_upsert_delayed() %@

    // The data will be updated collectively later.
    // upsert_delayed does not support relational tables.
    pub@{ visibility }@ async fn upsert_delayed(conn: &mut DbConn, mut obj: _@{ pascal_name }@Updater) -> Result<()> {
        obj.__validate()?;
        @%- if def.soft_delete().is_none() %@
        if obj.will_be_deleted() {
            panic!("DELETE is not supported.");
        }
        @%- endif %@
        obj.__set_default_value(conn)@% if config.use_sequence %@.await?@% endif %@;
@{- def.relations_one(false)|fmt_rel_join("\n        obj.{rel_name} = None;", "") }@
@{- def.relations_many(false)|fmt_rel_join("\n        obj.{rel_name} = None;", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("\n        obj.{rel_name} = None;", "") }@
        let shard_id = conn.shard_id() as usize;
        conn.push_callback(Box::new(move || {
            async move {
                UPSERT_DELAYED_QUEUE.get().unwrap()[shard_id].push(obj);
                if !crate::is_test_mode() {
                    DelayedActor::handle(DelayedMsg::Upsert);
                } else {
                    handle_delayed_msg_upsert().await;
                }
            }.boxed()
        })).await;
        Ok(())
    }
    @%- endif %@

    /// If ignore is ture, bulk_insert does not guarantee that the related table will be saved correctly
    pub@{ visibility }@ async fn bulk_insert(conn: &mut DbConn, mut list: Vec<_@{ pascal_name }@Updater>, ignore: bool) -> Result<()> {
        let mut vec: Vec<ForInsert> = Vec::with_capacity(list.len());
        for mut obj in list.into_iter() {
            ensure!(obj.is_new(), "The obj is not new.");
            obj.__validate()?;
            obj.__set_default_value(conn)@% if config.use_sequence %@.await?@% endif %@;
            vec.push(obj.into());
        }
        Self::__bulk_insert(conn, &vec, ignore, false).await
    }
@%- if !def.disable_update() %@

    pub@{ visibility }@ async fn bulk_overwrite(conn: &mut DbConn, mut list: Vec<_@{ pascal_name }@Updater>) -> Result<()> {
        let mut vec: Vec<ForInsert> = Vec::with_capacity(list.len());
        @%- if def.soft_delete().is_none() %@
        let mut remove_ids: Vec<Primary> = Vec::with_capacity(list.len());
        @%- endif %@
        for mut obj in list.into_iter() {
            obj.__validate()?;
            obj.__set_default_value(conn)@% if config.use_sequence %@.await?@% endif %@;
            @%- if def.soft_delete().is_none() %@
            if obj.will_be_deleted() {
                remove_ids.push((&obj).into());
                continue;
            } else {
                obj.__set_overwrite_extra_value(conn);
            }
            @%- else %@
            obj.__set_overwrite_extra_value(conn);
            @%- endif %@
            vec.push(obj.into());
        }
        @%- if def.soft_delete().is_none() %@
        Self::delete_by_ids(conn, remove_ids).await?;
        @%- endif %@
        Self::__bulk_insert(conn, &vec, false, true).await
    }
@%- endif %@

    async fn __bulk_insert(conn: &mut DbConn, list: &[ForInsert], ignore: bool, overwrite: bool) -> Result<()> {
        let result = Self::___bulk_insert(conn, list, ignore, overwrite).await?;
        @%- if !config.force_disable_cache %@
        @%- if def.act_as_job_queue() %@
        conn.push_cache_op(CacheOp::Queued.wrap()).await?;
        @%- else if def.use_clear_whole_cache() %@
        conn.clear_whole_cache = true;
        @%- else %@
        if !conn.clear_whole_cache && (USE_CACHE || USE_CACHE_ALL || USE_UPDATE_NOTICE) {
            for list in result {
                let cache_msg = CacheOp::BulkInsert {
                    shard_id: conn.shard_id(),
                    list,
                };
                conn.push_cache_op_to(cache_msg.wrap(), @{ def.disable_insert_cache_propagation }@).await?;
            }
        }
        @%- endif %@
        @%- endif %@
        Ok(())
    }

    pub(crate) async fn ___bulk_insert(conn: &mut DbConn, list: &[ForInsert], ignore: bool, overwrite: bool) -> Result<Vec<Vec<ForInsert>>> {
        if list.is_empty() {
            return Ok(Vec::new());
        }
        let total_size: usize = list.iter().map(|v| v._data._size()).sum();
        let ave = total_size / list.len();
        let chunks = list.chunks(cmp::max(1, BULK_INSERT_MAX_SIZE.get().unwrap() / ave));
        let mut result = Vec::new();
        for chunk in chunks {
            result.push(Self::____bulk_insert(conn, chunk, ignore, overwrite).await?);
        }
        Ok(result)
    }

    #[allow(clippy::needless_borrow)]
    #[allow(clippy::unnecessary_cast)]
    #[allow(clippy::needless_if)]
    #[allow(clippy::unwrap_or_default)]
    fn ____bulk_insert<'a>(conn: &'a mut DbConn, list: &'a [ForInsert], ignore: bool, overwrite: bool) -> BoxFuture<'a, Result<Vec<ForInsert>>> {
        async move {
            if list.is_empty() {
                return Ok(Vec::new());
            }
            const SQL_NORMAL: &str = r#"INSERT "#; 
            const SQL_IGNORE: &str = r#"INSERT IGNORE "#; 
            const SQL1: &str = r#"INTO @{ table_name|db_esc }@ (@{ def.all_fields()|fmt_join("{col_esc}", ",") }@) VALUES "#;
            const SQL2: &str = r#"(@{ def.all_fields()|fmt_join("{placeholder}", ",") }@)"#;
            const SQL3: &str = r#" ON DUPLICATE KEY UPDATE @{ def.non_primaries()|fmt_join("{col_esc}=VALUES({col_esc})", ",") }@"#;
            let mut sql = String::with_capacity(SQL_IGNORE.len() + SQL1.len() + (SQL2.len() + 1) * list.len() + SQL3.len());
            if ignore {
                sql.push_str(SQL_IGNORE);
            } else {
                sql.push_str(SQL_NORMAL);
            }
            sql.push_str(SQL1);
            sql.push_str(SQL2);
            for _i in 0..list.len() - 1 {
                sql.push(',');
                sql.push_str(SQL2);
            }
            if overwrite {
                sql.push_str(SQL3);
            }
            let mut query = sqlx::query(&sql);
            let _span = debug_span!("query", sql = &query.sql());
            for data in list {
    @{- def.all_fields()|fmt_join("\n                query = query.bind(data._data.{var}{bind_as});", "") }@
            }
            let result = if conn.wo_tx() {
                query.execute(conn.acquire_source().await?.as_mut()).await?
            } else {
                query.execute(conn.get_tx().await?.as_mut()).await?
            };
            @{- def.auto_inc()|fmt_join("
            let mut id = result.last_insert_id() as {inner};", "") }@
            let mut data_list = Vec::new();
            @{- def.relations_one(false)|fmt_rel_join("
            let mut _{rel_name} = Vec::new();
            let mut __{rel_name} = Vec::new();", "") }@
            @{- def.relations_many(false)|fmt_rel_join("
            let mut _{rel_name} = Vec::new();
            let mut __{rel_name} = Vec::new();", "") }@
            for row in list {
                info!(target: "db_update::@{ db|snake }@::@{ group_name }@::@{ mod_name }@", op = "bulk_insert", ctx = conn.ctx_no(); "{}", &row);
                debug!("{:?}", &row);
                let mut obj = row.clone();
                @{- def.auto_inc()|fmt_join("
                if obj._data.{var} == 0 {
                    obj._data.{var} = id;
                    // innodb_autoinc_lock_mode must be 0 or 1
                    id += 1;
                }", "") }@
                @{- def.relations_one(false)|fmt_rel_join("
                obj.{rel_name}.map(|v| v.map(|v| {
                    let mut v = v.as_ref().clone();
                    RelFk{rel_name_pascal}::set_fk(&mut v, (&obj._data).into());
                    _{rel_name}.push(v);
                    __{rel_name}.push(row);
                }));", "") }@
                @{- def.relations_many(false)|fmt_rel_join("
                if let Some(v) = obj.{rel_name} {
                    v.into_iter().for_each(|mut v| {
                        RelFk{rel_name_pascal}::set_fk(&mut v, (&obj._data).into());
                        _{rel_name}.push(v);
                    });
                    __{rel_name}.push(row);
                }", "") }@
                data_list.push(obj._data);
            }
            if ignore || overwrite {
                @{- def.relations_one(false)|fmt_rel_join("
                if !__{rel_name}.is_empty() {
                    let filter = RelFil{rel_name_pascal}::in_filter(&__{rel_name});
                    rel_{class_mod}::{class}::query().filter(filter).delete(conn).await?;
                }", "") }@
                @{- def.relations_many(false)|fmt_rel_join("
                if !__{rel_name}.is_empty() {
                    let filter = RelFil{rel_name_pascal}::in_filter(&__{rel_name});
                    rel_{class_mod}::{class}::query().filter(filter).delete(conn).await?;
                }", "") }@
            }
            @{- def.relations_one(false)|fmt_rel_join("
            let mut _{rel_name} = rel_{class_mod}::{class}::___bulk_insert(conn, &_{rel_name}, ignore, overwrite).await?.into_iter().fold(FxHashMap::default(), |mut map, v| {
                for v in v {
                    map.insert(RelFk{rel_name_pascal}::get_fk(&v._data).unwrap(), v);
                }
                map
            });", "") }@
            @{- def.relations_many(false)|fmt_rel_join("
            let mut _{rel_name} = rel_{class_mod}::{class}::___bulk_insert(conn, &_{rel_name}, ignore, overwrite).await?.into_iter().fold(FxHashMap::default(), |mut map, v| {
                for v in v {
                    map.entry(RelFk{rel_name_pascal}::get_fk(&v._data).unwrap())
                        .or_insert_with(Vec::new)
                        .push(v);
                }
                map
            });", "") }@
            let data_list = data_list.into_iter().map(|v| ForInsert {
                @{- def.relations_one(false)|fmt_rel_join("
                {rel_name}: Some(_{rel_name}.remove(&(&v).into()).map(Box::new)),", "") }@
                @{- def.relations_many(false)|fmt_rel_join("
                {rel_name}: _{rel_name}.remove(&(&v).into()),", "") }@
                _data: v,
            }).collect();
            Ok(data_list)
        }.boxed()
    }
    @%- if !def.disable_update() %@

    pub@{ visibility }@ async fn bulk_upsert(conn: &mut DbConn, mut list: Vec<_@{ pascal_name }@Updater>, mut updater: _@{ pascal_name }@Updater) -> Result<()> {
        if list.is_empty() {
            return Ok(());
        }
        let mut vec = Vec::with_capacity(list.len());
        for mut obj in list.into_iter() {
            ensure!(obj.is_new(), "bulk_upsert supports only new objects.");
            obj.__validate()?;
            obj.__set_default_value(conn)@% if config.use_sequence %@.await?@% endif %@;
            vec.push(obj._data);
        }
        @%- if def.updated_at_conf().is_some() %@
        if updater._op.@{ ConfigDef::updated_at()|to_var_name }@ == Op::None {
            updater.mut_@{ ConfigDef::updated_at() }@().set(@{(def.updated_at_conf().unwrap() == Timestampable::RealTime)|if_then_else_ref("SystemTime::now()","conn.time()")}@.into());
        }
        @%- endif %@
        Self::__bulk_upsert(conn, &vec, &updater).await
    }

    async fn __bulk_upsert(conn: &mut DbConn, list: &[Data], obj: &_Updater_) -> Result<()> {
        let total_size: usize = list.iter().map(|v| v._size()).sum();
        let ave = total_size / list.len();
        let chunks = list.chunks(cmp::max(1, BULK_INSERT_MAX_SIZE.get().unwrap() / ave));
        for chunk in chunks {
            Self::___bulk_upsert(conn, chunk, obj).await?;
        }
        Ok(())
    }
    @%- endif %@
    @%- if !def.disable_update() %@

    #[allow(unused_assignments)]
    #[allow(clippy::needless_borrow)]
    async fn ___bulk_upsert(conn: &mut DbConn, list: &[Data], obj: &_Updater_) -> Result<()> {
        if list.is_empty() {
            return Ok(());
        }
        const SQL1: &str = r#"INSERT INTO @{ table_name|db_esc }@ (@{ def.all_fields()|fmt_join("{col_esc}", ",") }@) VALUES "#;
        const SQL2: &str = r#"(@{ def.all_fields()|fmt_join("{placeholder}", ",") }@)"#;
        let mut sql = String::with_capacity(SQL1.len() + (SQL2.len() + 1) * list.len() + 100);
        sql.push_str(SQL1);
        sql.push_str(SQL2);
        for _i in 0..list.len() - 1 {
            sql.push(',');
            sql.push_str(SQL2);
        }
        let (mut vec, _) = Self::assign_non_primaries(obj);
        @%- if def.versioned %@
        vec.push(r#"\"@{ version_col }@\" = IF(\"@{ version_col }@\" < 4294967295, \"@{ version_col }@\" + 1, 0)"#.to_string());
        @%- endif %@
        write!(sql, " ON DUPLICATE KEY UPDATE {}", &vec.join(","))?;
        let mut query = sqlx::query(&sql);
        let _span = debug_span!("query", sql = &query.sql());
        for data in list {
@{- def.all_fields()|fmt_join("
            query = query.bind(data.{var}{bind_as});", "") }@
            info!(target: "db_update::@{ db|snake }@::@{ group_name }@::@{ mod_name }@", op = "bulk_upsert", ctx = conn.ctx_no(); "{}", &data);
            debug!("{:?}", &data);
        }
        info!(target: "db_update::@{ db|snake }@::@{ group_name }@::@{ mod_name }@", op = "bulk_upsert_updater", ctx = conn.ctx_no(); "{}", obj);
        let query = Self::bind_non_primaries(&obj, query, &sql);
        if conn.wo_tx() {
            query.execute(conn.acquire_source().await?.as_mut()).await?;
        } else {
            query.execute(conn.get_tx().await?.as_mut()).await?;
        }
        @%- if !config.force_disable_cache %@
        @%- if def.act_as_job_queue() %@
        @%- else if def.use_clear_whole_cache() %@
        conn.clear_whole_cache = true;
        @%- else %@
        if !conn.clear_whole_cache && (USE_CACHE || USE_CACHE_ALL || USE_UPDATE_NOTICE) {
            let cache_msg = CacheOp::BulkUpsert {
                shard_id: conn.shard_id(),
                data_list: list.to_vec(),
                update: obj._update.clone(),
                op: obj._op.clone(),
            };
            conn.push_cache_op(cache_msg.wrap()).await?;
        }
        @%- endif %@
        @%- endif %@
        Ok(())
    }
@%- endif %@
@%- if !def.disable_update() %@

    #[allow(clippy::needless_borrow)]
    pub@{ visibility }@ async fn delete_by_ids<I, T>(conn: &mut DbConn, ids: I) -> Result<u64>
    where
        I: IntoIterator<Item = T>,
        T: Into<Primary>,
    {
@%- if def.soft_delete().is_some() %@
        let ids: Vec<InnerPrimary> = ids.into_iter().map(|id| (&id.into()).into()).collect();
        async fn inner(conn: &mut DbConn, ids: Vec<InnerPrimary>) -> Result<u64> {
            if ids.is_empty() {
                return Ok(0);
            }
            let mut rows_affected = 0u64;
            let id_chunks = ids.chunks(IN_CONDITION_LIMIT);
            @{- def.soft_delete_tpl2("","
            let deleted_at: {filter_type} = {val}.into();","","
            let deleted = cmp::max(1, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as u32);")}@
            @%- if def.updated_at_conf().is_some() %@
            @%- let updated_at = def.get_updated_at() %@
            let updated_at: @{ updated_at.get_filter_type(false) }@ = @{(def.updated_at_conf().unwrap() == Timestampable::RealTime)|if_then_else_ref("SystemTime::now()","conn.time()")}@.into();
            @%- endif %@
            for ids in id_chunks {
                let q = "@{ def.primaries()|fmt_join_with_paren("{placeholder}", ",") }@,".repeat(ids.len());
                let sql = format!(
                    r#"UPDATE @{ table_name|db_esc }@ SET @{ def.soft_delete_tpl2("","deleted_at=?","deleted=1","deleted=?")}@@% if def.updated_at_conf().is_some() %@, updated_at=?@%- endif %@ WHERE @{ def.inheritance_cond(" AND ") }@@{ def.primaries()|fmt_join_with_paren("{col_esc}", ",") }@ in ({});"#,
                    &q[0..q.len() - 1]
                );
                let mut query = sqlx::query(&sql);
                let _span = debug_span!("query", sql = &query.sql());
    @{- def.soft_delete_tpl2("","
                query = query.bind(deleted_at);","","
                query = query.bind(deleted);")}@
    @%- if def.updated_at_conf().is_some() %@
                query = query.bind(updated_at);
    @%- endif %@
                for id in ids {
                    @{- def.primaries()|fmt_join("
                    query = query.bind(id.{index}{bind_as});", "") }@
                }
                let result = if conn.wo_tx() {
                    query.execute(conn.acquire_source().await?.as_mut()).await?
                } else {
                    query.execute(conn.get_tx().await?.as_mut()).await?
                };
                rows_affected += result.rows_affected();
            }
            info!(target: "db_update::@{ db|snake }@::@{ group_name }@::@{ mod_name }@", op = "delete_by_ids", ctx = conn.ctx_no(), ids = primaries_to_str(&ids); "");
            @%- if !config.force_disable_cache %@
            @%- if def.act_as_job_queue() %@
            @%- else if def.use_clear_whole_cache() %@
            conn.clear_whole_cache = true;
            @%- else %@
            if !conn.clear_whole_cache && (USE_CACHE || USE_CACHE_ALL || USE_UPDATE_NOTICE) {
                let mut updater = _@{ pascal_name }@::updater();
                @{- def.soft_delete_tpl2("","
                updater.mut_deleted_at().set(Some(deleted_at));","
                updater.mut_deleted().set(true);","
                let deleted = cmp::max(1, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as u32);
                updater.mut_deleted().set(deleted);")}@
    @%- if def.updated_at_conf().is_some() %@
                updater.mut_@{ ConfigDef::updated_at() }@().set(updated_at);
    @%- endif %@
                @{- def.non_primaries()|fmt_join_cache_or_not("", "
                updater._op.{var} = Op::None;
                updater._update.{var} = Default::default();", "") }@
                let cache_msg = CacheOp::UpdateMany {
                    ids,
                    shard_id: conn.shard_id(),
                    update: updater._update,
                    data_list: Vec::new(),
                    op: updater._op,
                };
                conn.push_cache_op(cache_msg.wrap()).await?;
                @{- def.cache_owners|fmt_cache_owners("
                conn.push_cache_op(crate::models::{mod}::CacheOp::InvalidateAll.wrap()).await?;") }@
            }
            @%- endif %@
            @%- endif %@
            Ok(rows_affected)
        }
        inner(conn, ids).await
@%- else %@
        Self::force_delete_by_ids(conn, ids).await
@%- endif %@
    }

    #[allow(clippy::needless_borrow)]
    pub@{ visibility }@ async fn force_delete_by_ids<I, T>(conn: &mut DbConn, ids: I) -> Result<u64>
    where
        I: IntoIterator<Item = T>,
        T: Into<Primary>,
    {
        let ids: Vec<InnerPrimary> = ids.into_iter().map(|id| (&id.into()).into()).collect();
        async fn inner(conn: &mut DbConn, ids: Vec<InnerPrimary>) -> Result<u64> {
            if ids.is_empty() {
                return Ok(0);
            }
            let mut rows_affected = 0u64;
            let id_chunks = ids.chunks(IN_CONDITION_LIMIT);
            for ids in id_chunks {
                let q = "@{ def.primaries()|fmt_join_with_paren("{placeholder}", ",") }@,".repeat(ids.len());
    @%- if def.use_on_delete_fn %@
                let sql = format!(
                    r#"SELECT {} FROM @{ table_name|db_esc }@ WHERE @{ def.inheritance_cond(" AND ") }@@{ def.primaries()|fmt_join_with_paren("{col_esc}", ",") }@ in ({}) FOR UPDATE;"#,
                    Data::_sql_cols(),
                    &q[0..q.len() - 1]
                );
                let mut query = sqlx::query_as::<_, Data>(&sql);
                let _span = debug_span!("query", sql = &query.sql());
                for id in ids {
                    @{- def.primaries()|fmt_join("
                    query = query.bind(id.{index}{bind_as});", "") }@
                }
                let result = if conn.wo_tx() {
                    query.fetch_all(conn.acquire_source().await?.as_mut()).await?
                } else {
                    query.fetch_all(conn.get_tx().await?.as_mut()).await?
                };
                let list: Vec<_@{ pascal_name }@> = result.into_iter().map(|v| v.into()).collect();
                _@{ pascal_name }@::_before_delete(conn, &list).await?;
                conn.push_callback(Box::new(|| {
                    async move {
                        _@{ pascal_name }@::_after_delete(&list).await;
                    }.boxed()
                })).await;
    @%- endif %@
    @%- for on_delete_str in def.on_delete_list %@
                crate::models::@{ on_delete_str }@::__on_delete_@{ group_name }@_@{ mod_name }@(conn, ids, false).await?;
    @%- endfor %@
                let sql = format!(
                    r#"DELETE FROM @{ table_name|db_esc }@ WHERE @{ def.primaries()|fmt_join_with_paren("{col_esc}", ",") }@ in ({});"#,
                    &q[0..q.len() - 1]
                );
                let mut query = sqlx::query(&sql);
                let _span = debug_span!("query", sql = &query.sql());
                for id in ids {
                    @{- def.primaries()|fmt_join("
                    query = query.bind(id.{index}{bind_as});", "") }@
                }
                let result = if conn.wo_tx() {
                    query.execute(conn.acquire_source().await?.as_mut()).await?
                } else {
                    query.execute(conn.get_tx().await?.as_mut()).await?
                };
                rows_affected += result.rows_affected();
            }
            info!(target: "db_update::@{ db|snake }@::@{ group_name }@::@{ mod_name }@", op = "force_delete_by_ids", ctx = conn.ctx_no(), ids = primaries_to_str(&ids); "");
            @%- if !config.force_disable_cache %@
            @%- if def.act_as_job_queue() %@
            @%- else if def.use_clear_whole_cache() %@
            conn.clear_whole_cache = true;
            @%- else %@
            if !conn.clear_whole_cache && (USE_CACHE || USE_CACHE_ALL || USE_UPDATE_NOTICE) {
                let shard_id = conn.shard_id();
                conn.push_cache_op(CacheOp::DeleteMany { ids, shard_id }.wrap()).await?;
                @{- def.cache_owners|fmt_cache_owners("
                conn.push_cache_op(crate::models::{mod}::CacheOp::InvalidateAll.wrap()).await?;") }@
            }
            @%- endif %@
            @%- endif %@
            Ok(rows_affected)
        }
        inner(conn, ids).await
    }
@%- endif %@
@%- if !def.disable_update() %@

    #[allow(unused_mut)]
    pub@{ visibility }@ async fn delete(conn: &mut DbConn, mut obj: _@{ pascal_name }@Updater) -> Result<()> {
        @%- if def.updated_at_conf().is_some() %@
        if obj._op.@{ ConfigDef::updated_at()|to_var_name }@ == Op::None {
            obj.mut_@{ ConfigDef::updated_at() }@().set(@{(def.updated_at_conf().unwrap() == Timestampable::RealTime)|if_then_else_ref("SystemTime::now()","conn.time()")}@.into());
        }
        @%- endif %@
        @%- if !config.force_disable_cache && !def.use_clear_whole_cache() && def.cache_owners.len() > 0 %@
        if !conn.clear_whole_cache {
            @{- def.cache_owners|fmt_cache_owners("
            if let Some(v) = crate::models::{mod}::RelFk{rel_name_pascal}::get_fk(&obj._data) {
                conn.push_cache_op(crate::models::{mod}::_{model_name}Cache::__invalidate_cache_op(conn, v)).await?;
            }") }@
        }
        @%- endif %@
        let cache_msg = Self::__delete(conn, obj).await?;
        @%- if !config.force_disable_cache %@
        @%- if def.act_as_job_queue() %@
        @%- else if def.use_clear_whole_cache() %@
        conn.clear_whole_cache = true;
        @%- else %@
        if !conn.clear_whole_cache && (USE_CACHE || USE_CACHE_ALL || USE_UPDATE_NOTICE) {
            if let Some(cache_msg) = cache_msg {
                conn.push_cache_op(cache_msg.wrap()).await?;
            }
        }
        @%- endif %@
        @%- endif %@
        Ok(())
    }
@%- endif %@
@%- if !def.disable_update() %@

    #[allow(unused_mut)]
    async fn __delete(conn: &mut DbConn, mut obj: _@{ pascal_name }@Updater) -> Result<Option<CacheOp>> {
        if obj.has_been_deleted() {
            return Ok(None);
        }
@{- def.soft_delete_tpl2("
        Self::force_delete(conn, obj).await?;
        Ok(None)","
        obj.mut_deleted_at().set(Some({val}.into()));
        let (_obj, cache_msg) = Self::__save_update(conn, obj).await?;
        Ok(cache_msg)","
        obj.mut_deleted().set(true);
        let (_obj, cache_msg) = Self::__save_update(conn, obj).await?;
        Ok(cache_msg)","
        let deleted = cmp::max(1, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as u32);
        obj.mut_deleted().set(deleted);
        let (_obj, cache_msg) = Self::__save_update(conn, obj).await?;
        Ok(cache_msg)")}@
    }
@%- endif %@
@%- if !def.disable_update() && def.soft_delete().is_some() %@

    #[allow(unused_mut)]
    pub@{ visibility }@ async fn restore(conn: &mut DbConn, mut obj: _@{ pascal_name }@Updater) -> Result<_@{ pascal_name }@> {
        obj._do_delete = false;
        if !obj.has_been_deleted() {
            return Ok(obj.into());
        }
        @%- if !config.force_disable_cache && !def.use_clear_whole_cache() && def.cache_owners.len() > 0 %@
        if !conn.clear_whole_cache {
            @{- def.cache_owners|fmt_cache_owners("
            if let Some(v) = crate::models::{mod}::RelFk{rel_name_pascal}::get_fk(&obj._data) {
                conn.push_cache_op(crate::models::{mod}::_{model_name}Cache::__invalidate_cache_op(conn, v)).await?;
            }") }@
        }
        @%- endif %@
        @%- if def.updated_at_conf().is_some() %@
        if obj._op.@{ ConfigDef::updated_at()|to_var_name }@ == Op::None {
            obj.mut_@{ ConfigDef::updated_at() }@().set(@{(def.updated_at_conf().unwrap() == Timestampable::RealTime)|if_then_else_ref("SystemTime::now()","conn.time()")}@.into());
        }
        @%- endif %@
@{- def.soft_delete_tpl2("","
        obj.mut_deleted_at().set(None);","
        obj.mut_deleted().set(false);","
        obj.mut_deleted().set(0);")}@
        let (obj, cache_msg) = Self::__save_update(conn, obj).await?;
        @%- if !config.force_disable_cache %@
        @%- if def.act_as_job_queue() %@
        conn.push_cache_op(CacheOp::Queued.wrap()).await?;
        @%- else if def.use_clear_whole_cache() %@
        conn.clear_whole_cache = true;
        @%- else %@
        if !conn.clear_whole_cache && (USE_CACHE || USE_CACHE_ALL || USE_UPDATE_NOTICE) {
            if let Some(cache_msg) = cache_msg {
                conn.push_cache_op(cache_msg.wrap()).await?;
            }
        }
        @%- endif %@
        @%- endif %@
        Ok(obj)
    }
@%- endif %@
@%- if !def.disable_update() %@

    #[allow(clippy::needless_borrow)]
    pub@{ visibility }@ async fn force_delete(conn: &mut DbConn, obj: _@{ pascal_name }@Updater) -> Result<()> {
        let id: InnerPrimary = (&obj).into();
@%- if def.use_on_delete_fn %@
        let notify_obj: Self = if obj._is_loaded {
            Self::from(obj.clone())
        } else {
            Self::find_for_update(conn, &id, None).await?.into()
        };
        Self::_before_delete(conn, &[notify_obj.clone()]).await?;
@%- endif %@
@%- for on_delete_str in def.on_delete_list %@
        crate::models::@{ on_delete_str }@::__on_delete_@{ group_name }@_@{ mod_name }@(conn, &[id.clone()], false).await?;
@%- endfor %@
        let mut query = sqlx::query(r#"DELETE FROM @{ table_name|db_esc }@ WHERE @{ def.primaries()|fmt_join("{col_esc}={placeholder}", " AND ") }@"#);
        let _span = debug_span!("query", sql = &query.sql());
        @{- def.primaries()|fmt_join("
        query = query.bind(id.{index}{bind_as});", "") }@
        if conn.wo_tx() {
            query.execute(conn.acquire_source().await?.as_mut()).await?;
        } else {
            query.execute(conn.get_tx().await?.as_mut()).await?;
        }
        info!(target: "db_update::@{ db|snake }@::@{ group_name }@::@{ mod_name }@", op = "force_delete", ctx = conn.ctx_no(), id = id.to_string(); "{}", &obj);
@%- if def.use_on_delete_fn %@
        conn.push_callback(Box::new(|| {
            async {
                Self::_after_delete(&[notify_obj]).await;
            }.boxed()
        })).await;
@%- endif %@
@%- if !config.force_disable_cache %@
        @%- if def.act_as_job_queue() %@
        @%- else if def.use_clear_whole_cache() %@
        conn.clear_whole_cache = true;
        @%- else %@
        if !conn.clear_whole_cache && (USE_CACHE || USE_CACHE_ALL || USE_UPDATE_NOTICE) {
            let shard_id = conn.shard_id();
            conn.push_cache_op(CacheOp::Delete { id, shard_id }.wrap()).await?;
        }
        @{- def.cache_owners|fmt_cache_owners("
        if let Some(v) = crate::models::{mod}::RelFk{rel_name_pascal}::get_fk(&obj._data) {
            conn.push_cache_op(crate::models::{mod}::_{model_name}Cache::__invalidate_cache_op(conn, v)).await?;
        }") }@
        @%- endif %@
@%- endif %@
        Ok(())
    }
@%- endif %@

    pub@{ visibility }@ async fn force_delete_relations(conn: &mut DbConn, obj: _@{ pascal_name }@Updater) -> Result<()> {
        let id: InnerPrimary = (&obj).into();
@%- for on_delete_str in def.on_delete_list %@
        crate::models::@{ on_delete_str }@::__on_delete_@{ group_name }@_@{ mod_name }@(conn, &[id.clone()], true).await?;
@%- endfor %@
        Ok(())
    }

    pub@{ visibility }@ async fn force_delete_all(conn: &mut DbConn) -> Result<()> {
        let query = sqlx::query(r#"DELETE FROM @{ table_name|db_esc }@"#);
        let _span = debug_span!("query", sql = &query.sql());
        if conn.wo_tx() {
            query.execute(conn.acquire_source().await?.as_mut()).await?;
        } else {
            query.execute(conn.get_tx().await?.as_mut()).await?;
        }
        info!(target: "db_update::@{ db|snake }@::@{ group_name }@::@{ mod_name }@", op = "force_delete_all", ctx = conn.ctx_no(); "");
        @%- if !config.force_disable_cache %@
        @%- if def.act_as_job_queue() %@
        @%- else if def.use_clear_whole_cache() %@
        conn.clear_whole_cache = true;
        @%- else %@
        if !conn.clear_whole_cache && (USE_CACHE || USE_CACHE_ALL || USE_UPDATE_NOTICE) {
            conn.push_cache_op(CacheOp::DeleteAll {
                shard_id: conn.shard_id(),
            }.wrap()).await?;
            @{- def.cache_owners|fmt_cache_owners("
            conn.push_cache_op(crate::models::{mod}::CacheOp::InvalidateAll.wrap()).await?;") }@
        }
        @%- endif %@
        @%- endif %@
        Ok(())
    }

    pub@{ visibility }@ async fn truncate(conn: &mut DbConn) -> Result<()> {
        let query = sqlx::query(r#"TRUNCATE TABLE @{ table_name|db_esc }@"#);
        let _span = debug_span!("query", sql = &query.sql());
        query.execute(conn.acquire_source().await?.as_mut()).await?;
        info!(target: "db_update::@{ db|snake }@::@{ group_name }@::@{ mod_name }@", op = "truncate", ctx = conn.ctx_no(); "");
        @%- if !config.force_disable_cache %@
        @%- if def.act_as_job_queue() %@
        @%- else if def.use_clear_whole_cache() %@
        conn.clear_whole_cache = true;
        @%- else %@
        if !conn.clear_whole_cache && (USE_CACHE || USE_CACHE_ALL || USE_UPDATE_NOTICE) {
            conn.push_cache_op(CacheOp::DeleteAll {
                shard_id: conn.shard_id(),
            }.wrap()).await?;
            @{- def.cache_owners|fmt_cache_owners("
            conn.push_cache_op(crate::models::{mod}::CacheOp::InvalidateAll.wrap()).await?;") }@
        }
        @%- endif %@
        @%- endif %@
        Ok(())
    }
@%- for base_mod_name in def.relations_on_delete_mod() %@

    pub(crate) fn __on_delete_@{ base_mod_name }@<'a>(
        conn: &'a mut DbConn,
        ids: &'a [rel_@{ base_mod_name }@::InnerPrimary],
        cascade_only: bool,
    ) -> BoxFuture<'a, Result<()>> {
        async move {
            @%- for (mod_name, rel_name, rel) in def.relations_on_delete_cascade() %@
            @%- if base_mod_name == mod_name %@
            Self::__on_delete_@{ mod_name }@_for_@{ rel_name }@(conn, ids, cascade_only).await?;
            @%- endif %@
            @%- endfor %@
            @%- for (mod_name, rel_name, rel) in def.relations_on_delete_restrict() %@
            @%- if base_mod_name == mod_name %@
            Self::__on_delete_@{ mod_name }@_for_@{ rel_name }@(conn, ids, cascade_only).await?;
            @%- endif %@
            @%- endfor %@
            @%- for (mod_name, rel_name, local, val, val2, rel) in def.relations_on_delete_not_cascade() %@
            @%- if base_mod_name == mod_name %@
            Self::__on_delete_@{ mod_name }@_for_@{ rel_name }@(conn, ids, cascade_only).await?;
            @%- endif %@
            @%- endfor %@
            Ok(())
        }.boxed()
    }
@%- endfor %@
@%- for (rel_mod_name, rel_name, rel) in def.relations_on_delete_cascade() %@

    #[allow(clippy::needless_borrow)]
    async fn __on_delete_@{ rel_mod_name }@_for_@{ rel_name }@(
        conn: &mut DbConn,
        ids: &[rel_@{ rel_mod_name }@::InnerPrimary],
        cascade_only: bool,
    ) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let id_chunks = ids.chunks(IN_CONDITION_LIMIT);
        for ids in id_chunks {
            let q = "@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("{placeholder}", ",") }@,".repeat(ids.len());
@%- if def.use_on_delete_fn %@
            let sql = format!(
                r#"SELECT {} FROM @{ table_name|db_esc }@ WHERE @{ def.inheritance_cond(" AND ") }@@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("{col_esc}", ", ") }@ in ({}) FOR UPDATE;"#,
                Data::_sql_cols(),
                &q[0..q.len() - 1]
            );
            let mut query = sqlx::query_as::<_, Data>(&sql);
            let _span = debug_span!("query", sql = &query.sql());
            for id in ids {
                query = query@{ rel.get_local_cols(rel_name, def)|fmt_join(".bind(id.{index}{bind_as})", "") }@;
            }
            let result = if conn.wo_tx() {
                query.fetch_all(conn.acquire_source().await?.as_mut()).await?
            } else {
                query.fetch_all(conn.get_tx().await?.as_mut()).await?
            };
            let result_num = result.len() as u64;
            let list: Vec<Self> = result.into_iter().map(|v| v.into()).collect();
            let id_list: Vec<InnerPrimary> = list.iter().map(|v| v.into()).collect();
            Self::_before_delete(conn, &list).await?;
            conn.push_callback(Box::new(|| {
                async move {
                    Self::_after_delete(&list).await;
                }.boxed()
            })).await;
@%- if !config.force_disable_cache %@
            @%- if def.act_as_job_queue() %@
            @%- else if def.use_clear_whole_cache() %@
            conn.clear_whole_cache = true;
            @%- else %@
            if !conn.clear_whole_cache && (USE_CACHE || USE_CACHE_ALL || USE_UPDATE_NOTICE) {
                let cache_msg = CacheOp::Cascade { ids: id_list.clone(), shard_id: conn.shard_id() };
                conn.push_cache_op(cache_msg.wrap()).await?;
            }
            @%- endif %@
@%- endif %@
@%- for on_delete_str in def.on_delete_list %@
            crate::models::@{ on_delete_str }@::__on_delete_@{ group_name }@_@{ mod_name }@(conn, &id_list, cascade_only).await?;
@%- endfor %@
@%- else %@
            let sql = format!(
                r#"SELECT @{ def.primaries()|fmt_join("{col_query}", ", ") }@ FROM @{ table_name|db_esc }@ WHERE @{ def.inheritance_cond(" AND ") }@@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("{col_esc}", ", ") }@ in ({});"#,
                &q[0..q.len() - 1]
            );
            let mut query = sqlx::query_as::<_, InnerPrimary>(&sql);
            let _span = debug_span!("query", sql = &query.sql());
            for id in ids {
                query = query@{ rel.get_local_cols(rel_name, def)|fmt_join(".bind(id.{index}{bind_as})", "") }@;
            }
            let id_list = if conn.wo_tx() {
                query.fetch_all(conn.acquire_source().await?.as_mut()).await?
            } else {
                query.fetch_all(conn.get_tx().await?.as_mut()).await?
            };
            let result_num = id_list.len() as u64;
@%- if !config.force_disable_cache %@
            @%- if def.act_as_job_queue() %@
            @%- else if def.use_clear_whole_cache() %@
            conn.clear_whole_cache = true;
            @%- else %@
            if !conn.clear_whole_cache && (USE_CACHE || USE_CACHE_ALL || USE_UPDATE_NOTICE) {
                let cache_msg = CacheOp::Cascade { ids: id_list.clone(), shard_id: conn.shard_id() };
                conn.push_cache_op(cache_msg.wrap()).await?;
            }
            @%- endif %@
@%- endif %@
@%- for on_delete_str in def.on_delete_list %@
            crate::models::@{ on_delete_str }@::__on_delete_@{ group_name }@_@{ mod_name }@(conn, &id_list, cascade_only).await?;
@%- endfor %@
@%- endif %@
            let sql = format!(
                r#"DELETE FROM @{ table_name|db_esc }@ WHERE @{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("{col_esc}", ", ") }@ in ({});"#,
                &q[0..q.len() - 1]
            );
            let mut query = sqlx::query(&sql);
            let _span = debug_span!("query", sql = &query.sql());
            for id in ids {
                query = query@{ rel.get_local_cols(rel_name, def)|fmt_join(".bind(id.{index}{bind_as})", "") }@;
            }
            let result = if conn.wo_tx() {
                query.execute(conn.acquire_source().await?.as_mut()).await?
            } else {
                query.execute(conn.get_tx().await?.as_mut()).await?
            };
            ensure!(
                result_num == result.rows_affected(),
                "Mismatch occurred when deleting @{ table_name }@."
            );
        }
        Ok(())
    }
@%- endfor %@
@%- for (rel_mod_name, rel_name, rel) in def.relations_on_delete_restrict() %@

    #[allow(clippy::needless_borrow)]
    async fn __on_delete_@{ rel_mod_name }@_for_@{ rel_name }@(
        conn: &mut DbConn,
        ids: &[rel_@{ rel_mod_name }@::InnerPrimary],
        cascade_only: bool,
    ) -> Result<()> {
        if ids.is_empty() || cascade_only {
            return Ok(());
        }
        let id_chunks = ids.chunks(IN_CONDITION_LIMIT);
        for ids in id_chunks {
            let q = "@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("{placeholder}", ",") }@,".repeat(ids.len());
            let sql = format!(
                r#"SELECT count(*) as c FROM @{ table_name|db_esc }@ WHERE @{ def.inheritance_cond(" AND ") }@@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("{col_esc}", ", ") }@ in ({});"#,
                &q[0..q.len() - 1]
            );
            let mut query = sqlx::query_as::<_, Count>(&sql);
            let _span = debug_span!("query", sql = &query.sql());
            for id in ids {
                query = query@{ rel.get_local_cols(rel_name, def)|fmt_join(".bind(id.{index}{bind_as})", "") }@;
            }
            let result = if conn.wo_tx() {
                query.fetch_one(conn.acquire_source().await?.as_mut()).await?
            } else {
                query.fetch_one(conn.get_tx().await?.as_mut()).await?
            };
            ensure!(
                result.c == 0,
                "Cannot delete or update a parent row: a foreign key constraint fails on @{ table_name }@."
            );
        }
        Ok(())
    }
@%- endfor %@
@%- for (rel_mod_name, rel_name, local, val, val2, rel) in def.relations_on_delete_not_cascade() %@

    #[allow(clippy::needless_borrow)]
    async fn __on_delete_@{ rel_mod_name }@_for_@{ rel_name }@(
        conn: &mut DbConn,
        ids: &[rel_@{ rel_mod_name }@::InnerPrimary],
        cascade_only: bool,
    ) -> Result<()> {
        if ids.is_empty() || cascade_only {
            return Ok(());
        }
        let id_chunks = ids.chunks(IN_CONDITION_LIMIT);
        for ids in id_chunks {
            let q = "@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("{placeholder}", ",") }@,".repeat(ids.len());
            let sql = format!(
                r#"SELECT @{ def.primaries()|fmt_join("{col_query}", ", ") }@ FROM @{ table_name|db_esc }@ WHERE @{ def.inheritance_cond(" AND ") }@@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("{col_esc}", ", ") }@ in ({});"#,
                &q[0..q.len() - 1]
            );
            let mut query = sqlx::query_as::<_, InnerPrimary>(&sql);
            let _span = debug_span!("query", sql = &query.sql());
            for id in ids {
                query = query@{ rel.get_local_cols(rel_name, def)|fmt_join(".bind(id.{index}{bind_as})", "") }@;
            }
            let id_list = if conn.wo_tx() {
                query.fetch_all(conn.acquire_source().await?.as_mut()).await?
            } else {
                query.fetch_all(conn.get_tx().await?.as_mut()).await?
            };
            let result_num = id_list.len() as u64;
@%- if !config.force_disable_cache %@
            @%- if def.act_as_job_queue() %@
            @%- else if def.use_clear_whole_cache() %@
            conn.clear_whole_cache = true;
            @%- else %@
            if !conn.clear_whole_cache && (USE_CACHE || USE_CACHE_ALL || USE_UPDATE_NOTICE) {
                let cache_msg = CacheOp::Reset@{ rel_name|pascal }@@{ val|pascal }@ { ids: id_list.clone(), shard_id: conn.shard_id() };
                conn.push_cache_op(cache_msg.wrap()).await?;
            }
            @%- endif %@
@%- endif %@
            let sql = format!(
                r#"UPDATE @{ table_name|db_esc }@ SET @{ local|db_esc }@ = @{ val }@ WHERE @{ def.inheritance_cond(" AND ") }@@{ rel.get_local_cols(rel_name, def)|fmt_join_with_paren("{col_esc}", ", ") }@ in ({});"#,
                &q[0..q.len() - 1]
            );
            let mut query = sqlx::query(&sql);
            let _span = debug_span!("query", sql = &query.sql());
            for id in ids {
                query = query@{ rel.get_local_cols(rel_name, def)|fmt_join(".bind(id.{index}{bind_as})", "") }@;
            }
            let result = if conn.wo_tx() {
                query.execute(conn.acquire_source().await?.as_mut()).await?
            } else {
                query.execute(conn.get_tx().await?.as_mut()).await?
            };
            ensure!(
                result_num == result.rows_affected(),
                "Mismatch occurred when set @{ local }@ = @{ val }@ @{ table_name }@."
            );
        }
        Ok(())
    }
@%- endfor %@
}

#[allow(clippy::needless_borrow)]
fn query_bind<'a>(sql: &'a str, data: &'a Data) -> Query<'a, DbType, DbArguments> {
    let mut query = sqlx::query(sql);
    @{- def.all_fields()|fmt_join("
    query = query.bind(data.{var}{bind_as});", "") }@
    query
}

impl _@{ pascal_name }@Factory {
    #[allow(clippy::needless_update)]
    pub@{ visibility }@ fn create(self) -> _@{ pascal_name }@Updater {
        _@{ pascal_name }@Updater {
            _data: Data {
@{ def.for_factory()|fmt_join("                {var}: self.{var}{convert_factory},", "\n") }@
                ..Data::default()
            },
            _update: Data::default(),
            _is_new: true,
            _do_delete: false,
            _upsert: false,
            _is_loaded: true,
            _op: OpData::default(),
@{- def.relations_one(false)|fmt_rel_join("\n            {rel_name}: self.{rel_name}.map(|v| vec![v.create()]),", "") }@
@{- def.relations_many(false)|fmt_rel_join("\n            {rel_name}: self.{rel_name}.map(|v| v.into_iter().map(|v| v.create()).collect()),", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("\n            {rel_name}: None,", "") }@
        }
    }
}

impl From<_@{ pascal_name }@Updater> for _@{ pascal_name }@ {
    fn from(from: _@{ pascal_name }@Updater) -> Self {
        let mut to: _@{ pascal_name }@ = from._data.into();
@{- def.relations_one(false)|fmt_rel_join("
        to.{rel_name} = from.{rel_name}.map(|v| v.into_iter().filter(|v| !v.will_be_deleted()).last().map(|v| Box::new(v.into())));", "") }@
@{- def.relations_many(false)|fmt_rel_join("
        to.{rel_name} = from.{rel_name}.map(|v| v.into_iter().map(|v| v.into()).collect());", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("
        to.{rel_name} = from.{rel_name};", "") }@
        to
    }
}
impl From<Box<_@{ pascal_name }@Updater>> for Box<_@{ pascal_name }@> {
    fn from(from: Box<_@{ pascal_name }@Updater>) -> Self {
        let mut to: _@{ pascal_name }@ = from._data.into();
@{- def.relations_one(false)|fmt_rel_join("
        to.{rel_name} = from.{rel_name}.map(|v| v.into_iter().filter(|v| !v.will_be_deleted()).last().map(|v| Box::new(v.into())));", "") }@
@{- def.relations_many(false)|fmt_rel_join("
        to.{rel_name} = from.{rel_name}.map(|v| v.into_iter().map(|v| v.into()).collect());", "") }@
@{- def.relations_belonging(false)|fmt_rel_join("
        to.{rel_name} = from.{rel_name};", "") }@
        Box::new(to)
    }
}

pub(crate) async fn _seed(seed: &serde_yaml::Value, conns: &mut [DbConn]) -> Result<()> {
    if let Some(mapping) = seed.as_mapping() {
        for (name, factory) in mapping {
            let seed: _@{ pascal_name }@Factory = serde_yaml::from_str(&serde_yaml::to_string(&factory)?)?;
            let shard_id = seed._shard_id().await as usize;
            let conn = &mut conns[shard_id];
            let obj = seed.create();
            if let Some(obj) = _@{ pascal_name }@::save(conn, obj).await? {
                @{- def.auto_inc_or_seq()|fmt_join("
                let id = obj._{raw_var}();
                if GENERATED_IDS.get().is_none() {
                    let _ = GENERATED_IDS.set(std::sync::RwLock::new(HashMap::new()));
                }
                let name = name.as_str().unwrap().to_string();
                GENERATED_IDS.get().unwrap().write().unwrap().insert(name, id);", "") }@
            }
        }
    }
    Ok(())
}
@{- def.relations_one(false)|fmt_rel_join("

async fn save_{rel_name}(
    conn: &mut DbConn,
    obj: &mut _{pascal_name},
    data: Option<Vec<rel_{class_mod}::{class}Updater>>,
    update_cache: &mut bool,
) -> Result<Option<Vec<rel_{class_mod}::CacheOp>>> {
    if let Some(list) = data {
        let mut msgs = Vec::new();
        let src = _save_{rel_name}(conn, (&*obj).into(), list);
        futures::pin_mut!(src);
        let mut vec = Vec::new();
        while let Some(value) = src.next().await.transpose()? {
            if let Some(obj2) = value.0 {
                vec.push(obj2);
            }
            if let Some(msg) = value.1 {
                msgs.push(msg);
                *update_cache = true;
            }
        }
        obj.{rel_name} = Some(vec.pop().map(Box::new));
        return Ok(Some(msgs));
    }
    Ok(None)
}", "") }@
@{- def.relations_many(false)|fmt_rel_join("

async fn save_{rel_name}(
    conn: &mut DbConn,
    obj: &mut _{pascal_name},
    data: Option<Vec<rel_{class_mod}::{class}Updater>>,
    update_cache: &mut bool,
) -> Result<Option<Vec<rel_{class_mod}::CacheOp>>> {
    if let Some(list) = data {
        let mut msgs = Vec::new();
        let src = _save_{rel_name}(conn, (&*obj).into(), list);
        futures::pin_mut!(src);
        let mut vec = Vec::new();
        while let Some(value) = src.next().await.transpose()? {
            if let Some(obj2) = value.0 {
                vec.push(obj2);
            }
            if let Some(msg) = value.1 {
                msgs.push(msg);
                *update_cache = true;
            }
        }
        obj.{rel_name} = Some(vec);
        return Ok(Some(msgs));
    }
    Ok(None)
}", "") }@
@{- def.relations_one_and_many(false)|fmt_rel_join("

fn _save_{rel_name}(
    conn: &mut DbConn,
    id: InnerPrimary,
    list: Vec<rel_{class_mod}::{class}Updater>,
) -> impl futures::Stream<
    Item = Result<(
        Option<rel_{class_mod}::{class}>,
        Option<rel_{class_mod}::CacheOp>,
    )>,
> + '_ {
    async_stream::try_stream! {
        let mut update_list = Vec::with_capacity(list.len());
        let mut insert_list = Vec::with_capacity(list.len());
        for row in list.into_iter() {
            if row.is_new() {
                insert_list.push(row);
            }else if row.will_be_deleted() {
                yield rel_{class_mod}::{class}::___save(conn, row).await?;
            } else{
                update_list.push(row);
            }
        }
        for mut row in update_list.into_iter() {
            RelFk{rel_name_pascal}::set_fk(&mut row._data, id.clone());
            RelFk{rel_name_pascal}::set_fk(&mut row._update, id.clone());
            RelCol{rel_name_pascal}::set_op_none(&mut row._op);
            yield rel_{class_mod}::{class}::___save(conn, row).await?;
        }
        for mut row in insert_list.into_iter() {
            RelFk{rel_name_pascal}::set_fk(&mut row._data, id.clone());
            yield rel_{class_mod}::{class}::___save(conn, row).await?;
        }
    }
}", "") }@
@{-"\n"}@