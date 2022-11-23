use actix::{fut::WrapFuture, Actor, Addr, ArbiterHandle, AsyncContext, Context, Handler, Message};
use ahash::{AHashMap, AHasher};
use anyhow::{ensure, Context as _, Result};
use arc_swap::ArcSwapOption;
use async_trait::async_trait;
use core::option::Option;
use crossbeam::queue::SegQueue;
use derive_more::Display;
use futures::stream::StreamExt;
use futures::{future, Future, FutureExt, Stream, TryStreamExt};
use fxhash::{FxHashMap, FxHashSet, FxHasher64};
use indexmap::IndexMap;
use log::{debug, error, info, warn};
use once_cell::sync::{Lazy, OnceCell};
use schemars::JsonSchema;
use senax_common::cache::db_cache::{CacheVal, HashVal};
use senax_common::cache::msec::MSec;
use senax_common::cache::{calc_mem_size, CycleCounter};
use senax_common::ShardId;
use senax_common::{err, types::blob::*, types::point::*, SqlColumns};
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
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use std::vec::Vec;
use std::{cmp, fmt};
use strum::{EnumMessage, EnumString, IntoStaticStr};
use tokio::sync::{mpsc, Semaphore};
use tokio::time::{sleep, Duration};
use tracing::info_span;
use validator::Validate;
use zstd::{decode_all, encode_all};

use crate::cache::Cache;
use crate::connection::{DbArguments, DbConn, DbType};
use crate::misc::IntoJson as _;
use crate::misc::{BindValue, ForUpdateTr, Size, TrashMode};
use crate::{
    accessor::*, CacheMsg, BULK_FETCH_SEMAPHORE, BULK_INSERT_MAX_SIZE, CACHE_DELAY_SAFETY1,
    CACHE_DELAY_SAFETY2, IN_CONDITION_LIMIT,
};

@% for mod_name in def.relation_mods() -%@
use crate::@{ mod_name[0] }@::@{ mod_name[1] }@::_@{ mod_name[1] }@ as rel_@{ mod_name[0] }@_@{ mod_name[1] }@;
@% endfor %@
const USE_CACHE: bool = @{ def.use_cache()|if_then_else("true", "false") }@;
const USE_FAST_CACHE: bool = @{ def.use_fast_cache()|if_then_else("true", "false") }@;
const USE_CACHE_ALL: bool = @{ def.use_cache_all()|if_then_else("true", "false") }@;
const IGNORE_PROPAGATED_INSERT_CACHE: bool = @{ def.ignore_propagated_insert_cache|if_then_else("true", "false") }@;
@% if def.primaries().len() == 1 -%@
@% for (name, column_def) in def.primaries() -%@
pub const ID_COLUMN: &str = "@{ name }@";
@% endfor -%@
@% endif -%@
pub const TRASHED_SQL: &str = r#"@{ def.inheritance_cond(" AND ") }@"#;
pub const NOT_TRASHED_SQL: &str = r#"@{ def.soft_delete_tpl("","deleted_at IS NULL AND ","deleted = 0 AND ")}@@{ def.inheritance_cond(" AND ") }@"#;
pub const ONLY_TRASHED_SQL: &str = r#"@{ def.soft_delete_tpl("","deleted_at IS NOT NULL AND ","deleted != 0 AND ")}@@{ def.inheritance_cond(" AND ") }@"#;

static CACHE_ALL: OnceCell<Vec<ArcSwapOption<Vec<_@{ pascal_name }@Cache>>>> = OnceCell::new();
static CACHE_ALL_DIRTY_FLAG: AtomicBool = AtomicBool::new(false);
static CACHE_RESET_TIME: AtomicU64 = AtomicU64::new(0);
static BULK_FETCH_QUEUE: OnceCell<Vec<SegQueue<Primary>>> = OnceCell::new();
static PRIMARY_TYPE_ID: u64 = @{ def.get_type_id("PRIMARY_TYPE_ID") }@;
static COL_KEY_TYPE_ID: u64 = @{ def.get_type_id("COL_KEY_TYPE_ID") }@;
static VERSION_TYPE_ID: u64 = @{ def.get_type_id("VERSION_TYPE_ID") }@;
static CACHE_TYPE_ID: u64 = @{ def.get_type_id("CACHE_TYPE_ID") }@;

pub(crate) async fn init(handle: Option<&ArbiterHandle>) -> Result<()> {
    if CACHE_ALL.get().is_none() {
        CACHE_ALL.set(DbConn::shard_num_range().map(|_| ArcSwapOption::const_empty()).collect()).unwrap();
        UPDATE_DELAYED_QUEUE.set(DbConn::shard_num_range().map(|_| SegQueue::new()).collect()).unwrap();
        UPSERT_DELAYED_QUEUE.set(DbConn::shard_num_range().map(|_| SegQueue::new()).collect()).unwrap();
        BULK_FETCH_QUEUE.set(DbConn::shard_num_range().map(|_| SegQueue::new()).collect()).unwrap();
    }

    if let Some(handle) = handle {
        let addr = DelayedActor::start_in_arbiter(handle, |_| DelayedActor);
        DELAYED_ADDR.set(addr).unwrap();
        handle.spawn(async {
            while !crate::is_stopped() {
                DELAYED_ADDR.get().unwrap().do_send(DelayedMsg::InsertFromDisk);
                sleep(Duration::from_secs(10)).await;
            }
        });
    }
    Ok(())
}

pub(crate) async fn check(shard_id: ShardId) -> Result<()> {
    let mut conn = DbConn::_new(shard_id);
    _@{ pascal_name }@::query().limit(0).select(&mut conn).await?;
    Ok(())
}

pub(crate) async fn init_db(db: &sled::Db) -> Result<()> {
    let tree = db.open_tree("@{ name }@")?;
    INSERT_DELAYED_DB.set(tree).unwrap();
    DELAYED_ADDR.get().unwrap().do_send(DelayedMsg::InsertFromMemory);
    DELAYED_ADDR.get().unwrap().do_send(DelayedMsg::InsertFromDisk);
    Ok(())
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) enum CacheOp {
    Insert {
        shard_id: ShardId,
        data: Data,
@{- def.relations_one_cache()|fmt_rel_join("
        #[serde(default, skip_serializing_if = \"Option::is_none\")]
        _{alias}: Option<Option<rel_{class_mod}::CacheOp>>,", "") }@
@{- def.relations_many_cache()|fmt_rel_join("
        #[serde(default, skip_serializing_if = \"Option::is_none\")]
        _{alias}: Option<Vec<rel_{class_mod}::CacheOp>>,", "") }@
    },
    BulkInsert {
        shard_id: ShardId,
        list: Vec<ForInsert>,
    },
    Update {
        id: Primary,
        shard_id: ShardId,
        update: Data,
        op: OpData,
@{- def.relations_one_cache()|fmt_rel_join("
        #[serde(default, skip_serializing_if = \"Option::is_none\")]
        _{alias}: Option<Option<rel_{class_mod}::CacheOp>>,", "") }@
@{- def.relations_many_cache()|fmt_rel_join("
        #[serde(default, skip_serializing_if = \"Option::is_none\")]
        _{alias}: Option<Vec<rel_{class_mod}::CacheOp>>,", "") }@
    },
    UpdateMany {
        ids: Vec<Primary>,
        shard_id: ShardId,
        update: Data,
        op: OpData,
    },
    BulkUpsert {
        shard_id: ShardId,
        data_list: Vec<Data>,
        update: Data,
        op: OpData,
    },
    Delete {
        id: Primary,
        shard_id: ShardId,
    },
    DeleteMany {
        ids: Vec<Primary>,
        shard_id: ShardId,
    },
    DeleteAll,
    Cascade {
        ids: Vec<Primary>,
        shard_id: ShardId,
    },
    Invalidate {
        id: Primary,
        shard_id: ShardId,
    },
    InvalidateAll,
@%- for (mod_name, local, val, val2) in def.relations_on_delete_not_cascade() %@
    Reset@{ local|pascal }@@{ val|pascal }@ {
        ids: Vec<Primary>,
        shard_id: ShardId,
    },
@%- endfor %@
}

impl CacheOp {
    pub(crate) fn update(mut obj: CacheData, update: &Data, op: &OpData) -> CacheData {
        @{- def.cache_cols_without_primary()|fmt_join("
        Accessor{accessor_with_sep_type}::_set(op.{var}, &mut obj.{var}, &update.{var});", "") }@
        obj
    }

    pub(crate) fn apply_to_obj(obj: &Option<Arc<CacheWrapper>>, msg: &Option<CacheOp>, shard_id: ShardId, time: MSec) -> Option<Arc<CacheWrapper>> {
        if let Some(msg) = msg {
            match msg {
                CacheOp::Insert { data, .. } => Some(Arc::new(CacheWrapper::_from_data(data.clone(), shard_id, time))),
                CacheOp::BulkInsert { .. } => None,
                CacheOp::Update { update, op, .. } => {
                    if let Some(ref obj) = obj {
                        let mut wrapper = obj.as_ref().clone();
                        wrapper._inner = Self::update(wrapper._inner.clone(), update, op);
                        Some(Arc::new(wrapper))
                    } else {
                        None
                    }
                }
                CacheOp::UpdateMany { .. } => None,
                CacheOp::BulkUpsert { .. } => None,
                CacheOp::Delete { .. } => None,
                CacheOp::DeleteMany { .. } => None,
                CacheOp::DeleteAll => None,
                CacheOp::Cascade { .. } => None,
                CacheOp::Invalidate { .. } => None,
                CacheOp::InvalidateAll => None,
                @%- for (mod_name, local, val, val2) in def.relations_on_delete_not_cascade() %@
                CacheOp::Reset@{ local|pascal }@@{ val|pascal }@ { .. } => None,
                @%- endfor %@
            }
        } else {
            obj.as_ref().cloned()
        }
    }

    pub(crate) fn apply_to_list(list: &[Arc<CacheWrapper>], msgs: &[CacheOp], shard_id: ShardId, time: MSec) -> Vec<Arc<CacheWrapper>> {
        let mut map = list.iter().map(|v| (Primary::from(&v._inner), Arc::clone(v))).collect::<IndexMap<_, _>>();
        for msg in msgs {
            match msg {
                CacheOp::Insert { data, .. } => {
                    map.insert(Primary::from(data), Arc::new(CacheWrapper::_from_data(data.clone(), shard_id, time)));
                }
                CacheOp::BulkInsert { .. } => {},
                CacheOp::Update { id, update, op, .. } => {
                    if let Some(obj) = map.get(id) {
                        let mut wrapper = obj.as_ref().clone();
                        wrapper._inner = Self::update(wrapper._inner.clone(), update, op);
                        map.insert(id.clone(), Arc::new(wrapper));
                    }
                }
                CacheOp::UpdateMany { ids, update, op, .. } => {
                    for id in ids {
                        if let Some(obj) = map.get(id) {
                            let mut wrapper = obj.as_ref().clone();
                            wrapper._inner = Self::update(wrapper._inner.clone(), update, op);
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
                CacheOp::DeleteAll => {},
                CacheOp::Cascade { .. } => {},
                CacheOp::Invalidate { .. } => {},
                CacheOp::InvalidateAll => {},
                @%- for (mod_name, local, val, val2) in def.relations_on_delete_not_cascade() %@
                CacheOp::Reset@{ local|pascal }@@{ val|pascal }@ { .. } => {},
                @%- endfor %@
            }
        }
        map.into_iter().map(|(_k, v)| v).collect()
    }

    #[allow(clippy::let_and_return)]
    async fn update_with_unique_cache(id: &PrimaryHasher, obj: CacheData, update: &Data, op: &OpData, time: MSec) -> CacheData {
@%- for (index_name, index) in def.unique_index() %@
        if @{ index.fields(index_name, def)|fmt_index_col_not_null_or_null("op.{var} != Op::None", "op.{var} != Op::None && obj.{var}.is_some()", " && ") }@ {
            let key = VecColKey(vec![@{- index.fields(index_name, def)|fmt_index_col_not_null_or_null("ColKey::{var}(obj.{var}.clone().into())", "ColKey::{var}(obj.{var}.as_ref().unwrap().clone().into())", ", ") }@]);
            Cache::invalidate(&key, id._shard_id()).await;
        }
@%- endfor  %@
        let obj = CacheOp::update(obj, update, op);
@%- for (index_name, index) in def.unique_index() %@
        if @{ index.fields(index_name, def)|fmt_index_col_not_null_or_null("op.{var} != Op::None", "op.{var} != Op::None && obj.{var}.is_some()", " && ") }@ {
            let key = VecColKey(vec![@{- index.fields(index_name, def)|fmt_index_col_not_null_or_null("ColKey::{var}(obj.{var}.clone().into())", "ColKey::{var}(obj.{var}.as_ref().unwrap().clone().into())", ", ") }@]);
            Cache::insert_short(&key, Arc::new(id.to_wrapper(time))).await;
        }
@%- endfor  %@
        obj
    }

    #[allow(clippy::redundant_clone)]
    pub(crate) fn handle_cache_msg(self, time: MSec, propagated: bool) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(async move {
            _@{pascal_name}@::_receive_update_notice(&self).await;
            match self {
                CacheOp::Insert { shard_id, data
                    @{- def.relations_one_cache()|fmt_rel_join(", _{alias}", "") -}@ 
                    @{- def.relations_many_cache()|fmt_rel_join(", _{alias}", "") }@ } => {
                    delayed_clear_cache_all();
                    if USE_CACHE && (!propagated || !IGNORE_PROPAGATED_INSERT_CACHE) {
                        let mut cache = CacheWrapper::_from_data(data.clone(), shard_id, time);
        @{- def.relations_one_cache()|fmt_rel_join("
                        if let Some(_{alias}) = _{alias} {
                            cache.{alias} = rel_{class_mod}::CacheOp::apply_to_obj(&cache.{alias}, &_{alias}, shard_id, time);
                            if let Some(msg) = _{alias} {
                                msg.handle_cache_msg(time, propagated).await;
                            }
                        }", "") }@
        @{- def.relations_many_cache()|fmt_rel_join("
                        if let Some(_{alias}) = _{alias} {
                            cache.{alias} = rel_{class_mod}::CacheOp::apply_to_list(&cache.{alias}, &_{alias}, shard_id, time);
                            {list_sort}
                            {list_limit}
                            for msg in _{alias} {
                                msg.handle_cache_msg(time, propagated).await;
                            }
                        }", "") }@
                        let id = PrimaryHasher(Primary::from(&cache._inner), shard_id);
                        @%- if def.versioned %@
                        let vw = VersionWrapper{
                            id: id.0.clone(),
                            shard_id,
                            time,
                            version: 0,
                        };
                        if Cache::get_version::<VersionWrapper>(&vw, shard_id).await.filter(|o| o.id == id.0).is_none() {
                            Cache::insert_short(&id, Arc::new(cache)).await;
                        }
                        @%- else %@
                        if Cache::get_from_memory::<CacheWrapper>(&id, shard_id, USE_FAST_CACHE).await.filter(|o| Primary::from(o) == id.0).is_none() {
                            Cache::insert_short(&id, Arc::new(cache)).await;
                        }
                        @%- endif %@
                        @%- for (index_name, index) in def.unique_index() %@
                        if @{ index.fields(index_name, def)|fmt_index_col_not_null_or_null("true", " data.{var}.is_some()", " && ") }@ {
                            let key = VecColKey(vec![@{- index.fields(index_name, def)|fmt_index_col_not_null_or_null("ColKey::{var}(data.{var}.clone().into())", "ColKey::{var}(data.{var}.unwrap().clone().into())", ", ") }@]);
                            Cache::invalidate(&key, shard_id).await;
                            Cache::insert_short(&key, Arc::new(id.to_wrapper(time))).await;
                        }
                        @%- endfor  %@
                    }
                }
                CacheOp::BulkInsert { shard_id, list } => {
                    delayed_clear_cache_all();
                    for row in list {
                        if USE_CACHE && (!propagated || !IGNORE_PROPAGATED_INSERT_CACHE) {
                            let mut cache = CacheWrapper::_from_data(row._data.clone(), shard_id, time);
                            @{- def.relations_one_cache()|fmt_rel_join("
                            if let Some(_{alias}) = row.{alias} {
                                cache.{alias} = _{alias}.map(|v| Arc::new(rel_{class_mod}::CacheWrapper::_from_data(v._data, shard_id, time)));
                            }", "") }@
                            @{- def.relations_many_cache()|fmt_rel_join("
                            if let Some(_{alias}) = row.{alias} {
                                cache.{alias} = _{alias}.into_iter().map(|v| Arc::new(rel_{class_mod}::CacheWrapper::_from_data(v._data, shard_id, time))).collect();
                                {list_sort}
                                {list_limit}
                            }", "") }@
                            let id = PrimaryHasher(Primary::from(&cache._inner), shard_id);
                            @%- if def.versioned %@
                            let vw = VersionWrapper{
                                id: id.0.clone(),
                                shard_id,
                                time,
                                version: 0,
                            };
                            if Cache::get_version::<VersionWrapper>(&vw, shard_id).await.filter(|o| o.id == id.0).is_none() {
                                Cache::insert_short(&id, Arc::new(cache)).await;
                            }
                            @%- else %@
                            if Cache::get_from_memory::<CacheWrapper>(&id, shard_id, USE_FAST_CACHE).await.filter(|o| Primary::from(o) == id.0).is_none() {
                                Cache::insert_short(&id, Arc::new(cache)).await;
                            }
                            @%- endif %@
                            @%- for (index_name, index) in def.unique_index() %@
                            if @{ index.fields(index_name, def)|fmt_index_col_not_null_or_null("true", " row._data.{var}.is_some()", " && ") }@ {
                                let key = VecColKey(vec![@{- index.fields(index_name, def)|fmt_index_col_not_null_or_null("ColKey::{var}(row._data.{var}.clone().into())", "ColKey::{var}(row._data.{var}.unwrap().clone().into())", ", ") }@]);
                                Cache::invalidate(&key, shard_id).await;
                                Cache::insert_short(&key, Arc::new(id.to_wrapper(time))).await;
                            }
                            @%- endfor  %@
                            }
                    }
                }
                CacheOp::Update { id, shard_id, update, op
                    @{- def.relations_one_cache()|fmt_rel_join(", _{alias}", "") -}@ 
                    @{- def.relations_many_cache()|fmt_rel_join(", _{alias}", "") }@ } => {
                    delayed_clear_cache_all();
                    if USE_CACHE {
                        let id = PrimaryHasher(id.clone(), shard_id);
                        @%- if def.versioned %@
                        let vw = VersionWrapper{
                            id: id.0.clone(),
                            shard_id,
                            time,
                            version: update.@{ version_col }@,
                        };
                        if let Some(old) = Cache::get_version::<VersionWrapper>(&vw, shard_id).await.filter(|o| o.id == id.0) {
                            if old.version.less_than(vw.version) {
                                Cache::insert_version(&vw, Arc::new(vw.clone())).await;
                            }
                        } else {
                            Cache::insert_version(&vw, Arc::new(vw.clone())).await;
                        }
                        @%- endif %@
                        if let Some(cache) = Cache::get_from_memory::<CacheWrapper>(&id, shard_id, USE_FAST_CACHE).await.filter(|o| Primary::from(o) == id.0) {
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
                            cache._inner = CacheOp::update_with_unique_cache(&id, cache._inner.clone(), &update, &op, time).await;
                            @{- def.relations_one_cache()|fmt_rel_join("
                            if let Some(_{alias}) = _{alias} {
                                cache.{alias} = rel_{class_mod}::CacheOp::apply_to_obj(&cache.{alias}, &_{alias}, shard_id, time);
                                if let Some(msg) = _{alias} {
                                    msg.handle_cache_msg(time, propagated).await;
                                }
                            }", "") }@
                            @{- def.relations_many_cache()|fmt_rel_join("
                            if let Some(_{alias}) = _{alias} {
                                cache.{alias} = rel_{class_mod}::CacheOp::apply_to_list(&cache.{alias}, &_{alias}, shard_id, time);
                                {list_sort}
                                {list_limit}
                                for msg in _{alias} {
                                    msg.handle_cache_msg(time, propagated).await;
                                }
                            }", "") }@
                            Cache::insert_long(&id, Arc::new(cache), USE_FAST_CACHE).await;
                        } else {
                            let time = MSec::now().add_sec(CACHE_DELAY_SAFETY1);
                            sleep(Duration::from_millis(10)).await;
                            Cache::invalidate(&id, shard_id).await;
                            sleep(Duration::from_secs(CACHE_DELAY_SAFETY1)).await;
                            Cache::invalidate(&id, shard_id).await;
                            sleep(Duration::from_secs(CACHE_DELAY_SAFETY2)).await;
                            if let Some(cache) = Cache::get_from_memory::<CacheWrapper>(&id, shard_id, USE_FAST_CACHE).await.filter(|o| Primary::from(o) == id.0) {
                                if cache._time().less_than(time) {
                                    Cache::invalidate(&id, shard_id).await;
                                }
                            }
                        }
                    }
                }
                CacheOp::UpdateMany { ids, shard_id, update, op } => {
                    delayed_clear_cache_all();
                    if USE_CACHE {
                        let mut rest_ids = Vec::new();
                        for id in &ids {
                            let id = PrimaryHasher(id.clone(), shard_id);
                            if let Some(cache) = Cache::get_from_memory::<CacheWrapper>(&id, shard_id, USE_FAST_CACHE).await.filter(|o| Primary::from(o) == id.0) {
                                let mut cache = cache.as_ref().clone();
                                cache._inner = CacheOp::update_with_unique_cache(&id, cache._inner.clone(), &update, &op, time).await;
                                Cache::insert_long(&id, Arc::new(cache), USE_FAST_CACHE).await;
                            } else {
                                rest_ids.push(id.clone());
                            }
                        }
                        if !rest_ids.is_empty() {
                            sleep(Duration::from_millis(10)).await;
                            for id in &rest_ids {
                                Cache::invalidate(id, shard_id).await;
                            }
                            sleep(Duration::from_secs(CACHE_DELAY_SAFETY1)).await;
                            for id in &rest_ids {
                                Cache::invalidate(id, shard_id).await;
                            }
                        }
                    }
                }
                CacheOp::BulkUpsert { shard_id, data_list, update, op } => {
                    delayed_clear_cache_all();
                    if USE_CACHE {
                        let mut rest_ids = Vec::new();
                        for data in &data_list {
                            let id = PrimaryHasher(Primary::from(data), shard_id);
                            if let Some(cache) = Cache::get_from_memory::<CacheWrapper>(&id, shard_id, USE_FAST_CACHE).await.filter(|o| Primary::from(o) == id.0) {
                                let mut cache = cache.as_ref().clone();
                                cache._inner = CacheOp::update_with_unique_cache(&id, cache._inner.clone(), &update, &op, time).await;
                                Cache::insert_long(&id, Arc::new(cache), USE_FAST_CACHE).await;
                            } else {
                                rest_ids.push(id.clone());
                            }
                        }
                        if !rest_ids.is_empty() {
                            sleep(Duration::from_millis(10)).await;
                            for id in &rest_ids {
                                Cache::invalidate(id, shard_id).await;
                            }
                            sleep(Duration::from_secs(CACHE_DELAY_SAFETY1)).await;
                            for id in &rest_ids {
                                Cache::invalidate(id, shard_id).await;
                            }
                        }
                    }
                }
                CacheOp::Delete { id, shard_id } => {
                    delayed_clear_cache_all();
                    sleep(Duration::from_secs(2)).await;
                    if USE_CACHE {
                        let id = PrimaryHasher(id.clone(), shard_id);
                        Cache::invalidate(&id, shard_id).await;
                    }
                }
                CacheOp::DeleteMany { ids, shard_id } => {
                    delayed_clear_cache_all();
                    sleep(Duration::from_secs(2)).await;
                    if USE_CACHE {
                        for id in &ids {
                            let id = PrimaryHasher(id.clone(), shard_id);
                            Cache::invalidate(&id, shard_id).await;
                        }
                    }
                }
                CacheOp::DeleteAll => {
                    sleep(Duration::from_secs(CACHE_DELAY_SAFETY1)).await;
                    _@{pascal_name}@::_clear_cache();
                }
                CacheOp::Cascade { ids, shard_id } => {
                    delayed_clear_cache_all();
                    sleep(Duration::from_secs(CACHE_DELAY_SAFETY1)).await;
                    if USE_CACHE {
                        for id in &ids {
                            let id = PrimaryHasher(id.clone(), shard_id);
                            Cache::invalidate(&id, shard_id).await;
                        }
                    }
                }
                CacheOp::Invalidate { id, shard_id  } => {
                    clear_cache_all();
                    if USE_CACHE {
                        let id = PrimaryHasher(id.clone(), shard_id);
                        Cache::invalidate(&id, shard_id).await;
                    }
                }
                CacheOp::InvalidateAll => {
                    _@{pascal_name}@::_clear_cache();
                }
                @%- for (mod_name, local, val, val2) in def.relations_on_delete_not_cascade() %@
                CacheOp::Reset@{ local|pascal }@@{ val|pascal }@ { ids, shard_id } => {
                    delayed_clear_cache_all();
                    sleep(Duration::from_secs(CACHE_DELAY_SAFETY1)).await;
                    if USE_CACHE {
                        for id in &ids {
                            let id = PrimaryHasher(id.clone(), shard_id);
                            let mut update = Data::default();
                            let mut op = OpData::default();
                            update.@{ local }@ = @{ val2 }@;
                            op.@{ local }@ = Op::Set;
                            if let Some(cache) = Cache::get::<CacheWrapper>(&id, shard_id, USE_FAST_CACHE).await.filter(|o| Primary::from(o) == id.0) {
                                let mut cache = cache.as_ref().clone();
                                cache._inner = CacheOp::update(cache._inner.clone(), &update, &op);
                                Cache::insert_long(&id, Arc::new(cache), USE_FAST_CACHE).await;
                            }
                        }
                    }
                }
                @%- endfor %@
            }
        })
    }

    fn wrap(self) -> crate::CacheOp {
        crate::CacheOp::@{ group_name|to_pascal_name }@(crate::@{ group_name|to_var_name }@::CacheOp::@{ name|to_pascal_name }@(self))
    }
}

pub(crate) fn delayed_clear_cache_all() {
    if !USE_CACHE_ALL || CACHE_ALL_DIRTY_FLAG.load(Ordering::SeqCst) {
        return;
    }
    CACHE_ALL_DIRTY_FLAG.store(true, Ordering::SeqCst);
    tokio::spawn(async {
        sleep(Duration::from_secs(CACHE_DELAY_SAFETY1)).await;
        CACHE_ALL_DIRTY_FLAG.store(false, Ordering::SeqCst);
        sleep(Duration::from_millis(10)).await;
        clear_cache_all();
    });
}
pub(crate) fn clear_cache_all() {
    if USE_CACHE_ALL {
        let _ = CACHE_ALL.get().map(|c| c.iter().map(|c| c.swap(None)));
    }
@{- def.auto_increments()|fmt_join("
    if let Some(ids) = GENERATED_IDS.get() { ids.write().unwrap().clear() }", "") }@
}

static DELAYED_ADDR: OnceCell<Addr<DelayedActor>> = OnceCell::new();
static INSERT_DELAYED_QUEUE: Lazy<SegQueue<ForInsert>> = Lazy::new(SegQueue::new);
static INSERT_DELAYED_DB: OnceCell<sled::Tree> = OnceCell::new();
static INSERT_DELAYED_WAITING: AtomicBool = AtomicBool::new(false);
static DELAYED_DB_NO: Lazy<AtomicU64> = Lazy::new(|| {
    let now = SystemTime::now();
    let time = now.duration_since(UNIX_EPOCH).unwrap();
    AtomicU64::new(time.as_secs() << 20)
});
static UPDATE_DELAYED_QUEUE: OnceCell<Vec<SegQueue<ForUpdate>>> = OnceCell::new();
static UPDATE_DELAYED_WAITING: AtomicBool = AtomicBool::new(false);
static UPDATE_DELAYED_SEMAPHORE: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(1));
static UPSERT_DELAYED_QUEUE: OnceCell<Vec<SegQueue<ForUpdate>>> = OnceCell::new();
static UPSERT_DELAYED_WAITING: AtomicBool = AtomicBool::new(false);
static UPSERT_DELAYED_SEMAPHORE: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(1));

struct DelayedActor;
impl Actor for DelayedActor {
    type Context = Context<Self>;
}

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

#[derive(Message)]
#[rtype(result = "()")]
enum DelayedMsg {
    InsertFromMemory,
    InsertFromDisk,
    Update,
    Upsert,
}
impl Handler<DelayedMsg> for DelayedActor {
    type Result = ();

    fn handle(&mut self, msg: DelayedMsg, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            DelayedMsg::InsertFromMemory => {
                if INSERT_DELAYED_WAITING.load(Ordering::SeqCst) {
                    return;
                }
                ctx.spawn(
                    async move {
                        INSERT_DELAYED_WAITING.store(true, Ordering::SeqCst);
                        let _guard = crate::get_shutdown_guard();
                        sleep(Duration::from_millis(100)).await;
                        INSERT_DELAYED_WAITING.store(false, Ordering::SeqCst);
                        handle_delayed_msg_insert_from_memory().await;
                    }
                    .into_actor(self),
                );
            }
            DelayedMsg::InsertFromDisk => {
                ctx.spawn(
                    async move {
                        let _guard = crate::get_shutdown_guard();
                        let mut handles = Vec::new();
                        for shard_id in DbConn::shard_num_range() {
                            handles.push(handle_delayed_msg_insert_from_disk(shard_id));
                        }
                        future::join_all(handles).await.iter().for_each(|r| {
                            if let Err(err) = r {
                                error!(model ="@{ name }@"; "INSERT DELAYED ERROR:{}", err);
                            }
                        });
                    }
                    .into_actor(self),
                );
            }
            DelayedMsg::Update => {
                if UPDATE_DELAYED_WAITING.load(Ordering::SeqCst) {
                    return;
                }
                ctx.spawn(
                    async move {
                        UPDATE_DELAYED_WAITING.store(true, Ordering::SeqCst);
                        let _guard = crate::get_shutdown_guard();
                        let _semaphore = UPDATE_DELAYED_SEMAPHORE.acquire().await;
                        UPDATE_DELAYED_WAITING.store(false, Ordering::SeqCst);
                        handle_delayed_msg_update().await;
                    }
                    .into_actor(self),
                );
            }
            DelayedMsg::Upsert => {
                if UPSERT_DELAYED_WAITING.load(Ordering::SeqCst) {
                    return;
                }
                ctx.spawn(
                    async move {
                        UPSERT_DELAYED_WAITING.store(true, Ordering::SeqCst);
                        let _guard = crate::get_shutdown_guard();
                        let _semaphore = UPSERT_DELAYED_SEMAPHORE.acquire().await;
                        UPSERT_DELAYED_WAITING.store(false, Ordering::SeqCst);
                        handle_delayed_msg_upsert().await;
                    }
                    .into_actor(self),
                );
            }
        }
    }
}

async fn handle_delayed_msg_update() {
    let mut handles = Vec::new();
    for shard_id in DbConn::shard_num_range() {
        handles.push(_handle_delayed_msg_update(shard_id));
    }
    future::join_all(handles).await;
}

async fn handle_delayed_msg_upsert() {
    let mut handles = Vec::new();
    for shard_id in DbConn::shard_num_range() {
        handles.push(_handle_delayed_msg_upsert(shard_id));
    }
    future::join_all(handles).await;
}

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
        match conn.begin().await {
            Ok(_) => {
                conn_list.push(conn);
            }
            Err(err) => {
                error!(model ="@{ name }@"; "INSERT DELAYED ERROR:{}", err);
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
    let result = _@{ pascal_name }@::_bulk_insert(&mut conn, &buf.0, true).await;
    if let Err(err) = result {
        if let Some(err) = err.downcast_ref::<sqlx::Error>() {
            match err {
                sqlx::Error::Io(..) => {
                    // retry all
                    error!(model ="@{ name }@"; "INSERT DELAYED ERROR:{}", err);
                    drop(buf);
                    return;
                }
                sqlx::Error::WorkerCrashed => {
                    // retry all
                    error!(model ="@{ name }@"; "INSERT DELAYED ERROR:{}", err);
                    drop(buf);
                    return;
                }
                _ => {
                    let data = serde_json::to_string(&buf.0).unwrap();
                    error!(model ="@{ name }@", data = data; "INSERT DELAYED FAILED:{}", err);
                }
            }
        } else {
            let data = serde_json::to_string(&buf.0).unwrap();
            error!(model ="@{ name }@", data = data; "INSERT DELAYED FAILED:{}", err);
        }
    }
    let result = conn.commit().await;
    if is_retry_error(result) {
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
    if let Err(err) = conn.begin().await {
        error!(model ="@{ name }@"; "INSERT DELAYED ERROR:{}", err);
        return Ok(());
    }
    let mut vec = Vec::new();
    let mut total_size = 0;
    let max_size = *BULK_INSERT_MAX_SIZE.get().unwrap();
    while let Ok(x) = db.pop_min() {
        if let Some(x) = x {
            let list: Vec<ForInsert> = serde_cbor::from_slice(&decode_all::<&[u8]>(x.1.borrow())?)?;
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
        DELAYED_ADDR.get().unwrap().do_send(DelayedMsg::InsertFromDisk);
    }
    tokio::spawn(async move {
        let _guard = crate::get_shutdown_guard();
        let _ = db.flush_async().await;
    });
    Ok(())
}

async fn _handle_delayed_msg_update(shard_id: ShardId) {
    let mut map: BTreeMap<Primary, IndexMap<OpData, ForUpdate>> = BTreeMap::new();
    while let Some(x) = UPDATE_DELAYED_QUEUE.get().unwrap()[shard_id as usize].pop() {
        let inner_map = map.entry(Primary::from(&x)).or_insert_with(IndexMap::new);
        if let Some(old) = inner_map.get_mut(&x._op) {
            aggregate_update(&x, old);
        } else {
            inner_map.insert(x._op.clone(), x);
        }
    }
    if map.is_empty() {
        return;
    }
    let mut vec: Vec<IndexMap<OpData, ForUpdate>> = map.into_iter().map(|(_k, v)| v).collect();
    let chunk_num = cmp::max(10, vec.len() * 2 / (*crate::connection::SOURCE_MAX_CONNECTIONS as usize) + 1);
    let local = tokio::task::LocalSet::new();
    while !vec.is_empty() {
        let mut buf = vec.split_off(vec.len().saturating_sub(chunk_num));
        local.spawn_local(async move {
            loop {
                let mut conn = DbConn::_new(shard_id);
                if let Err(err) = conn.begin().await {
                    error!(model ="@{ name }@"; "UPDATE DELAYED ERROR:{}", err);
                    sleep(Duration::from_secs(10)).await;
                    continue;
                }
                for inner_map in buf.iter() {
                    for (_op, for_update) in inner_map.iter() {
                        let result = _@{ pascal_name }@::_save(&mut conn, for_update.clone()).await;
                        if is_retry_error(result) {
                            sleep(Duration::from_secs(1)).await;
                            continue;
                        }
                    }
                }
                let result = conn.commit().await;
                if is_retry_error(result) {
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
                break;
            }
        });
    }
    local.await;
}

fn is_retry_error<T>(result: Result<T>) -> bool {
    if let Err(err) = result {
        if let Some(err) = err.downcast_ref::<sqlx::Error>() {
            match err {
                sqlx::Error::Io(..) => {
                    // retry all
                    error!(model ="todo"; "{}", err);
                    return true;
                }
                sqlx::Error::WorkerCrashed => {
                    // retry all
                    error!(model ="todo"; "{}", err);
                    return true;
                }
                _ => {
                    error!(model ="todo"; "{}", err);
                }
            }
        } else {
            error!(model ="todo"; "{}", err);
        }
    }
    false
}

fn aggregate_update(x: &_@{ pascal_name }@ForUpdate, old: &mut _@{ pascal_name }@ForUpdate) {
    @{- def.non_primaries()|fmt_join("
    Accessor{accessor_with_sep_type}::_set(x._op.{var}, &mut old._update.{var}, &x._update.{var});", "") }@
}

async fn _handle_delayed_msg_upsert(shard_id: ShardId) {
    let mut map: BTreeMap<Primary, IndexMap<OpData, ForUpdate>> = BTreeMap::new();
    while let Some(x) = UPSERT_DELAYED_QUEUE.get().unwrap()[shard_id as usize].pop() {
        let inner_map = map.entry(Primary::from(&x)).or_insert_with(IndexMap::new);
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
    let local = tokio::task::LocalSet::new();
    for (op, update, list) in vec {
        local.spawn_local(async move {
            loop {
                let mut conn = DbConn::_new(shard_id);
                if let Err(err) = conn.begin().await {
                    error!(model ="@{ name }@"; "UPSERT DELAYED ERROR:{}", err);
                    sleep(Duration::from_secs(10)).await;
                    continue;
                }
                let mut for_update = ForUpdate {
                    _data: list[0].clone(),
                    _update: update.clone(),
                    _is_new: false,
                    _do_delete: false,
                    _upsert: false,
                    _is_loaded: true,
                    _op: op.clone(),
@{- def.relations_one_owner()|fmt_rel_join("
                    {alias}: None,", "") }@
@{- def.relations_many()|fmt_rel_join("
                    {alias}: None,", "") }@
                };
                @%- if def.updated_at_conf().is_some() %@
                if for_update._op.updated_at == Op::None {
                    for_update.updated_at().set(@{(def.updated_at_conf().unwrap() == Timestampable::RealTime)|if_then_else("SystemTime::now()","conn.time()")}@.into());
                }
                @%- endif %@
                let result = _@{ pascal_name }@::_bulk_upsert(&mut conn, &list, &for_update).await;
                if is_retry_error(result) {
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
                let result = conn.commit().await;
                if is_retry_error(result) {
                    sleep(Duration::from_secs(1)).await;
                    continue;
                }
                break;
            }
        });
    }
    local.await;
}

fn push_delayed_db(list: &Vec<ForInsert>) -> Result<()> {
    if let Some(db) = INSERT_DELAYED_DB.get() {
        let no = DELAYED_DB_NO.fetch_add(1, Ordering::SeqCst);
        let mut buf = encode_all(serde_cbor::to_vec(list)?.as_slice(), 3)?;
        db.insert(&no.to_be_bytes(), buf)?;
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

@% for (name, column_def) in def.id_except_auto_increment() -%@
#[derive(Deserialize, Serialize, Hash, Eq, PartialEq, Clone,@% if column_def.is_copyable() %@ Copy,@% endif %@ Display, Debug, JsonSchema)]
pub struct @{ id_name }@(pub(crate) @{ column_def.get_inner_type(false) }@);
@% endfor -%@
@% for (name, column_def) in def.id_auto_increment() -%@
#[derive(Serialize, Hash, Eq, PartialEq, Clone,@% if column_def.is_copyable() %@ Copy,@% endif %@ Display, Debug, JsonSchema)]
pub struct @{ id_name }@(
    #[schemars(schema_with = "crate::seeder::id_schema")]
    pub(crate) @{ column_def.get_inner_type(false) }@
);

static GENERATED_IDS: OnceCell<RwLock<HashMap<String, @{ id_name }@>>> = OnceCell::new();

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
                Ok(@{ id_name }@(v as @{ column_def.get_inner_type(false) }@))
            }
        }
        deserializer.deserialize_u64(IdVisitor)
    }
}

@% endfor -%@
#[derive(
    sqlx::FromRow, Hash, Eq, PartialEq, Deserialize, Serialize, Clone, Debug, Ord, PartialOrd,
)]
pub(crate) struct Primary(@{ def.primaries()|fmt_join("pub {inner}", ", ") }@);

#[derive(Hash, Eq, PartialEq, Deserialize, Serialize, Clone, Debug)]
struct PrimaryHasher(Primary, ShardId);

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
impl PrimaryHasher {
    fn _shard_id(&self) -> ShardId {
        self.1
    }
    fn to_wrapper(&self, time: MSec) -> PrimaryWrapper {
        PrimaryWrapper(self.0.clone(), self.1, time)
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
struct PrimaryWrapper(Primary, ShardId, MSec);

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
        Ok(serde_cbor::to_vec(self)?)
    }
    fn _decode(v: &[u8]) -> Result<Self> {
        Ok(serde_cbor::from_slice::<Self>(v)?)
    }
}

#[derive(sqlx::FromRow, Validate, Deserialize, Serialize, PartialEq, Clone, Debug, Default, senax_macros::SqlCol)]
pub(crate) struct Data {
@{ def.all_columns()|fmt_join("{default}{rename}{validate}    pub {var}: {inner},\n", "") -}@
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

#[derive(Deserialize, Serialize, Clone, Debug, Default, Eq, PartialEq, Hash)]
pub(crate) struct OpData {
@{- def.non_primaries()|fmt_join("
    #[serde(default, skip_serializing_if = \"Op::is_none\")]
    pub {var}: Op,", "") }@
}

#[derive(Serialize, Deserialize, sqlx::FromRow, Clone, Debug, senax_macros::SqlCol)]
pub(crate) struct CacheData {
@{ def.cache_cols()|fmt_join("{default}{rename}    pub {var}: {inner},\n", "") -}@
}

@% for (name, column_def) in def.enums() -%@
@% if column_def.enum_values.is_some() -%@
@% let values = column_def.enum_values.as_ref().unwrap() -%@
#[derive(Serialize_repr, Deserialize_repr, Hash, Eq, PartialEq, Clone, Copy, Debug, strum::Display, EnumMessage, EnumString, IntoStaticStr, JsonSchema)]
#[repr(u8)]
#[allow(non_camel_case_types)]
pub enum _@{ name|to_pascal_name }@ {
@% for row in values -%@@{ row.title|comment4 }@@{ row.comment|comment4 }@@{ row.title|strum_message4 }@@{ row.comment|strum_detailed4 }@    @{ row.name }@ = @{ row.value }@,
@% endfor -%@
}
impl _@{ name|to_pascal_name }@ {
    pub fn get(&self) -> u8 {
        *self as u8
    }
}
impl From<u8> for _@{ name|to_pascal_name }@ {
    fn from(val: u8) -> Self {
        match val {
@% for row in values %@            @{ row.value }@ => _@{ name|to_pascal_name }@::@{ row.name }@,
@% endfor %@            _ => panic!("{} is a value outside the range of _@{ name|to_pascal_name }@.", val),
        }
    }
}
impl From<_@{ name|to_pascal_name }@> for u8 {
    fn from(val: _@{ name|to_pascal_name }@) -> Self {
        val.get()
    }
}
impl From<_@{ name|to_pascal_name }@> for BindValue {
    fn from(val: _@{ name|to_pascal_name }@) -> Self {
        Self::Enum(Some(val.get()))
    }
}
impl From<Option<_@{ name|to_pascal_name }@>> for BindValue {
    fn from(val: Option<_@{ name|to_pascal_name }@>) -> Self {
        Self::Enum(val.map(|t| t.get()))
    }
}

@% endif -%@
@% endfor -%@
@% for (name, column_def) in def.db_enums() -%@
@% if column_def.db_enum_values.is_some() -%@
@% let values = column_def.db_enum_values.as_ref().unwrap() -%@
#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Clone, Copy, Debug, strum::Display, EnumMessage, EnumString, IntoStaticStr)]
#[allow(non_camel_case_types)]
pub enum _@{ name|to_pascal_name }@ {
@% for row in values -%@@{ row.title|comment4 }@@{ row.comment|comment4 }@@{ row.title|strum_message4 }@@{ row.comment|strum_detailed4 }@    @{ row.name }@,
@% endfor -%@
}

@% endif -%@
@% endfor -%@
@{ def.title|comment0 -}@
@{ def.comment|comment0 -}@
#[derive(Clone, Debug)]
pub struct _@{ pascal_name }@ {
    pub(crate) _inner: Data,
@{ def.relations_one_except_cache()|fmt_rel_join("    pub(crate) {alias}: Option<Option<Box<rel_{class_mod}::{class}>>>,\n", "") -}@@# Boxにしないと無限参照 -#@
@{ def.relations_one_only_cache()|fmt_rel_join("    pub(crate) {alias}: Option<Option<Box<rel_{class_mod}::{class}Cache>>>,\n", "") -}@
@{ def.relations_many()|fmt_rel_join("    pub(crate) {alias}: Option<Vec<rel_{class_mod}::{class}>>,\n", "") -}@
}

#[derive(Serialize, Deserialize, Hash, Eq, PartialEq, Clone, Copy, Debug, strum::Display, EnumMessage, EnumString, IntoStaticStr, strum_macros::EnumIter, strum_macros::EnumProperty)]
#[allow(non_camel_case_types)]
pub enum _@{ pascal_name }@Info {
@%- for (col_name, column_def) in def.all_columns() %@
@{ column_def.title|strum_message4 }@@{ column_def.comment|strum_detailed4 }@@{ column_def|strum_props4 }@    @{ col_name|to_var_name }@,
@%- endfor %@
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct CacheWrapper {
    pub _inner: CacheData,
    _shard_id: ShardId,
    _time: MSec,
@{ def.relations_one_cache()|fmt_rel_join("    pub {alias}: Option<Arc<rel_{class_mod}::CacheWrapper>>,\n", "") -}@
@{ def.relations_many_cache()|fmt_rel_join("    pub {alias}: Vec<Arc<rel_{class_mod}::CacheWrapper>>,\n", "") -}@
}

#[derive(Clone, Debug)]
pub struct _@{ pascal_name }@Cache {
    pub(crate) _wrapper: Arc<CacheWrapper>,
@{ def.relations_one_only_cache()|fmt_rel_join("    pub(crate) {alias}: Option<Option<Box<rel_{class_mod}::{class}Cache>>>,\n", "") -}@
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub(crate) struct VersionWrapper {
    id: Primary,
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
        Ok(serde_cbor::to_vec(self)?)
    }
    fn _decode(v: &[u8]) -> Result<Self> {
        Ok(serde_cbor::from_slice::<Self>(v)?)
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

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct _@{ pascal_name }@Factory {
@{ def.for_factory()|fmt_join("{title}{comment}{factory_default}    pub {var}: {factory},", "\n") }@
}

#[derive(Clone, Debug)]
pub struct _@{ pascal_name }@ForUpdate {
    pub(crate) _data: Data,
    pub(crate) _update: Data,
    pub(crate) _is_new: bool,
    pub(crate) _do_delete: bool,
    pub(crate) _upsert: bool,
    pub(crate) _is_loaded: bool,
    pub(crate) _op: OpData,
@{ def.relations_one_owner()|fmt_rel_join("    pub(crate) {alias}: Option<Option<Box<rel_{class_mod}::{class}ForUpdate>>>,\n", "") -}@
@{ def.relations_many()|fmt_rel_join("    pub(crate) {alias}: Option<Vec<rel_{class_mod}::{class}ForUpdate>>,\n", "") -}@
}
type ForUpdate = _@{ pascal_name }@ForUpdate;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub(crate) struct ForInsert {
    pub _data: Data,
@{- def.relations_one_owner()|fmt_rel_join("
    #[serde(skip_serializing_if = \"Option::is_none\")]
    pub {alias}: Option<Option<Box<rel_{class_mod}::ForInsert>>>,", "") }@
@{- def.relations_many()|fmt_rel_join("
    #[serde(skip_serializing_if = \"Option::is_none\")]
    pub {alias}: Option<Vec<rel_{class_mod}::ForInsert>>,", "") }@
}

impl From<ForUpdate> for ForInsert {
    fn from(v: ForUpdate) -> Self {
        Self {
            _data: v._data,
            @{- def.relations_one_owner()|fmt_rel_join("
            {alias}: v.{alias}.map(|v| v.map(|v| v.into())),", "") }@
            @{- def.relations_many()|fmt_rel_join("
            {alias}: v.{alias}.map(|v| v.into_iter().map(|v| v.into()).collect()),", "") }@
        }
    }
}

impl From<Box<ForUpdate>> for Box<ForInsert> {
    fn from(v: Box<ForUpdate>) -> Self {
        Box::new(ForInsert {
            _data: v._data,
            @{- def.relations_one_owner()|fmt_rel_join("
            {alias}: v.{alias}.map(|v| v.map(|v| v.into())),", "") }@
            @{- def.relations_many()|fmt_rel_join("
            {alias}: v.{alias}.map(|v| v.into_iter().map(|v| v.into()).collect()),", "") }@
        })
    }
}

pub trait _@{ pascal_name }@Tr {
@{ def.all_columns()|fmt_join("{title}{comment}    fn {var}(&self) -> {outer};
", "") -}@
@{ def.relations_one_except_cache()|fmt_rel_join("{title}{comment}    fn {alias}(&self) -> Option<&rel_{class_mod}::{class}>;
", "") -}@
@{ def.relations_one_only_cache()|fmt_rel_join("{title}{comment}    fn {alias}(&self) -> Option<&rel_{class_mod}::{class}Cache>;
", "") -}@
@{ def.relations_many()|fmt_rel_join("{title}{comment}    fn {alias}(&self) -> &Vec<rel_{class_mod}::{class}>;
", "") -}@
}

pub trait _@{ pascal_name }@MutTr {
@{ def.all_columns()|fmt_join("{title}{comment}    fn {var}(&mut self) -> {outer};
", "") -}@
@{ def.relations_one_except_cache()|fmt_rel_join("{title}{comment}    fn {alias}(&mut self) -> Option<&mut rel_{class_mod}::{class}>;
", "") -}@
@{ def.relations_one_only_cache()|fmt_rel_join("{title}{comment}    fn {alias}(&mut self) -> Option<&mut rel_{class_mod}::{class}Cache>;
", "") -}@
@{ def.relations_many()|fmt_rel_join("{title}{comment}    fn {alias}(&mut self) -> &mut Vec<rel_{class_mod}::{class}>;
", "") -}@
}
    
#[async_trait(?Send)]
pub trait _@{ pascal_name }@Rel {
@{ def.relations_one()|fmt_rel_join("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        unimplemented!(\"fetch_{raw_alias} is not implemented\")
    }\n", "") -}@
@{ def.relations_many()|fmt_rel_join("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        unimplemented!(\"fetch_{raw_alias} is not implemented\")
    }\n", "") -}@
}

#[async_trait(?Send)]
impl _@{ pascal_name }@Rel for _@{ pascal_name }@ {
@{ def.relations_one_except_cache()|fmt_rel_join_not_null_or_null("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        if self.{alias}.is_some() {
            return Ok(());
        }
        self.{alias} = Some(
            rel_{class_mod}::{class}::find_optional(conn, &self.{var}()).await?.map(Box::new)
        );
        Ok(())
    }\n", "    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        if self.{alias}.is_some() {
            return Ok(());
        }
        if let Some(id) = self.{var}() {
            self.{alias} = Some(
                rel_{class_mod}::{class}::find_optional(conn, &id).await?.map(Box::new)
            );
        } else {
            self.{alias} = Some(None);
        }
        Ok(())
    }\n", "") -}@
@{ def.relations_one_only_cache()|fmt_rel_join_not_null_or_null("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        if self.{alias}.is_some() {
            return Ok(());
        }
        self.{alias} = Some(
            rel_{class_mod}::{class}::find_optional_from_cache{with_trashed}(conn, &self.{var}()).await?.map(Box::new)
        );
        Ok(())
    }\n", "    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        if self.{alias}.is_some() {
            return Ok(());
        }
        if let Some(id) = self.{var}() {
            self.{alias} = Some(
                rel_{class_mod}::{class}::find_optional_from_cache{with_trashed}(conn, &id).await?.map(Box::new)
            );
        } else {
            self.{alias} = Some(None);
        }
        Ok(())
    }\n", "") -}@
@{ def.relations_many()|fmt_rel_join("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        if self.{alias}.is_some() {
            return Ok(());
        }
        let cond = rel_{class_mod}::Cond::Eq(rel_{class_mod}::ColOne::{foreign_var}(self.{local_id}())){and_cond};
        let order_by = vec![{order_by}];
        self.{alias} = Some(rel_{class_mod}::{class}::query().cond(cond).order_by(order_by){limit}.select(conn).await?);
        Ok(())
    }\n", "") -}@
}

#[async_trait(?Send)]
impl _@{ pascal_name }@Rel for _@{ pascal_name }@ForUpdate {
@{ def.relations_one_owner()|fmt_rel_join_not_null_or_null("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        if self.{alias}.is_some() {
            return Ok(());
        }
        self.{alias} = Some(
            rel_{class_mod}::{class}::find_optional_for_update(conn, &self.{var}().get()).await?.map(Box::new)
        );
        Ok(())
    }\n", "    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        if self.{alias}.is_some() {
            return Ok(());
        }
        if let Some(id) = self.{var}().get() {
            self.{alias} = Some(
                rel_{class_mod}::{class}::find_optional_for_update(conn, &id).await?.map(Box::new)
            );
        } else {
            self.{alias} = Some(None);
        }
        Ok(())
    }\n", "") -}@
@{ def.relations_many()|fmt_rel_join("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        if self.{alias}.is_some() {
            return Ok(());
        }
        let cond = rel_{class_mod}::Cond::Eq(rel_{class_mod}::ColOne::{foreign_var}(self.{local_id}().get())){and_cond};
        let order_by = vec![{order_by}];
        self.{alias} = Some(rel_{class_mod}::{class}::query().cond(cond).order_by(order_by).select_for_update(conn).await?);
        Ok(())
    }\n", "") -}@
}

impl CacheWrapper {
@{ def.relations_one_cache()|fmt_rel_join_not_null_or_null("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        self.{alias} = rel_{class_mod}::{class}::find_optional_for_cache(conn, &self.{var}()).await?.map(|v| v._wrapper);
        Ok(())
    }\n", "    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        if let Some(id) = self.{var}() {
            self.{alias} = rel_{class_mod}::{class}::find_optional_for_cache(conn, &id).await?.into_iter().map(|v| v._wrapper);
        } else {
            self.{alias} = None;
        }
        Ok(())
    }\n", "") -}@
@{ def.relations_many_cache()|fmt_rel_join("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        let cond = rel_{class_mod}::Cond::Eq(rel_{class_mod}::ColOne::{foreign_var}(self.{local_id}())){and_cond};
        let order_by = vec![{order_by}];
        self.{alias} = rel_{class_mod}::{class}::query().cond(cond).order_by(order_by){limit}.select_for_cache(conn).await?.into_iter().map(|v| v._wrapper).collect();
        self.{alias}.shrink_to_fit();
        Ok(())
    }\n", "") -}@
}

#[async_trait(?Send)]
impl _@{ pascal_name }@Rel for _@{ pascal_name }@Cache {
@{ def.relations_one_only_cache()|fmt_rel_join_not_null_or_null("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        self.{alias} = Some(
            rel_{class_mod}::{class}::find_optional_from_cache{with_trashed}(conn, &self.{var}()).await?.map(Box::new)
        );
        Ok(())
    }\n", "    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        if let Some(id) = self.{var}() {
            self.{alias} = Some(
                rel_{class_mod}::{class}::find_optional_from_cache{with_trashed}(conn, &id).await?.map(Box::new)
            );
        } else {
            self.{alias} = Some(None);
        }
        Ok(())
    }\n", "") -}@
}

#[async_trait(?Send)]
impl _@{ pascal_name }@Rel for Vec<_@{ pascal_name }@> {
@{ def.relations_one_except_cache()|fmt_rel_join_not_null_or_null("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        let ids: FxHashSet<_> = self.iter().map(|v| v.{var}()).collect();
        if ids.is_empty() { return Ok(()); }
        let map = rel_{class_mod}::{class}::find_many(conn, ids.iter()).await?;
        for val in self.iter_mut() {
            val.{alias} = Some(map.get(&val.{var}()).map(|v| Box::new(v.clone())));
        }
        Ok(())
    }\n", "    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        let ids: FxHashSet<_> = self.iter().flat_map(|v| v.{var}()).collect();
        if ids.is_empty() { return Ok(()); }
        let map = rel_{class_mod}::{class}::find_many(conn, ids.iter()).await?;
        for val in self.iter_mut() {
            if let Some(id) = val.{var}() {
                val.{alias} = Some(map.get(&id).map(|v| Box::new(v.clone())));
            }
        }
        Ok(())
    }\n", "") -}@
@{ def.relations_one_only_cache()|fmt_rel_join_not_null_or_null("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        let ids: FxHashSet<_> = self.iter().map(|v| v.{var}()).collect();
        if ids.is_empty() { return Ok(()); }
        let map = rel_{class_mod}::{class}::find_many_from_cache{with_trashed}(conn, ids.iter()).await?;
        for val in self.iter_mut() {
            val.{alias} = Some(map.get(&val.{var}()).map(|v| Box::new(v.clone())));
        }
        Ok(())
    }\n", "    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        let ids: FxHashSet<_> = self.iter().flat_map(|v| v.{var}()).collect();
        if ids.is_empty() { return Ok(()); }
        let map = rel_{class_mod}::{class}::find_many_from_cache{with_trashed}(conn, ids.iter()).await?;
        for val in self.iter_mut() {
            if let Some(id) = val.{var}() {
                val.{alias} = Some(map.get(&id).map(|v| Box::new(v.clone())));
            }
        }
        Ok(())
    }\n", "") -}@
@{ def.relations_many()|fmt_rel_join_foreign_is_not_null_or_null("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        if self.is_empty() { return Ok(()); }
        let union: Vec<_> = self.iter().map(|v| {
            let cond = rel_{class_mod}::Cond::Eq(rel_{class_mod}::ColOne::{foreign_var}(v.{local_id}())){and_cond};
            let order_by = vec![{order_by}];
            rel_{class_mod}::{class}::query().cond(cond).order_by(order_by){limit}
        }).collect();
        use rel_{class_mod}::UnionBuilder;
        let list = union.select(conn).await?;
        let mut map = FxHashMap::default();
        for row in list {
            map.entry(row._inner.{foreign})
                .or_insert_with(Vec::new)
                .push(row);
        }
        for val in self.iter_mut() {
            val.{alias} = Some(
                map.remove(&val._inner.{local_id}).unwrap_or_default(),
            );
        }
        Ok(())
    }\n", "    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        if self.is_empty() { return Ok(()); }
        let union: Vec<_> = self.iter().map(|v| {
            let cond = rel_{class_mod}::Cond::Eq(rel_{class_mod}::ColOne::{foreign_var}(v.{local_id}())){and_cond};
            let order_by = vec![{order_by}];
            rel_{class_mod}::{class}::query().cond(cond).order_by(order_by){limit}
        }).collect();
        use rel_{class_mod}::UnionBuilder;
        let list = union.select(conn).await?;
        let mut map = FxHashMap::default();
        for row in list {
            if let Some(id) = row._inner.{foreign} {
                map.entry(id).or_insert_with(Vec::new).push(row);
            }
        }
        for val in self.iter_mut() {
            val.{alias} = Some(
                map.remove(&val._inner.{local_id}).unwrap_or_default(),
            );
        }
        Ok(())
    }\n", "") -}@
}

#[async_trait(?Send)]
impl _@{ pascal_name }@Rel for Vec<&mut _@{ pascal_name }@> {
@{ def.relations_one_except_cache()|fmt_rel_join_not_null_or_null("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        let ids: FxHashSet<_> = self.iter_mut().map(|v| v.{var}()).collect();
        if ids.is_empty() { return Ok(()); }
        let map = rel_{class_mod}::{class}::find_many(conn, ids.iter()).await?;
        for val in self.iter_mut() {
            val.{alias} = Some(map.get(&val.{var}()).map(|v| Box::new(v.clone())));
        }
        Ok(())
    }\n", "    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        let ids: FxHashSet<_> = self.iter_mut().flat_map(|v| v.{var}()).collect();
        if ids.is_empty() { return Ok(()); }
        let map = rel_{class_mod}::{class}::find_many(conn, ids.iter()).await?;
        for val in self.iter_mut() {
            if let Some(id) = val.{var}() {
                val.{alias} = Some(map.get(&id).map(|v| Box::new(v.clone())));
            }
        }
        Ok(())
    }\n", "") -}@
@{ def.relations_one_only_cache()|fmt_rel_join_not_null_or_null("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        let ids: FxHashSet<_> = self.iter_mut().map(|v| v.{var}()).collect();
        if ids.is_empty() { return Ok(()); }
        let map = rel_{class_mod}::{class}::find_many_from_cache{with_trashed}(conn, ids.iter()).await?;
        for val in self.iter_mut() {
            val.{alias} = Some(map.get(&val.{var}()).map(|v| Box::new(v.clone())));
        }
        Ok(())
    }\n", "    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        let ids: FxHashSet<_> = self.iter_mut().flat_map(|v| v.{var}()).collect();
        if ids.is_empty() { return Ok(()); }
        let map = rel_{class_mod}::{class}::find_many_from_cache{with_trashed}(conn, ids.iter()).await?;
        for val in self.iter_mut() {
            if let Some(id) = val.{var}() {
                val.{alias} = Some(map.get(&id).map(|v| Box::new(v.clone())));
            }
        }
        Ok(())
    }\n", "") -}@
@{ def.relations_many()|fmt_rel_join_foreign_is_not_null_or_null("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        if self.is_empty() { return Ok(()); }
        let union: Vec<_> = self.iter_mut().map(|v| {
            let cond = rel_{class_mod}::Cond::Eq(rel_{class_mod}::ColOne::{foreign_var}(v.{local_id}())){and_cond};
            let order_by = vec![{order_by}];
            rel_{class_mod}::{class}::query().cond(cond).order_by(order_by){limit}
        }).collect();
        use rel_{class_mod}::UnionBuilder;
        let list = union.select(conn).await?;
        let mut map = FxHashMap::default();
        for row in list {
            map.entry(row._inner.{foreign})
                .or_insert_with(Vec::new)
                .push(row);
        }
        for val in self.iter_mut() {
            val.{alias} = Some(
                map.remove(&val._inner.{local_id}).unwrap_or_default(),
            );
        }
        Ok(())
    }\n", "    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        if self.is_empty() { return Ok(()); }
        let union: Vec<_> = self.iter_mut().map(|v| {
            let cond = rel_{class_mod}::Cond::Eq(rel_{class_mod}::ColOne::{foreign_var}(v.{local_id}())){and_cond};
            let order_by = vec![{order_by}];
            rel_{class_mod}::{class}::query().cond(cond).order_by(order_by){limit}
        }).collect();
        use rel_{class_mod}::UnionBuilder;
        let list = union.select(conn).await?;
        let mut map = FxHashMap::default();
        for row in list {
            if let Some(id) = row._inner.{foreign} {
                map.entry(id).or_insert_with(Vec::new).push(row);
            }
        }
        for val in self.iter_mut() {
            val.{alias} = Some(
                map.remove(&val._inner.{local_id}).unwrap_or_default(),
            );
        }
        Ok(())
    }\n", "") -}@
}

#[async_trait(?Send)]
impl _@{ pascal_name }@Rel for Vec<_@{ pascal_name }@ForUpdate> {
@{ def.relations_one_owner()|fmt_rel_join_not_null_or_null("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        let ids: FxHashSet<_> = self.iter().map(|v| v.{var}().get()).collect();
        if ids.is_empty() { return Ok(()); }
        let map = rel_{class_mod}::{class}::find_many_for_update(conn, ids.iter()).await?;
        for val in self.iter_mut() {
            val.{alias} = Some(map.get(&val.{var}().get()).map(|v| Box::new(v.clone())));
        }
        Ok(())
    }\n", "    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        let ids: FxHashSet<_> = self.iter().flat_map(|v| v.{var}().get()).collect();
        if ids.is_empty() { return Ok(()); }
        let map = rel_{class_mod}::{class}::find_many_for_update(conn, ids.iter()).await?;
        for val in self.iter_mut() {
            if let Some(id) = val.{var}().get() {
                val.{alias} = Some(map.get(&id).map(|v| Box::new(v.clone())));
            }
        }
        Ok(())
    }\n", "") -}@
@{ def.relations_many()|fmt_rel_join_foreign_is_not_null_or_null("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        let ids: Vec<_> = self.iter().map(|v| v.{local_id}().get()).collect();
        if ids.is_empty() { return Ok(()); }
        let cond = rel_{class_mod}::Cond::In(rel_{class_mod}::ColMany::{foreign_var}(ids)){and_cond};
        let order_by = vec![{order_by}];
        let list = rel_{class_mod}::{class}::query().cond(cond).order_by(order_by).select_for_update(conn).await?;
        let mut map = FxHashMap::default();
        for row in list {
            map.entry(row._data.{foreign})
                .or_insert_with(Vec::new)
                .push(row);
        }
        for val in self.iter_mut() {
            val.{alias} = Some(
                map.remove(&val._data.{local_id}).unwrap_or_default(),
            );
        }
        Ok(())
    }\n", "    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        let ids: Vec<_> = self.iter().map(|v| v.{local_id}().get()).collect();
        if ids.is_empty() { return Ok(()); }
        let cond = rel_{class_mod}::Cond::In(rel_{class_mod}::ColMany::{foreign_var}(ids)){and_cond};
        let order_by = vec![{order_by}];
        let list = rel_{class_mod}::{class}::query().cond(cond).order_by(order_by).select_for_update(conn).await?;
        let mut map = FxHashMap::default();
        for row in list {
            if let Some(id) = row._data.{foreign} {
                map.entry(id).or_insert_with(Vec::new).push(row);
            }
        }
        for val in self.iter_mut() {
            val.{alias} = Some(
                map.remove(&val._data.{local_id}).unwrap_or_default(),
            );
        }
        Ok(())
    }\n", "") -}@
}

#[async_trait(?Send)]
impl _@{ pascal_name }@Rel for Vec<&mut _@{ pascal_name }@ForUpdate> {
@{ def.relations_one_owner()|fmt_rel_join_not_null_or_null("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        let ids: FxHashSet<_> = self.iter().map(|v| v.{var}().get()).collect();
        if ids.is_empty() { return Ok(()); }
        let map = rel_{class_mod}::{class}::find_many_for_update(conn, ids.iter()).await?;
        for val in self.iter_mut() {
            val.{alias} = Some(map.get(&val.{var}().get()).map(|v| Box::new(v.clone())));
        }
        Ok(())
    }\n", "    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        let ids: FxHashSet<_> = self.iter().flat_map(|v| v.{var}().get()).collect();
        if ids.is_empty() { return Ok(()); }
        let map = rel_{class_mod}::{class}::find_many_for_update(conn, ids.iter()).await?;
        for val in self.iter_mut() {
            if let Some(id) = val.{var}().get() {
                val.{alias} = Some(map.get(&id).map(|v| Box::new(v.clone())));
            }
        }
        Ok(())
    }\n", "") -}@
@{ def.relations_many()|fmt_rel_join_foreign_is_not_null_or_null("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        let ids: Vec<_> = self.iter().map(|v| v.{local_id}().get()).collect();
        if ids.is_empty() { return Ok(()); }
        let cond = rel_{class_mod}::Cond::In(rel_{class_mod}::ColMany::{foreign_var}(ids)){and_cond};
        let order_by = vec![{order_by}];
        let list = rel_{class_mod}::{class}::query().cond(cond).order_by(order_by).select_for_update(conn).await?;
        let mut map = FxHashMap::default();
        for row in list {
            map.entry(row._data.{foreign})
                .or_insert_with(Vec::new)
                .push(row);
        }
        for val in self.iter_mut() {
            val.{alias} = Some(
                map.remove(&val._data.{local_id}).unwrap_or_default(),
            );
        }
        Ok(())
    }\n", "    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        let ids: Vec<_> = self.iter().map(|v| v.{local_id}().get()).collect();
        if ids.is_empty() { return Ok(()); }
        let cond = rel_{class_mod}::Cond::In(rel_{class_mod}::ColMany::{foreign_var}(ids)){and_cond};
        let order_by = vec![{order_by}];
        let list = rel_{class_mod}::{class}::query().cond(cond).order_by(order_by).select_for_update(conn).await?;
        let mut map = FxHashMap::default();
        for row in list {
            if let Some(id) = row._data.{foreign} {
                map.entry(id).or_insert_with(Vec::new).push(row);
            }
        }
        for val in self.iter_mut() {
            val.{alias} = Some(
                map.remove(&val._data.{local_id}).unwrap_or_default(),
            );
        }
        Ok(())
    }\n", "") -}@
}

impl CacheWrapper {
@{ def.relations_one_cache()|fmt_rel_join_not_null_or_null("    async fn fetch_{raw_alias}_4vec(vec: &mut Vec<CacheWrapper>, conn: &mut DbConn) -> Result<()> {
        let ids: FxHashSet<_> = vec.iter().map(|v| v.{var}()).collect();
        if ids.is_empty() { return Ok(()); }
        let map: FxHashMap<_, _> = rel_{class_mod}::{class}::find_many_for_cache(conn, ids.iter()).await?.into_iter().map(|(k, v)| (k, v._wrapper)).collect();
        for val in vec.iter_mut() {
            val.{alias} = map.get(&val.{var}()).cloned();
        }
        Ok(())
    }\n", "    async fn fetch_{raw_alias}_4vec(vec: &mut Vec<CacheWrapper>, conn: &mut DbConn) -> Result<()> {
        let ids: FxHashSet<_> = vec.iter().flat_map(|v| v.{var}()).collect();
        if ids.is_empty() { return Ok(()); }
        let map: FxHashMap<_, _> = rel_{class_mod}::{class}::find_many_for_cache(conn, ids.iter()).await?.into_iter().map(|(k, v)| (k, v._wrapper)).collect();
        for val in vec.iter_mut() {
            if let Some(id) = val.{var}() {
                val.{alias} = map.get(&id).cloned();
            }
        }
        Ok(())
    }\n", "") -}@
@{ def.relations_many_cache()|fmt_rel_join_foreign_is_not_null_or_null("    async fn fetch_{raw_alias}_4vec(vec: &mut Vec<CacheWrapper>, conn: &mut DbConn) -> Result<()> {
        if vec.is_empty() { return Ok(()); }
        let union: Vec<_> = vec.iter().map(|v| {
            let cond = rel_{class_mod}::Cond::Eq(rel_{class_mod}::ColOne::{foreign_var}(v.{local_id}())){and_cond};
            let order_by = vec![{order_by}];
            rel_{class_mod}::{class}::query().cond(cond).order_by(order_by){limit}
        }).collect();
        use rel_{class_mod}::UnionBuilder;
        let list: Vec<Arc<_>> = union.select_for_cache(conn).await?.into_iter().map(|v| v._wrapper).collect();
        let mut map = FxHashMap::default();
        for row in list {
            map.entry(row._inner.{foreign})
                .or_insert_with(Vec::new)
                .push(row);
        }
        for val in vec.iter_mut() {
            val.{alias} = map.remove(&val._inner.{local_id}).unwrap_or_default();
            val.{alias}.shrink_to_fit();
        }
        Ok(())
    }\n", "    async fn fetch_{raw_alias}_4vec(vec: &mut Vec<CacheWrapper>, conn: &mut DbConn) -> Result<()> {
        if vec.is_empty() { return Ok(()); }
        let union: Vec<_> = vec.iter().map(|v| {
            let cond = rel_{class_mod}::Cond::Eq(rel_{class_mod}::ColOne::{foreign_var}(v.{local_id}())){and_cond};
            let order_by = vec![{order_by}];
            rel_{class_mod}::{class}::query().cond(cond).order_by(order_by){limit}
        }).collect();
        use rel_{class_mod}::UnionBuilder;
        let list: Vec<Arc<_>> = union.select_for_cache(conn).await?.into_iter().map(|v| v._wrapper).collect();
        let mut map = FxHashMap::default();
        for row in list {
            if let Some(id) = row._inner.{foreign} {
                map.entry(id).or_insert_with(Vec::new).push(row);
            }
        }
        for val in vec.iter_mut() {
            val.{alias} = map.remove(&val._inner.{local_id}).unwrap_or_default();
            val.{alias}.shrink_to_fit();
        }
        Ok(())
    }\n", "") -}@
}

#[async_trait(?Send)]
impl _@{ pascal_name }@Rel for Vec<_@{ pascal_name }@Cache> {
@{ def.relations_one_only_cache()|fmt_rel_join_not_null_or_null("    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        let mut ids = FxHashSet::default();
        for val in self.iter() {
            ids.insert(val.{var}());
        }
        if ids.is_empty() { return Ok(()); }
        let map = rel_{class_mod}::{class}::find_many_from_cache{with_trashed}(conn, ids.iter()).await?;
        for val in self.iter_mut() {
            val.{alias} = Some(map.get(&val.{var}()).map(|v| Box::new(v.clone())));
        }
        Ok(())
    }\n", "    async fn fetch_{raw_alias}(&mut self, conn: &mut DbConn) -> Result<()> {
        let mut ids = FxHashSet::default();
        for val in self.iter() {
            if let Some(id) = val.{var}() {
                ids.insert(id);
            }
        }
        if ids.is_empty() { return Ok(()); }
        let map = rel_{class_mod}::{class}::find_many_from_cache{with_trashed}(conn, ids.iter()).await?;
        for val in self.iter_mut() {
            if let Some(id) = val.{var}() {
                val.{alias} = Some(map.get(&id).map(|v| Box::new(v.clone())));
            }
        }
        Ok(())
    }\n", "") -}@
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum Col {
@{ def.all_columns()|fmt_join("    {var},", "\n") }@
}
impl Col {
    fn name(&self) -> &'static str {
        match self {
@{ def.all_columns()|fmt_join("            Col::{var} => \"{col_esc}\",", "\n") }@
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColOne {
@{ def.all_columns_without_json()|fmt_join("    {var}({cond_type}),", "\n") }@
}
impl ColOne {
    fn name(&self) -> &'static str {
        match self {
@{ def.all_columns_without_json()|fmt_join("            ColOne::{var}(_v) => \"{col_esc}\",", "\n") }@
            _ => unreachable!(),
        }
    }
    fn bind<T>(
        self,
        query: QueryAs<'_, DbType, T, DbArguments>,
    ) -> QueryAs<'_, DbType, T, DbArguments> {
        debug!("bind:{:?}", &self);
        match self {
@{ def.all_columns_without_json()|fmt_join("            ColOne::{var}(v) => query.bind(v{bind_as}),", "\n") }@
            _ => unreachable!(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Hash, Serialize)]
pub enum ColKey {
    @{- def.unique_key()|fmt_index_col("
    {var}({cond_type}),", "") }@
}
impl ColKey {
    fn name(&self) -> &'static str {
        match self {
            @{- def.unique_key()|fmt_index_col("
            ColKey::{var}(_v) => \"{col_esc}\",", "") }@
            _ => unreachable!(),
        }
    }
    fn bind<T>(
        self,
        query: QueryAs<'_, DbType, T, DbArguments>,
    ) -> QueryAs<'_, DbType, T, DbArguments> {
        debug!("bind:{:?}", &self);
        match self {
            @{- def.unique_key()|fmt_index_col("
            ColKey::{var}(v) => query.bind(v{bind_as}),", "") }@
            _ => unreachable!(),
        }
    }
}
struct VecColKey(Vec<ColKey>);
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

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColMany {
@{ def.all_columns_without_json()|fmt_join("    {var}(Vec<{cond_type}>),", "\n") }@
}
impl ColMany {
    fn name(&self) -> &'static str {
        match self {
@{ def.all_columns_without_json()|fmt_join("            ColMany::{var}(_v) => \"{col_esc}\",", "\n") }@
            _ => unreachable!(),
        }
    }
    fn len(&self) -> usize {
        match self {
@{ def.all_columns_without_json()|fmt_join("            ColMany::{var}(v) => v.len(),", "\n") }@
            _ => unreachable!(),
        }
    }
    fn bind<T>(
        self,
        query: QueryAs<'_, DbType, T, DbArguments>,
    ) -> QueryAs<'_, DbType, T, DbArguments> {
        debug!("bind:{:?}", &self);
        match self {
@{ def.all_columns_without_json()|fmt_join("            ColMany::{var}(v) => v.into_iter().fold(query, |query, v| query.bind(v{bind_as})),", "\n") }@
            _ => unreachable!(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColJson {
@{- def.all_columns_only_json()|fmt_join("
    {var}(Value),", "") }@
}
impl ColJson {
    fn name(&self) -> &'static str {
        match self {
@{- def.all_columns_only_json()|fmt_join("
            ColJson::{var}(_v) => \"{col_esc}\",", "") }@
            _ => unreachable!(),
        }
    }
    fn bind<T>(
        self,
        query: QueryAs<'_, DbType, T, DbArguments>,
    ) -> QueryAs<'_, DbType, T, DbArguments> {
        debug!("bind:{:?}", &self);
        match self {
@{- def.all_columns_only_json()|fmt_join("
            ColJson::{var}(v) => query.bind(sqlx::types::Json(v{bind_as})),", "") }@
            _ => unreachable!(),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug)]
pub enum ColRel {
@{- def.relations_one()|fmt_rel_join("\n    {alias_pascal}(Option<Box<rel_{class_mod}::Cond>>),", "") }@
@{- def.relations_many()|fmt_rel_join("\n    {alias_pascal}(Option<Box<rel_{class_mod}::Cond>>),", "") }@
}

impl ColRel {
    #[allow(unused_mut)]
    #[allow(clippy::ptr_arg)]
    pub(crate) fn write(&self, buf: &mut String, idx: i32) {
@%- if def.relations_one().len() + def.relations_many().len() > 0 %@
        match self {
@{- def.relations_one()|fmt_rel_join("
            ColRel::{alias_pascal}(c) => {
                let _ = write!(buf, \"SELECT * FROM {table} as _t{} WHERE `{}`=_t{}.{col_esc} AND \", idx + 1, rel_{class_mod}::ID_COLUMN, idx);
                let mut trash_mode = TrashMode::Not;
                if let Some(cond) = c {
                    cond.write(buf, idx + 1, &mut trash_mode);
                }
                if trash_mode == TrashMode::Not {
                    buf.push_str(rel_{class_mod}::NOT_TRASHED_SQL)
                } else if trash_mode == TrashMode::Only {
                    buf.push_str(rel_{class_mod}::ONLY_TRASHED_SQL)
                } else {
                    buf.push_str(rel_{class_mod}::TRASHED_SQL)
                }
                buf.truncate(buf.len() - 5);
            }", "") }@
@{- def.relations_many()|fmt_rel_join("
            ColRel::{alias_pascal}(c) => {
                let _ = write!(buf, \"SELECT * FROM {table} as _t{} WHERE _t{}.`{}`={foreign_esc} AND \", idx + 1, idx, ID_COLUMN);
                let mut trash_mode = TrashMode::Not;
                if let Some(cond) = c {
                    cond.write(buf, idx + 1, &mut trash_mode);
                }
                if trash_mode == TrashMode::Not {
                    buf.push_str(rel_{class_mod}::NOT_TRASHED_SQL)
                } else if trash_mode == TrashMode::Only {
                    buf.push_str(rel_{class_mod}::ONLY_TRASHED_SQL)
                } else {
                    buf.push_str(rel_{class_mod}::TRASHED_SQL)
                }
                buf.truncate(buf.len() - 5);
            }", "") }@
        };
@%- endif %@
    }
    pub(crate) fn bind<T>(
        self,
        query: QueryAs<'_, DbType, T, DbArguments>,
    ) -> QueryAs<'_, DbType, T, DbArguments> {
@%- if def.relations_one().len() + def.relations_many().len() > 0 %@
        match self {
@{- def.relations_one()|fmt_rel_join("
            ColRel::{alias_pascal}(c) => {
                if let Some(cond) = c {
                    cond.bind(query)
                } else {
                    query
                }
            }", "") }@
@{- def.relations_many()|fmt_rel_join("
            ColRel::{alias_pascal}(c) => {
                if let Some(cond) = c {
                    cond.bind(query)
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

#[derive(Clone, Debug)]
pub enum Cond {
    WithTrashed,
    OnlyTrashed,
    IsNull(Col),
    IsNotNull(Col),
    Eq(ColOne),
    EqKey(ColKey),
    NotEq(ColOne),
    NullSafeEq(ColOne),
    Gt(ColOne),
    Gte(ColOne),
    Lt(ColOne),
    Lte(ColOne),
    Like(ColOne),
    AllBits(ColMany),
    AnyBits(ColOne),
    In(ColMany),
    NotIn(ColMany),
    MemberOf(ColJson),
    Contains(ColJson),
    Overlaps(ColJson),
    Not(Box<Cond>),
    And(Vec<Cond>),
    Or(Vec<Cond>),
    Exists(ColRel),
    NotExists(ColRel),
}
impl Cond {
    pub fn new_and() -> Cond {
        Cond::And(vec![])
    }
    pub fn new_or() -> Cond {
        Cond::Or(vec![])
    }
    pub fn and(mut self, cond: Cond) -> Cond {
        match self {
            Cond::And(ref mut v) => {
                v.push(cond);
                self
            },
            _ => Cond::And(vec![self]),
        }
    }
    pub fn or(mut self, cond: Cond) -> Cond {
        match self {
            Cond::Or(ref mut v) => {
                v.push(cond);
                self
            },
            _ => Cond::Or(vec![self]),
        }
    }
    pub fn add(&mut self, cond: Cond) {
        match self {
            Cond::And(v) => v.push(cond),
            Cond::Or(v) => v.push(cond),
            _ => {
                panic!("'and' is not supported for the Cond: {:?}", cond);
            }
        };
    }
    crate::misc::condition!(Data);
}

@% let cond_macro_name = "cond_{}_{}"|format(group_name, name) -%@
@% let model_path = "db_{}::{}::{}::_{}"|format(db, group_name, name|to_var_name, name) -%@
@% let macro_path = "db_{}"|format(db) -%@
#[macro_export]
macro_rules! @{ cond_macro_name }@_null {
@%- for (col_name, column_def) in def.nullable() %@
    (@{ col_name }@) => (@{ model_path }@::Col::@{ col_name|to_var_name }@);
@%- endfor %@
    () => ();@# nullableが空の場合の対策 #@
}
pub use @{ cond_macro_name }@_null;

#[macro_export]
macro_rules! @{ cond_macro_name }@_one {
@%- for (col_name, column_def) in def.all_columns_without_json() %@
    (@{ col_name }@ $e:expr) => (@{ model_path }@::ColOne::@{ col_name|to_var_name }@($e.into()));
@%- endfor %@
}
pub use @{ cond_macro_name }@_one;

#[macro_export]
macro_rules! @{ cond_macro_name }@_many {
@%- for (col_name, column_def) in def.all_columns_without_json() %@
    (@{ col_name }@ [$($e:expr),*]) => (@{ model_path }@::ColMany::@{ col_name|to_var_name }@(vec![ $( $e.into() ),* ]));
    (@{ col_name }@ $e:expr) => (@{ model_path }@::ColMany::@{ col_name|to_var_name }@($e.into_iter().map(|v| v.into()).collect()));
@%- endfor %@
}
pub use @{ cond_macro_name }@_many;

#[macro_export]
macro_rules! @{ cond_macro_name }@_json {
@%- for (col_name, column_def) in def.all_columns_only_json() %@
    (@{ col_name }@ $e:expr) => (@{ model_path }@::ColJson::@{ col_name|to_var_name }@($e.into()));
@%- endfor %@
    () => ();
}
pub use @{ cond_macro_name }@_json;

#[macro_export]
macro_rules! @{ cond_macro_name }@_rel {
@%- for (model_def, col_name, rel_def) in def.relations_one() %@
    (@{ col_name }@) => (@{ model_path }@::ColRel::@{ col_name|to_var_name }@(None));
    (@{ col_name }@ $t:tt) => (@{ model_path }@::ColRel::@{ col_name|to_var_name }@(Some(std::boxed::Box::new(@{ macro_path }@::cond_@{ RelDef::get_group_name(rel_def, def) }@_@{ RelDef::get_mod_name(rel_def, col_name) }@!($t)))));
@%- endfor %@
@%- for (model_def, col_name, rel_def) in def.relations_many() %@
    (@{ col_name }@) => (@{ model_path }@::ColRel::@{ col_name|to_var_name }@(None));
    (@{ col_name }@ $t:tt) => (@{ model_path }@::ColRel::@{ col_name|to_var_name }@(Some(std::boxed::Box::new(@{ macro_path }@::cond_@{ RelDef::get_group_name(rel_def, def) }@_@{ RelDef::get_mod_name(rel_def, col_name) }@!($t)))));
@%- endfor %@
    () => ();
}
pub use @{ cond_macro_name }@_rel;

#[macro_export]
macro_rules! @{ cond_macro_name }@ {
    (($($t:tt)*)) => (@{ macro_path }@::@{ cond_macro_name }@!($($t)*));
    (NOT $t:tt) => (@{ model_path }@::Cond::Not(std::boxed::Box::new(@{ macro_path }@::@{ cond_macro_name }@!($t))));
    (WITH_TRASHED) => (@{ model_path }@::Cond::WithTrashed);
    (ONLY_TRASHED) => (@{ model_path }@::Cond::OnlyTrashed);
    ($i:ident IS NULL) => (@{ model_path }@::Cond::IsNull(@{ macro_path }@::@{ cond_macro_name }@_null!($i)));
    ($i:ident IS NOT NULL) => (@{ model_path }@::Cond::IsNotNull(@{ macro_path }@::@{ cond_macro_name }@_null!($i)));
    ($i:ident = $e:expr) => (@{ model_path }@::Cond::Eq(@{ macro_path }@::@{ cond_macro_name }@_one!($i $e)));
    ($i:ident != $e:expr) => (@{ model_path }@::Cond::NotEq(@{ macro_path }@::@{ cond_macro_name }@_one!($i $e)));
    ($i:ident <=> $e:expr) => (@{ model_path }@::Cond::NullSafeEq(@{ macro_path }@::@{ cond_macro_name }@_one!($i $e)));
    ($i:ident > $e:expr) => (@{ model_path }@::Cond::Gt(@{ macro_path }@::@{ cond_macro_name }@_one!($i $e)));
    ($i:ident >= $e:expr) => (@{ model_path }@::Cond::Gte(@{ macro_path }@::@{ cond_macro_name }@_one!($i $e)));
    ($i:ident < $e:expr) => (@{ model_path }@::Cond::Lt(@{ macro_path }@::@{ cond_macro_name }@_one!($i $e)));
    ($i:ident <= $e:expr) => (@{ model_path }@::Cond::Lte(@{ macro_path }@::@{ cond_macro_name }@_one!($i $e)));
    ($i:ident LIKE $e:expr) => (@{ model_path }@::Cond::Like(@{ macro_path }@::@{ cond_macro_name }@_one!($i $e)));
    ($i:ident ALL_BITS $e:expr) => (@{ model_path }@::Cond::AllBits(@{ macro_path }@::@{ cond_macro_name }@_many!($i [$e, $e])));
    ($i:ident ANY_BITS $e:expr) => (@{ model_path }@::Cond::AnyBits(@{ macro_path }@::@{ cond_macro_name }@_one!($i $e)));
    ($i:ident BETWEEN ($e1:expr, $e2:expr)) => (@{ macro_path }@::@{ cond_macro_name }@!(($i >= $e1) AND ($i <= $e2)));
    ($i:ident SEMI_OPEN ($e1:expr, $e2:expr)) => (@{ macro_path }@::@{ cond_macro_name }@!(($i >= $e1) AND ($i < $e2)));
    ($i:ident IN ( $($e:expr),* )) => (@{ model_path }@::Cond::In(@{ macro_path }@::@{ cond_macro_name }@_many!($i [ $( $e ),* ])));
    ($i:ident IN $e:expr) => (@{ model_path }@::Cond::In(@{ macro_path }@::@{ cond_macro_name }@_many!($i $e)));
    ($i:ident NOT IN ( $($e:expr),* )) => (@{ model_path }@::Cond::NotIn(@{ macro_path }@::@{ cond_macro_name }@_many!($i [ $( $e ),* ])));
    ($i:ident NOT IN $e:expr) => (@{ model_path }@::Cond::NotIn(@{ macro_path }@::@{ cond_macro_name }@_many!($i $e)));
    ($i:ident HAS $e:expr) => (@{ model_path }@::Cond::MemberOf(@{ macro_path }@::@{ cond_macro_name }@_json!($i $e)));
    ($i:ident CONTAINS [ $($e:expr),* ]) => (@{ model_path }@::Cond::Contains(@{ macro_path }@::@{ cond_macro_name }@_json!($i vec![ $( $e ),* ])));
    ($i:ident CONTAINS $e:expr) => (@{ model_path }@::Cond::Contains(@{ macro_path }@::@{ cond_macro_name }@_json!($i $e)));
    ($i:ident OVERLAPS [ $($e:expr),* ]) => (@{ model_path }@::Cond::Overlaps(@{ macro_path }@::@{ cond_macro_name }@_json!($i vec![ $( $e ),* ])));
    ($i:ident OVERLAPS $e:expr) => (@{ model_path }@::Cond::Overlaps(@{ macro_path }@::@{ cond_macro_name }@_json!($i $e)));
    ($i:ident EXISTS) => (@{ model_path }@::Cond::Exists(@{ macro_path }@::@{ cond_macro_name }@_rel!($i)));
    ($i:ident EXISTS $t:tt) => (@{ model_path }@::Cond::Exists(@{ macro_path }@::@{ cond_macro_name }@_rel!($i $t)));
    ($i:ident NOT EXISTS) => (@{ model_path }@::Cond::NotExists(@{ macro_path }@::@{ cond_macro_name }@_rel!($i)));
    ($i:ident NOT EXISTS $t:tt) => (@{ model_path }@::Cond::NotExists(@{ macro_path }@::@{ cond_macro_name }@_rel!($i $t)));
    ($t1:tt AND $($t2:tt)AND+) => (@{ model_path }@::Cond::And(vec![ @{ macro_path }@::@{ cond_macro_name }@!($t1), $( @{ macro_path }@::@{ cond_macro_name }@!($t2) ),* ]));
    ($t1:tt OR $($t2:tt)OR+) => (@{ model_path }@::Cond::Or(vec![ @{ macro_path }@::@{ cond_macro_name }@!($t1), $( @{ macro_path }@::@{ cond_macro_name }@!($t2) ),* ]));
}
pub use @{ cond_macro_name }@;

#[derive(Clone, Debug)]
pub enum OrderBy {
    Asc(Col),
    Desc(Col),
    IsNullAsc(Col),
    IsNullDesc(Col),
}
impl OrderBy {
    crate::misc::order_by!();
}

@% let order_by_macro_name = "order_by_{}_{}"|format(group_name, name) -%@
#[macro_export]
macro_rules! @{ order_by_macro_name }@_col {
@%- for (col_name, column_def) in def.all_columns() %@
    (@{ col_name }@) => (@{ model_path }@::Col::@{ col_name|to_var_name }@);
@%- endfor %@
}
pub use @{ order_by_macro_name }@_col;

#[macro_export]
macro_rules! @{ order_by_macro_name }@_one {
    ($i:ident) => (@{ model_path }@::OrderBy::Asc(@{ macro_path }@::@{ order_by_macro_name }@_col!($i)));
    ($i:ident ASC) => (@{ model_path }@::OrderBy::Asc(@{ macro_path }@::@{ order_by_macro_name }@_col!($i)));
    ($i:ident DESC) => (@{ model_path }@::OrderBy::Desc(@{ macro_path }@::@{ order_by_macro_name }@_col!($i)));
    ($i:ident IS NULL ASC) => (@{ model_path }@::OrderBy::IsNullAsc(@{ macro_path }@::@{ order_by_macro_name }@_col!($i)));
    ($i:ident IS NULL DESC) => (@{ model_path }@::OrderBy::IsNullDesc(@{ macro_path }@::@{ order_by_macro_name }@_col!($i)));
}
pub use @{ order_by_macro_name }@_one;

#[macro_export]
macro_rules! @{ order_by_macro_name }@ {
    ($($($i:ident)+),+) => (vec![$( @{ macro_path }@::@{ order_by_macro_name }@_one!($($i)+)),+]);
}
pub use @{ order_by_macro_name }@;

#[derive(sqlx::FromRow)]
struct Count {
    pub c: i64,
}

#[derive(Debug, Clone, Default)]
pub struct QueryBuilder {
    condition: Option<Cond>,
    order_by: Option<Vec<OrderBy>>,
    limit: Option<usize>,
    offset: Option<usize>,
    trash_mode: TrashMode,
    raw_query: Option<String>,
    bind: Vec<BindValue>,
@{ def.relations()|fmt_rel_join("    fetch_{raw_alias}: bool,\n", "") -}@
}

impl QueryBuilder {
    pub fn cond(mut self, condition: Cond) -> Self {
        self.condition = Some(condition);
        self
    }
    pub fn order_by(mut self, order_by: Vec<OrderBy>) -> Self {
        self.order_by = Some(order_by);
        self
    }
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }
    pub fn with_trashed(mut self) -> Self {
        self.trash_mode = TrashMode::With;
        self
    }
    pub fn only_trashed(mut self) -> Self {
        self.trash_mode = TrashMode::Only;
        self
    }
    pub fn raw_query<T: Into<String>>(mut self, query: T) -> Self {
        self.raw_query = Some(query.into());
        self
    }
    pub fn bind<T: Into<BindValue>>(mut self, value: T) -> Self {
        self.bind.push(value.into());
        self
    }
@{- def.relations()|fmt_rel_join("
    pub fn fetch_{raw_alias}(mut self) -> Self {
        self.fetch_{raw_alias} = true;
        self
    }", "") }@
    async fn _select<T>(self, conn: &mut DbConn, with_count: bool) -> Result<Vec<T>>
    where
        T: for<'r> sqlx::FromRow<'r, <DbType as sqlx::Database>::Row> + SqlColumns + Send + Unpin,
    {
        let sql = self._sql(with_count, T::_sql_cols("`"));
        let mut query = sqlx::query_as::<_, T>(&sql);
        let _span = info_span!("query", sql = &query.sql());
        query = self._bind(query);
        let result = crate::misc::fetch!(conn, query, fetch_all);
        Ok(result)
    }

    fn _sql(&self, with_count: bool, sql_cols: &str) -> String {
        let mut sql = format!(
            r#"SELECT {} {} FROM @{ table_name|db_esc }@ as _t1 {} {}"#,
            if with_count {
                "SQL_CALC_FOUND_ROWS"
            } else {
                ""
            },
            sql_cols,
            if let Some(ref query) = self.raw_query {
                query.clone()
            } else {
                Cond::write_where(
                    &self.condition,
                    self.trash_mode,
                    TRASHED_SQL,
                    NOT_TRASHED_SQL,
                    ONLY_TRASHED_SQL
                )
            },
            OrderBy::write_order_by(&self.order_by)
        );
        if let Some(limit) = self.limit {
            let _ = write!(sql, " limit {}", limit);
        }
        if let Some(offset) = self.offset {
            let _ = write!(sql, " offset {}", offset);
        }
        sql
    }

    fn _bind<T>(self, mut query: QueryAs<DbType, T, DbArguments>) -> QueryAs<DbType, T, DbArguments> {
        if self.raw_query.is_some() {
            for value in self.bind.into_iter() {
                debug!("bind:{:?}", &value);
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
                };
            }
        } else if let Some(c) = self.condition {
            query = c.bind(query);
        }
        query
    }

    async fn _select_stream<'a, T: 'a>(self, conn: &mut DbConn) -> Result<mpsc::Receiver<T>>
    where
        T: for<'r> sqlx::FromRow<'r, <DbType as sqlx::Database>::Row>
            + SqlColumns
            + Send
            + Unpin
            + 'static,
    {
        let mut sql = format!(
            r#"SELECT {} FROM @{ table_name|db_esc }@ as _t1 {} {}"#,
            T::_sql_cols("`"),
            if let Some(ref query) = self.raw_query {
                query.clone()
            } else {
                Cond::write_where(
                    &self.condition,
                    self.trash_mode,
                    TRASHED_SQL,
                    NOT_TRASHED_SQL,
                    ONLY_TRASHED_SQL
                )
            },
            OrderBy::write_order_by(&self.order_by)
        );
        if let Some(limit) = self.limit {
            let _ = write!(sql, " limit {}", limit);
        }
        if let Some(offset) = self.offset {
            let _ = write!(sql, " offset {}", offset);
        }
        let (tx, rx) = mpsc::channel(1000);
        let mut executor = conn.acquire_replica().await?;
        tokio::spawn(async move {
            let mut query = sqlx::query_as::<_, T>(&sql);
            let _span = info_span!("query", sql = &query.sql());
            if self.raw_query.is_some() {
                for value in self.bind.into_iter() {
                    debug!("bind:{:?}", &value);
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
                    };
                }
            } else if let Some(c) = self.condition {
                query = c.bind(query);
            }
            let mut result = query.fetch(&mut executor);
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

    async fn _select_from_cache(self, conn: &mut DbConn, with_count: bool) -> Result<Vec<_@{ pascal_name }@Cache>> {
        let mut sql = format!(
            r#"SELECT {} @{ def.primaries()|fmt_join("{col_esc}", ", ") }@ FROM @{ table_name|db_esc }@ as _t1 {} {}"#,
            if with_count {
                "SQL_CALC_FOUND_ROWS"
            } else {
                ""
            },
            Cond::write_where(&self.condition, self.trash_mode, TRASHED_SQL, NOT_TRASHED_SQL, ONLY_TRASHED_SQL),
            OrderBy::write_order_by(&self.order_by)
        );
        if let Some(limit) = self.limit {
            let _ = write!(sql, " limit {}", limit);
        }
        if let Some(offset) = self.offset {
            let _ = write!(sql, " offset {}", offset);
        }
        let mut query = sqlx::query_as::<_, Primary>(&sql);
        let _span = info_span!("query", sql = &query.sql());
        if let Some(c) = self.condition {
            query = c.bind(query);
        }
        let result = crate::misc::fetch!(conn, query, fetch_all);
        let ids: Vec<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@> = result.iter().map(|id| id.into()).collect();
        conn.release_cache_tx();
        let mut map = _@{ pascal_name }@::find_many_from_cache(conn, ids).await?;
        #[allow(unused_mut)]
        let mut list: Vec<_@{ pascal_name }@Cache> = result
            .iter()
            .flat_map(|id| map.remove(&id.into()))
            .collect();
        Ok(list)
    }
    @%- endif %@

    pub async fn select_for_update(self, conn: &mut DbConn) -> Result<Vec<_@{ pascal_name }@ForUpdate>> {
        let mut sql = format!(
            r#"SELECT {} FROM @{ table_name|db_esc }@ as _t1 {} {} FOR UPDATE"#,
            Data::_sql_cols("`"),
            Cond::write_where(&self.condition, self.trash_mode, TRASHED_SQL, NOT_TRASHED_SQL, ONLY_TRASHED_SQL),
            OrderBy::write_order_by(&self.order_by)
        );
        if let Some(limit) = self.limit {
            let _ = write!(sql, " limit {}", limit);
        }
        if let Some(offset) = self.offset {
            let _ = write!(sql, " offset {}", offset);
        }
        let mut query = sqlx::query_as::<_, Data>(&sql);
        let _span = info_span!("query", sql = &query.sql());
        if let Some(c) = self.condition {
            query = c.bind(query);
        }
        let result = query.fetch_all(conn.get_tx().await?).await?;
        let list: Vec<ForUpdate> = result
            .into_iter()
            .map(|data| ForUpdate {
                _data: data,
                _update: Default::default(),
                _is_new: false,
                _do_delete: false,
                _upsert: false,
                _is_loaded: true,
                _op: Default::default(),
@{- def.relations_one_owner()|fmt_rel_join("\n                {alias}: None,", "") }@
@{- def.relations_many()|fmt_rel_join("\n                {alias}: None,", "") }@
            })
            .collect();
        Ok(list)
    }

    pub async fn select(self, conn: &mut DbConn) -> Result<Vec<_@{ pascal_name }@>> {
@{- def.relations()|fmt_rel_join("\n        let fetch_{raw_alias} = self.fetch_{raw_alias};", "") }@
        let result: Vec<Data> = self._select(conn, false).await?;
        #[allow(unused_mut)]
        let mut list: Vec<_@{ pascal_name }@> = result.into_iter().map(_@{ pascal_name }@::from).collect();
@{- def.relations()|fmt_rel_join("
        if fetch_{raw_alias} {
            list.fetch_{raw_alias}(conn).await?;
        }", "") }@
        Ok(list)
    }

    pub async fn select_one(self, conn: &mut DbConn) -> Result<Option<_@{ pascal_name }@>> {
        let mut list = Self::select(self, conn).await?;
        Ok(list.pop())
    }

    pub async fn select_for<T>(self, conn: &mut DbConn) -> Result<Vec<T>>
    where
        T: for<'r> sqlx::FromRow<'r, <DbType as sqlx::Database>::Row> + SqlColumns + Send + Unpin,
    {
        self._select(conn, false).await
    }

    pub async fn select_one_for<T>(self, conn: &mut DbConn) -> Result<Option<T>>
    where
        T: for<'r> sqlx::FromRow<'r, <DbType as sqlx::Database>::Row> + SqlColumns + Send + Unpin,
    {
        let mut list = Self::select_for(self, conn).await?;
        Ok(list.pop())
    }

    pub(crate) async fn select_for_cache(self, conn: &mut DbConn) -> Result<Vec<_@{ pascal_name }@Cache>> {
        let result: Vec<CacheData> = self._select(conn, false).await?;
        let list = result.into_iter().map(|v| Arc::new(CacheWrapper::_from_inner(v, conn.shard_id(), conn.begin_time())).into()).collect();
        Ok(list)
    }
    @%- if def.use_cache() %@

    pub async fn select_from_cache(self, conn: &mut DbConn) -> Result<Vec<_@{ pascal_name }@Cache>> {
        self._select_from_cache(conn, false).await
    }
    @%- endif %@

    pub async fn select_with_count(self, conn: &mut DbConn) -> Result<(Vec<_@{ pascal_name }@>, i64)> {
        @{- def.relations()|fmt_rel_join("\n        let fetch_{raw_alias} = self.fetch_{raw_alias};", "") }@
        let result: Vec<Data> = self._select(conn, true).await?;
        let query = sqlx::query_scalar("SELECT FOUND_ROWS()");
        let count = crate::misc::fetch!(conn, query, fetch_one);
        #[allow(unused_mut)]
        let mut list: Vec<_@{ pascal_name }@> = result.into_iter().map(_@{ pascal_name }@::from).collect();
@{- def.relations()|fmt_rel_join("
        if fetch_{raw_alias} {
            list.fetch_{raw_alias}(conn).await?;
        }", "") }@
        Ok((list, count))
    }
    @%- if def.use_cache() %@

    pub async fn select_from_cache_with_count(self, conn: &mut DbConn) -> Result<(Vec<_@{ pascal_name }@Cache>, i64)> {
        let list = self._select_from_cache(conn, true).await?;
        let query = sqlx::query_scalar("SELECT FOUND_ROWS()");
        let count = crate::misc::fetch!(conn, query, fetch_one);
        Ok((list, count))
    }
    @%- endif %@

    pub async fn select_with_count_for<T>(self, conn: &mut DbConn) -> Result<(Vec<T>, i64)>
    where
        T: for<'r> sqlx::FromRow<'r, <DbType as sqlx::Database>::Row> + SqlColumns + Send + Unpin,
    {
        let list: Vec<T> = self._select(conn, true).await?;
        let query = sqlx::query_scalar("SELECT FOUND_ROWS()");
        let count = crate::misc::fetch!(conn, query, fetch_one);
        Ok((list, count))
    }

    pub async fn count(self, conn: &mut DbConn) -> Result<i64> {
        let sql = format!(
            r#"SELECT count(*) as c FROM @{ table_name|db_esc }@ as _t1 {}"#,
            Cond::write_where(
                &self.condition,
                self.trash_mode,
                TRASHED_SQL,
                NOT_TRASHED_SQL,
                ONLY_TRASHED_SQL
            ),
        );
        let mut query = sqlx::query_as::<_, Count>(&sql);
        let _span = info_span!("query", sql = &query.sql());
        if let Some(c) = self.condition {
            query = c.bind(query);
        }
        let result = crate::misc::fetch!(conn, query, fetch_one);
        Ok(result.c)
    }

    pub async fn select_stream(self, conn: &mut DbConn) -> Result<impl Stream<Item = _@{ pascal_name }@>> {
        let mut rx: mpsc::Receiver<Data> = self._select_stream(conn).await?;
        Ok(async_stream::stream! {
            while let Some(v) = rx.recv().await {
                yield  _@{ pascal_name }@::from(v);
            }
        })
    }

    pub async fn select_stream_for<T>(self, conn: &mut DbConn) -> Result<impl Stream<Item = T>>
    where
        T: for<'r> sqlx::FromRow<'r, <DbType as sqlx::Database>::Row> + SqlColumns + Send + Unpin + 'static,
    {
        let mut rx: mpsc::Receiver<T> = self._select_stream(conn).await?;
        Ok(async_stream::stream! {
            while let Some(v) = rx.recv().await {
                yield  v;
            }
        })
    }

    #[allow(unused_mut)]
    pub async fn update(self, conn: &mut DbConn, mut obj: _@{ pascal_name }@ForUpdate) -> Result<()> {
        @%- if def.updated_at_conf().is_some() %@
        if obj._op.updated_at == Op::None {
            obj.updated_at().set(@{(def.updated_at_conf().unwrap() == Timestampable::RealTime)|if_then_else("SystemTime::now()","conn.time()")}@.into());
        }
        @%- endif %@
        let mut vec: Vec<String> = Vec::new();
        @{- def.non_primaries()|fmt_join_cache_or_not("
        assignment_sql_no_cache_update!(obj, vec, {var}, \"{col_esc}\", {may_null}, \"{placeholder}\");", "
        assignment_sql_no_cache_update!(obj, vec, {var}, \"{col_esc}\", {may_null}, \"{placeholder}\");", "") }@
        let mut sql = format!(
            r#"UPDATE @{ table_name|db_esc }@ as _t1 SET {} {} {}"#,
            &vec.join(","),
            Cond::write_where(
                &self.condition,
                self.trash_mode,
                TRASHED_SQL,
                NOT_TRASHED_SQL,
                ONLY_TRASHED_SQL
            ),
            OrderBy::write_order_by(&self.order_by)
        );
        if let Some(limit) = self.limit {
            let _ = write!(sql, " limit {}", limit);
        }
        let mut query = sqlx::query_as::<_, Count>(&sql);
        let _span = info_span!("query", sql = &query.sql());
        @{- def.non_primaries()|fmt_join("
        bind_sql!(obj, query, {var}, {may_null});","") }@
        if let Some(c) = self.condition {
            query = c.bind(query);
        }
        debug!("{}", &obj);
        query.fetch_optional(conn.get_tx().await?).await?;
        Ok(())
    }

    #[allow(unused_mut)]
    pub async fn delete(self, conn: &mut DbConn) -> Result<()> {
        @{- def.soft_delete_tpl2("
        self.force_delete(conn).await","
        let mut obj = _{pascal_name}::for_update(conn);
        obj.deleted_at().set(Some({val}.into()));
        self.update(conn, obj).await","
        let mut obj = _{pascal_name}::for_update(conn);
        obj.deleted().set(true);
        self.update(conn, obj).await","
        let mut obj = _{pascal_name}::for_update(conn);
        let deleted = cmp::max(1, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as u32);
        obj.deleted().set(deleted);
        self.update(conn, obj).await")}@
    }

    #[allow(unused_mut)]
    pub async fn force_delete(self, conn: &mut DbConn) -> Result<()> {
        let mut sql = format!(
            r#"DELETE FROM @{ table_name|db_esc }@ as _t1 {} {}"#,
            Cond::write_where(
                &self.condition,
                self.trash_mode,
                TRASHED_SQL,
                NOT_TRASHED_SQL,
                ONLY_TRASHED_SQL
            ),
            OrderBy::write_order_by(&self.order_by)
        );
        if let Some(limit) = self.limit {
            let _ = write!(sql, " limit {}", limit);
        }
        let mut query = sqlx::query_as::<_, Count>(&sql);
        let _span = info_span!("query", sql = &query.sql());
        if let Some(c) = self.condition {
            query = c.bind(query);
        }
        query.fetch_optional(conn.get_tx().await?).await?;
        Ok(())
    }
}

#[async_trait(?Send)]
pub trait UnionBuilder {
    async fn select(self, conn: &mut DbConn) -> Result<Vec<_@{ pascal_name }@>>;
    async fn select_for_cache(self, conn: &mut DbConn) -> Result<Vec<_@{ pascal_name }@Cache>>;
}

async fn _union<T>(vec: Vec<QueryBuilder>, conn: &mut DbConn) -> Result<Vec<T>>
where
    T: for<'r> sqlx::FromRow<'r, <DbType as sqlx::Database>::Row> + SqlColumns + Send + Unpin,
{
    let sql = vec.iter().map(|v| format!("({})", v._sql(false, T::_sql_cols("`")))).collect::<Vec<_>>().join(" UNION ");
    let mut query = sqlx::query_as::<_, T>(&sql);
    let _span = info_span!("query", sql = &query.sql());
    for builder in vec {
        query = builder._bind(query);
    }
    let result = crate::misc::fetch!(conn, query, fetch_all);
    Ok(result)
}

#[async_trait(?Send)]
impl UnionBuilder for Vec<QueryBuilder> {
    async fn select(self, conn: &mut DbConn) -> Result<Vec<_@{ pascal_name }@>> {
        if self.is_empty() {
            return Ok(Vec::new());
        }
        let first = self.first().unwrap();
@{- def.relations()|fmt_rel_join("\n        let fetch_{raw_alias} = first.fetch_{raw_alias};", "") }@
        let result: Vec<Data> = _union(self, conn).await?;
        #[allow(unused_mut)]
        let mut list: Vec<_@{ pascal_name }@> = result.into_iter().map(_@{ pascal_name }@::from).collect();
@{- def.relations()|fmt_rel_join("
        if fetch_{raw_alias} {
            list.fetch_{raw_alias}(conn).await?;
        }", "") }@
        Ok(list)
    }

    async fn select_for_cache(self, conn: &mut DbConn) -> Result<Vec<_@{ pascal_name }@Cache>> {
        let result: Vec<CacheData> = _union(self, conn).await?;
        let list = result.into_iter().map(|v| Arc::new(CacheWrapper::_from_inner(v, conn.shard_id(), conn.begin_time())).into()).collect();
        Ok(list)
    }
}

@% for (name, column_def) in def.id() -%@
impl std::ops::Deref for @{ id_name }@ {
    type Target = @{ column_def.get_inner_type(false) }@;
    fn deref(&self) -> &@{ column_def.get_inner_type(false) }@ {
        &self.0@{ column_def.clone_str() }@
    }
}

impl @{ id_name }@ {
    pub fn get(&self) -> @{ column_def.get_inner_type(false) }@ {
        self.0@{ column_def.clone_str() }@
    }
@% if def.primaries().len() == 1 %@
    pub async fn fetch(&self, conn: &mut DbConn) -> Result<Option<_@{ pascal_name }@>> {
        _@{ pascal_name }@::find_optional(conn, self).await
    }

    pub async fn fetch_with_trashed(&self, conn: &mut DbConn) -> Result<Option<_@{ pascal_name }@>> {
        _@{ pascal_name }@::find_optional_with_trashed(conn, self).await
    }
    @%- if def.use_cache() %@

    pub async fn fetch_from_cache(&self, conn: &DbConn) -> Result<Option<_@{ pascal_name }@Cache>> {
        _@{ pascal_name }@::find_optional_from_cache(conn, self).await
    }

    pub async fn fetch_from_cache_with_trashed(&self, conn: &mut DbConn) -> Result<Option<_@{ pascal_name }@Cache>> {
        _@{ pascal_name }@::find_optional_from_cache_with_trashed(conn, self).await
    }
    @%- endif %@

    pub async fn fetch_for_update(&self, conn: &mut DbConn) -> Result<_@{ pascal_name }@ForUpdate> {
        _@{ pascal_name }@::find_for_update(conn, self).await
    }

    pub async fn fetch_for_update_with_trashed(&self, conn: &mut DbConn) -> Result<_@{ pascal_name }@ForUpdate> {
        _@{ pascal_name }@::find_for_update_with_trashed(conn, self).await
    }

    pub fn for_update(&self, conn: &DbConn) -> _@{ pascal_name }@ForUpdate {
        _@{ pascal_name }@ForUpdate {
            _data: Data {
                @{ name }@: self.get(),
                ..Default::default()
            },
            _update: Default::default(),
            _is_new: false,
            _do_delete: false,
            _upsert: false,
            _is_loaded: false,
            _op: Default::default(),
@{- def.relations_one_owner()|fmt_rel_join("\n            {alias}: None,", "") }@
@{- def.relations_many()|fmt_rel_join("\n            {alias}: None,", "") }@
        }
    }
@% endif -%@
}

#[async_trait(?Send)]
pub trait @{ id_name }@Tr {
    async fn fetch_from_cache(&self, conn: &DbConn) -> Result<Option<_@{ pascal_name }@Cache>>;
    async fn fetch_from_cache_with_trashed(&self, conn: &DbConn) -> Result<Option<_@{ pascal_name }@Cache>>;
}
@%- if def.primaries().len() == 1 && def.use_cache() %@

#[async_trait(?Send)]
impl @{ id_name }@Tr for Option<@{ id_name }@> {
    async fn fetch_from_cache(&self, conn: &DbConn) -> Result<Option<_@{ pascal_name }@Cache>> {
        if let Some(id) = self {
            _@{ pascal_name }@::find_optional_from_cache(conn, id).await
        } else {
            Ok(None)
        }
    }
    async fn fetch_from_cache_with_trashed(&self, conn: &DbConn) -> Result<Option<_@{ pascal_name }@Cache>> {
        if let Some(id) = self {
            _@{ pascal_name }@::find_from_cache_with_trashed(conn, id)
                .await
                .map(Some)
        } else {
            Ok(None)
        }
    }
}
@%- endif %@

impl From<@{ column_def.get_inner_type(false) }@> for @{ id_name }@ {
    fn from(id: @{ column_def.get_inner_type(false) }@) -> Self {
        Self(id)
    }
}
impl From<@{ id_name }@> for @{ column_def.get_inner_type(false) }@ {
    fn from(id: @{ id_name }@) -> Self {
        id.0@{ column_def.clone_str() }@
    }
}
impl From<&@{ id_name }@> for @{ id_name }@ {
    fn from(id: &@{ id_name }@) -> Self {
        Self(id.get())
    }
}
@%- endfor  %@
@% if def.primaries().len() == 1 %@
impl From<&Primary> for @{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@ {
    fn from(id: &Primary) -> Self {
        Self(@{ def.primaries()|fmt_join_with_paren("id.{index}", ", ") }@)
    }
}
impl From<&Arc<Primary>> for @{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@ {
    fn from(id: &Arc<Primary>) -> Self {
        Self(@{ def.primaries()|fmt_join_with_paren("id.{index}", ", ") }@)
    }
}
impl From<@{ def.primaries()|fmt_join_with_paren("{outer_ref}", ", ") }@> for Primary {
    fn from(obj: @{ def.primaries()|fmt_join_with_paren("{outer_ref}", ", ") }@) -> Self {
        @% if def.primaries().len() == 1 -%@
        Self(obj.get())
        @%- else -%@
        Self(@{ def.primaries()|fmt_join("obj.{index}.get()", ", ") }@)
        @%- endif %@
    }
}
@% endif -%@
impl From<@{ def.primaries()|fmt_join_with_paren("{inner}", ", ") }@> for Primary {
    fn from(id: @{ def.primaries()|fmt_join_with_paren("{inner}", ", ") }@) -> Self {
        @% if def.primaries().len() == 1 %@Self(id)@% else %@Self(@{ def.primaries()|fmt_join("id.{index}", ", ") }@)@% endif %@
    }
}
@% if def.primaries().len() > 1 -%@
impl From<&@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@> for Primary {
    fn from(id: &@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@) -> Self {
        @% if def.primaries().len() == 1 %@Self(id)@% else %@Self(@{ def.primaries()|fmt_join("id.{index}.into()", ", ") }@)@% endif %@
    }
}
impl From<&Primary> for @{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@ {
    fn from(id: &Primary) -> Self {
        @{ def.primaries()|fmt_join_with_paren("id.{index}.into()", ", ") }@
    }
}
@% endif -%@
impl From<&_@{ pascal_name }@> for Primary {
    fn from(obj: &_@{ pascal_name }@) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._inner.{var}{clone}", ", ") }@)
    }
}

impl From<&Data> for Primary {
    fn from(obj: &Data) -> Self {
        Self(@{ def.primaries()|fmt_join("obj.{var}{clone}", ", ") }@)
    }
}

impl From<&CacheData> for Primary {
    fn from(obj: &CacheData) -> Self {
        Self(@{ def.primaries()|fmt_join("obj.{var}{clone}", ", ") }@)
    }
}

impl From<&_@{ pascal_name }@Cache> for Primary {
    fn from(obj: &_@{ pascal_name }@Cache) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._wrapper._inner.{var}{clone}", ", ") }@)
    }
}

impl From<&Arc<CacheWrapper>> for Primary {
    fn from(obj: &Arc<CacheWrapper>) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._inner.{var}{clone}", ", ") }@)
    }
}

impl From<&_@{ pascal_name }@ForUpdate> for Primary {
    fn from(obj: &_@{ pascal_name }@ForUpdate) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._data.{var}{clone}", ", ") }@)
    }
}

impl From<&mut _@{ pascal_name }@ForUpdate> for Primary {
    fn from(obj: &mut _@{ pascal_name }@ForUpdate) -> Self {
        Self(@{ def.primaries()|fmt_join("obj._data.{var}{clone}", ", ") }@)
    }
}

fn id_to_string(id: &@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@) -> String {
    format!("@{ def.primaries()|fmt_join("{col}={}", ", ") }@"@{ def.primaries()|fmt_join(", id.{index}", "") }@)
}

impl fmt::Display for Primary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{ def.primaries()|fmt_join("{col}={}", ", ") }@"@{ def.primaries()|fmt_join(", self.{index}", "") }@)
    }
}

fn vec_pri_to_str(s: &[Primary]) -> String {
    let v = s.iter().fold(Vec::new(), |mut i, p| {
        i.push(format!("@{ def.primaries()|fmt_join_with_paren("{}", ", ") }@"@{ def.primaries()|fmt_join(", p.{index}", "") }@));
        i
    });
    format!("@{ def.primaries()|fmt_join_with_paren("{col}", ", ") }@={}", v.join(","))
}

impl _@{ pascal_name }@Tr for _@{ pascal_name }@ {
    @{- def.all_columns()|fmt_join("
    fn {var}(&self) -> {outer} {
        self._inner.{var}{clone}{convert_outer}
    }", "") }@
    @{- def.relations_one_except_cache()|fmt_rel_join("
    fn {alias}(&self) -> Option<&rel_{class_mod}::{class}> {
        self.{alias}.as_ref().expect(\"{alias} is not loaded\").as_ref().map(|b| &**b)
    }", "") }@
    @{- def.relations_one_only_cache()|fmt_rel_join("
    fn {alias}(&self) -> Option<&rel_{class_mod}::{class}Cache> {
        self.{alias}.as_ref().expect(\"{alias} is not loaded\").as_ref().map(|b| &**b)
    }", "") }@
    @{- def.relations_many()|fmt_rel_join("
    fn {alias}(&self) -> &Vec<rel_{class_mod}::{class}> {
        self.{alias}.as_ref().expect(\"{alias} is not loaded\")
    }", "") }@
}

impl _@{ pascal_name }@MutTr for _@{ pascal_name }@ {
    @{- def.all_columns()|fmt_join("
    fn {var}(&mut self) -> {outer} {
        self._inner.{var}{clone}{convert_outer}
    }", "") }@
    @{- def.relations_one_except_cache()|fmt_rel_join("
    fn {alias}(&mut self) -> Option<&mut rel_{class_mod}::{class}> {
        self.{alias}.as_mut().expect(\"{alias} is not loaded\").as_mut().map(|b| &mut **b)
    }", "") }@
    @{- def.relations_one_only_cache()|fmt_rel_join("
    fn {alias}(&mut self) -> Option<&mut rel_{class_mod}::{class}Cache> {
        self.{alias}.as_mut().expect(\"{alias} is not loaded\").as_mut().map(|b| &mut **b)
    }", "") }@
    @{- def.relations_many()|fmt_rel_join("
    fn {alias}(&mut self) -> &mut Vec<rel_{class_mod}::{class}> {
        self.{alias}.as_mut().expect(\"{alias} is not loaded\")
    }", "") }@
}
@%- for parent in def.parents() %@
impl crate::@{ parent.group_name }@::@{ parent.name }@::_@{ parent.name|pascal }@Tr for _@{ pascal_name }@ {
    @{- parent.primaries()|fmt_join("
    fn {var}(&self) -> &{inner} {
        &self._inner.{var}
    }", "") }@
    @{- parent.non_primaries()|fmt_join("
    fn {var}(&self) -> {outer} {
        self._inner.{var}{clone}{convert_outer}
    }", "") }@
    @{- parent.relations_one_except_cache()|fmt_rel_join("
    fn {alias}(&self) -> Option<&rel_{class_mod}::{class}> {
        self.{alias}.as_ref().expect(\"{alias} is not loaded\").as_ref().map(|b| &**b)
    }", "") }@
    @{- parent.relations_one_only_cache()|fmt_rel_join("
    fn {alias}(&self) -> Option<&rel_{class_mod}::{class}Cache> {
        self.{alias}.as_ref().expect(\"{alias} is not loaded\").as_ref().map(|b| &**b)
    }", "") }@
    @{- parent.relations_many()|fmt_rel_join("
    fn {alias}(&self) -> &Vec<rel_{class_mod}::{class}> {
        self.{alias}.as_ref().expect(\"{alias} is not loaded\").as_ref()
    }", "") }@
}
@%- endfor %@

static CACHE_WRAPPER_AVG: AtomicUsize = AtomicUsize::new(0);
static CACHE_WRAPPER_AVG_NUM: AtomicUsize = AtomicUsize::new(0);

impl CacheVal for CacheWrapper {
    fn _size(&self) -> u32 {
        let mut size = calc_mem_size(std::mem::size_of::<Self>());
        @{- def.cache_cols_not_null_sized()|fmt_join("
        size += self._inner.{var}._size();", "") }@
        @{- def.cache_cols_null_sized()|fmt_join("
        size += self._inner.{var}.as_ref().map(|v| v._size()).unwrap_or(0);", "") }@
        @{- def.relations_one_cache()|fmt_rel_join("
        size += self.{alias}.as_ref().map(|v| v._size() as usize).unwrap_or(0);", "") }@
        @{- def.relations_many_cache()|fmt_rel_join("
        size += self.{alias}.iter().fold(0, |i, v| i + v._size() as usize);", "") }@
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
        let vec = encode_all(serde_cbor::ser::to_vec_packed(self)?.as_slice(), 1)?;
        let num = CACHE_WRAPPER_AVG_NUM.load(Ordering::Relaxed);
        let ave = (CACHE_WRAPPER_AVG.load(Ordering::Relaxed) * num + vec.len()) / num.saturating_add(1);
        CACHE_WRAPPER_AVG_NUM.store(num.saturating_add(1), Ordering::Relaxed);
        CACHE_WRAPPER_AVG.store(ave, Ordering::Relaxed);
        Ok(vec)
    }
    fn _decode(v: &[u8]) -> Result<Self> {
        Ok(serde_cbor::from_slice(&decode_all(v)?)?)
    }
}

impl CacheWrapper {
    @{- def.cache_cols()|fmt_join("
    pub fn {var}(&self) -> {outer} {
        self._inner.{var}{clone}{convert_outer}
    }", "") }@
    @{- def.relations_one_cache()|fmt_rel_join("
    pub fn {alias}(&self) -> Option<&Arc<rel_{class_mod}::CacheWrapper>> {
        self.{alias}.as_ref()
    }", "") }@
    @{- def.relations_many_cache()|fmt_rel_join("
    pub fn {alias}(&self) -> &Vec<Arc<rel_{class_mod}::CacheWrapper>> {
        self.{alias}.as_ref()
    }", "") }@
}

impl _@{ pascal_name }@Cache {
    @{- def.cache_cols()|fmt_join("
{title}{comment}    pub fn {var}(&self) -> {outer} {
        self._wrapper.{var}()
    }", "") }@
    @{- def.relations_one_cache()|fmt_rel_join("
{title}{comment}    pub fn {alias}(&self) -> Option<rel_{class_mod}::{class}Cache> {
        self._wrapper.{alias}().map(|v| v.clone().into())
    }", "") }@
    @{- def.relations_many_cache()|fmt_rel_join("
{title}{comment}    pub fn {alias}(&self) -> Vec<rel_{class_mod}::{class}Cache> {
        self._wrapper.{alias}().iter().map(|v| v.clone().into()).collect()
    }", "") }@
    @{- def.relations_one_only_cache()|fmt_rel_join("
{title}{comment}    pub fn {alias}(&self) -> Option<&rel_{class_mod}::{class}Cache> {
        self.{alias}.as_ref().expect(\"{alias} is not loaded\").as_ref().map(|b| &**b)
    }", "") }@
    pub async fn _invalidate(&self, conn: &DbConn) {
        let id: Primary = self.into();
        CacheMsg(vec![CacheOp::Invalidate{id, shard_id: conn.shard_id()}.wrap()], MSec::now())
            .do_send()
            .await;
    }
}

impl ForUpdateTr for _@{ pascal_name }@ForUpdate {
    fn _is_new(&self) -> bool {
        self._is_new
    }
    fn _has_been_deleted(&self) -> bool {
        @{ def.soft_delete_tpl("false","self._data.deleted_at.is_some()","self._data.deleted != 0")}@
    }
    fn _delete(&mut self) {
        self._do_delete = true;
    }
    fn _will_be_deleted(&self) -> bool {
        self._do_delete
    }
    fn _upsert(&mut self) {
        self._upsert = true;
        @{- def.auto_increments()|fmt_join("
        panic!(\"Tables using auto increment are not supported.\");", "") }@
    }
    fn _is_updated(&self) -> bool {
        self._is_new
        || self._do_delete
        @{- def.non_primaries()|fmt_join("
        || self._op.{var} != Op::None", "") }@
        @{- def.relations_one_owner()|fmt_rel_join("
        || self.{alias}.as_ref().and_then(|v| v.as_ref().map(|v| v._is_updated())).unwrap_or_default()", "") }@
        @{- def.relations_many()|fmt_rel_join("
        || self.{alias}.as_ref().map(|v| v.iter().any(|v| v._is_updated())).unwrap_or_default()", "") }@
    }
}

impl _@{ pascal_name }@ForUpdate {
@{- def.primaries()|fmt_join("
{title}{comment}    pub fn {var}(&self) -> Accessor{accessor_with_type} {
        Accessor{accessor} {
            val: &self._data.{var},
            _phantom: Default::default(),
        }
    }", "") }@
@{- def.non_primaries()|fmt_join("
{title}{comment}    pub fn {var}(&mut self) -> Accessor{accessor_with_type} {
        Accessor{accessor} {
            op: &mut self._op.{var},
            val: &mut self._data.{var},
            update: &mut self._update.{var},
            _phantom: Default::default(),
        }
    }", "") }@
@{- def.relations_one_owner()|fmt_rel_join("
{title}{comment}    pub fn {alias}(&mut self) -> AccessorOneToOne<rel_{class_mod}::{class}ForUpdate> {
        AccessorOneToOne {
            name: \"{alias}\",
            val: &mut self.{alias},
        }
    }", "") }@
@{- def.relations_many()|fmt_rel_join("
{title}{comment}    pub fn {alias}(&mut self) -> AccessorMany<rel_{class_mod}::{class}ForUpdate> {
        AccessorMany {
            name: \"{alias}\",
            val: &mut self.{alias},
        }
    }", "") }@
    pub(crate) fn _set_default_value(&mut self, conn: &DbConn) {
@%- if def.created_at_conf().is_some() %@
        if self._op.created_at == Op::None {
            self._data.created_at = @{(def.created_at_conf().unwrap() == Timestampable::RealTime)|if_then_else("SystemTime::now()","conn.time()")}@.into();
        }
@%- endif %@
@%- if def.updated_at_conf().is_some() %@
        if self._op.updated_at == Op::None {
            self._data.updated_at = @{(def.updated_at_conf().unwrap() == Timestampable::RealTime)|if_then_else("SystemTime::now()","conn.time()")}@.into();
        }
@%- endif %@
@%- if def.versioned %@
        self._data.@{ version_col }@ = 1;
@%- endif %@
        @{ def.inheritance_set() }@
@{- def.relations_one_owner()|fmt_rel_join("
        self.{alias}.as_mut().map(|v| v.as_mut().map(|v| v._set_default_value(conn)));", "") }@
@{- def.relations_many()|fmt_rel_join("
        if let Some(v) = self.{alias}.as_mut() {
            for v in v.iter_mut() {
                v._set_default_value(conn);
            }
        }", "") }@
    }
}

impl fmt::Display for _@{ pascal_name }@ForUpdate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let id = Primary::from(self);
        if self._is_new() {
            write!(f, "INSERT @{ table_name }@: ")?;
            @{- def.all_columns()|fmt_join("
            Accessor{accessor_with_sep_type}::_write_insert(f, \"{raw_var}\", &self._data.{var})?;", "") }@
        } else {
            write!(f, "UPDATE @{ table_name }@ {}: ", id)?;
            @{- def.non_primaries()|fmt_join("
            Accessor{accessor_with_sep_type}::_write_update(f, \"{raw_var}\", self._op.{var}, &self._update.{var})?;", "") }@
        }
        Ok(())
    }
}

impl From<Data> for _@{ pascal_name }@ {
    fn from(_inner: Data) -> Self {
        Self {
            _inner,
@{- def.relations_one()|fmt_rel_join("\n            {alias}: None,", "") }@
@{- def.relations_many()|fmt_rel_join("\n            {alias}: None,", "") }@
        }
    }
}

impl From<Arc<CacheWrapper>> for _@{ pascal_name }@Cache {
    fn from(wrapper: Arc<CacheWrapper>) -> Self {
        Self {
            _wrapper: wrapper,
@{- def.relations_one_only_cache()|fmt_rel_join("\n            {alias}: None,", "") }@
        }
    }
}

impl CacheWrapper {
    fn _from_inner(inner: CacheData, shard_id: ShardId, time: MSec) -> Self {
        Self {
            _inner: inner,
            _shard_id: shard_id,
            _time: time,
@{- def.relations_one_cache()|fmt_rel_join("\n            {alias}: None,", "") }@
@{- def.relations_many_cache()|fmt_rel_join("\n            {alias}: Vec::new(),", "") }@
        }
    }
    pub(crate) fn _from_data(data: Data, shard_id: ShardId, time: MSec) -> Self {
        Self {
            _inner: CacheData {
@{- def.cache_cols()|fmt_join("\n                {var}: data.{var},", "") }@
            },
            _shard_id: shard_id,
            _time: time,
@{- def.relations_one_cache()|fmt_rel_join("\n            {alias}: None,", "") }@
@{- def.relations_many_cache()|fmt_rel_join("\n            {alias}: Vec::new(),", "") }@
        }
    }
}

impl Serialize for _@{ pascal_name }@ {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        #[allow(unused_mut)]
        let mut len = @{ def.serializable().len() }@;
        @{- def.relations_one()|fmt_rel_join("
        if self.{alias}.is_some() {
            len += 1;
        }", "") }@
        @{- def.relations_many()|fmt_rel_join("
        if self.{alias}.is_some() {
            len += 1;
        }", "") }@
        let mut state = serializer.serialize_struct("@{ pascal_name }@", len)?;
        @{- def.serializable()|fmt_join("
        state.serialize_field(\"{var}\", &(self._inner.{var}{convert_serialize}))?;", "") }@
        @{- def.relations_one()|fmt_rel_join("
        if self.{alias}.is_some() {
            state.serialize_field(\"{alias}\", &self.{alias})?;
        }", "") }@
        @{- def.relations_many()|fmt_rel_join("
        if self.{alias}.is_some() {
            state.serialize_field(\"{alias}\", &self.{alias})?;
        }", "") }@
        state.end()
    }
}

impl Serialize for _@{ pascal_name }@Cache {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = @{ def.serializable_cache().len() + def.relations_one_cache().len() + def.relations_one_only_cache().len() + def.relations_many_cache().len() }@;
        let mut state = serializer.serialize_struct("@{ pascal_name }@", len)?;
        @{- def.serializable_cache()|fmt_join("
        state.serialize_field(\"{var}\", &(self._wrapper._inner.{var}{convert_serialize}))?;", "") }@
        @{- def.relations_one_cache()|fmt_rel_join("
        state.serialize_field(\"{alias}\", &self.{alias}())?;", "") }@
        @{- def.relations_many_cache()|fmt_rel_join("
        state.serialize_field(\"{alias}\", &self.{alias}())?;", "") }@
        @{- def.relations_one_only_cache()|fmt_rel_join("
        if self.{alias}.is_some() {
            state.serialize_field(\"{alias}\", &self.{alias})?;
        }", "") }@
        state.end()
    }
}

impl _@{ pascal_name }@ {
    pub fn for_update(conn: &DbConn) -> _@{ pascal_name }@ForUpdate {
        _@{ pascal_name }@ForUpdate {
            _data: Default::default(),
            _update: Default::default(),
            _is_new: false,
            _do_delete: false,
            _upsert: false,
            _is_loaded: false,
            _op: Default::default(),
@{- def.relations_one_owner()|fmt_rel_join("\n            {alias}: None,", "") }@
@{- def.relations_many()|fmt_rel_join("\n            {alias}: None,", "") }@
        }
    }
@%- for parent in def.downcast_aggregation() %@

    pub fn downcast_from(base: &crate::@{ parent.group_name }@::@{ parent.name }@::_@{ parent.name|pascal }@) -> Option<_@{ pascal_name }@> {
        if base._inner.@{ def.inheritance_check() }@ {
            let clone = base._inner.clone();
            Some(Self {
                _inner: Data {
                    @{- def.all_columns()|fmt_join("
                    {var}: clone.{var},", "") }@
                },
@{- def.relations_one()|fmt_rel_join("\n                {alias}: None,", "") }@
@{- def.relations_many()|fmt_rel_join("\n                {alias}: None,", "") }@
            })
        } else {
            None
        }
    }
@%- endfor %@
@%- for parent in def.downcast_simple() %@

    pub fn downcast_from(base: &crate::@{ parent.group_name }@::@{ parent.name }@::_@{ parent.name|pascal }@) -> _@{ pascal_name }@ {
        let clone = base._inner.clone();
        Self {
            _inner: Data {
                @{- def.all_columns()|fmt_join("
                {var}: clone.{var},", "") }@
            },
@{- def.relations_one()|fmt_rel_join("\n            {alias}: None,", "") }@
@{- def.relations_many()|fmt_rel_join("\n            {alias}: None,", "") }@
        }
    }
@%- endfor %@
@%- if def.use_cache_all() %@

    pub async fn find_all_from_cache(
        conn: &DbConn,
        condition: Option<Cond>,
        order_by: Option<Vec<OrderBy>>,
        limit: Option<usize>,
    ) -> Result<Arc<Vec<_@{ pascal_name }@Cache>>> {
        if let Some(arc) = CACHE_ALL.get().unwrap()[conn.shard_id() as usize].load_full() {
            return Ok(arc);
        }
        let _guard = BULK_FETCH_SEMAPHORE.get().unwrap()[conn.shard_id() as usize].acquire().await?;
        if let Some(arc) = CACHE_ALL.get().unwrap()[conn.shard_id() as usize].load_full() {
            return Ok(arc);
        }
        let mut conn = DbConn::_new(conn.shard_id());
        conn.begin_cache_tx().await?;
        let mut sql = format!(
            r#"SELECT {} FROM @{ table_name|db_esc }@ as _t1 {} {}"#,
            CacheData::_sql_cols("`"),
            Cond::write_where(
                &condition,
                TrashMode::Not,
                TRASHED_SQL,
                NOT_TRASHED_SQL,
                ONLY_TRASHED_SQL
            ),
            OrderBy::write_order_by(&order_by)
        );
        if let Some(limit) = limit {
            let _ = write!(sql, " limit {}", limit);
        }
        let mut query = sqlx::query_as::<_, CacheData>(&sql);
        let _span = info_span!("query", sql = &query.sql());
        if let Some(c) = condition {
            query = c.bind(query);
        }
        let result = crate::misc::fetch!(conn, query, fetch_all);
        let list: Vec<_@{ pascal_name }@Cache> = result.into_iter().map(|data| Arc::new(CacheWrapper::_from_inner(data, conn.shard_id(), conn.begin_time())).into()).collect();
        let arc = Arc::new(list);
        if MSec::from(CACHE_RESET_TIME.load(Ordering::Relaxed)).less_than(conn.begin_time())
        {
            CACHE_ALL.get().unwrap()[conn.shard_id() as usize].swap(Some(Arc::clone(&arc)));
        }
        Ok(arc)
    }
    @%- endif %@

    pub fn query() -> QueryBuilder {
        QueryBuilder::default()
    }

    pub async fn clear_cache() {
        CacheMsg(vec![CacheOp::InvalidateAll.wrap()], MSec::now())
            .do_send()
            .await;
    }

    fn _clear_cache() {
        CACHE_RESET_TIME.store(MSec::now().add(1).get(), Ordering::Relaxed);
        clear_cache_all();
        if USE_CACHE {
            Cache::invalidate_all_of::<CacheWrapper>();
            Cache::invalidate_all_of::<PrimaryWrapper>();
            Cache::invalidate_all_of_version::<VersionWrapper>();
        }
    }

    async fn _find_many(
        conn: &mut DbConn,
        ids: &[Primary],
    ) -> Result<Vec<Data>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let id_chunks = ids.chunks(IN_CONDITION_LIMIT);
        let mut list = Vec::with_capacity(ids.len());
        for ids in id_chunks {
            let mut v = Self::__find_many(conn, ids).await?;
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

    async fn __find_many<T>(conn: &mut DbConn, ids: &[Primary]) -> Result<Vec<T>>
    where
        T: for<'r> sqlx::FromRow<'r, <DbType as sqlx::Database>::Row> + SqlColumns + Send + Unpin,
    {
        use futures::TryStreamExt;
        use sqlx::Executor;
        let mut sql = String::new();
        for id in ids {
            let _ = write!(sql, 
                r#"SELECT {} FROM @{ table_name|db_esc }@ WHERE @{ def.inheritance_cond(" AND ") }@@{ def.primaries()|fmt_join("{col_esc}='{}'", " AND ") }@;"#,
                T::_sql_cols("`"),
                @{ def.primaries()|fmt_join("check_id(id.{index})?", ", ") }@
            );
        }
        let mut list = Vec::new();
        if conn.has_tx() {
            let mut stream = conn.get_tx().await?.fetch_many(&*sql);
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
    async fn __find_many<T>(conn: &mut DbConn, ids: &[Primary]) -> Result<Vec<T>>
    where
        T: for<'r> sqlx::FromRow<'r, <DbType as sqlx::Database>::Row> + SqlColumns + Send + Unpin,
    {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let q = "@{ def.primaries()|fmt_join_with_paren("?", ",") }@,".repeat(ids.len());
        let sql = format!(
            r#"SELECT {} FROM @{ table_name|db_esc }@ WHERE @{ def.inheritance_cond(" AND ") }@@{ def.primaries()|fmt_join_with_paren("{col_esc}", ",") }@ in ({});"#,
            T::_sql_cols("`"),
            &q[0..q.len() - 1]
        );
        let mut query = sqlx::query_as::<_, T>(&sql);
        let _span = info_span!("query", sql = &query.sql());
        for id in ids {
            @{- def.primaries()|fmt_join("
            query = query.bind(&id.{index});", "") }@
        }
        let result = crate::misc::fetch!(conn, query, fetch_all);
        Ok(result)
    }
@%- endif %@
@%- if def.use_cache() %@

    pub(crate) async fn find_many_for_cache<I, T>(conn: &mut DbConn, ids: I) -> Result<FxHashMap<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@, _@{ pascal_name }@Cache>>
    where
        I: IntoIterator<Item = T>,
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        let ids: Vec<Primary> = ids.into_iter().map(|id| (&id.into()).into()).collect();
        let list = Self::_find_many_for_cache(conn, &ids).await?;
        let map = list.into_iter()@{- def.soft_delete_tpl("",".filter(|data| data._inner.deleted_at.is_none())",".filter(|data| data._inner.deleted == 0)")}@.fold(FxHashMap::default(), |mut map, v| {
            map.insert(@{ def.primaries()|fmt_join_with_paren("v._inner.{var}{clone}.into()", ", ") }@, Arc::new(v).into());
            map
        });
        Ok(map)
    }

    async fn _find_many_for_cache(
        conn: &mut DbConn,
        ids: &[Primary],
    ) -> Result<Vec<CacheWrapper>> {
        if ids.is_empty() {
            return Ok(Vec::new());
        }
        let id_chunks = ids.chunks(IN_CONDITION_LIMIT);
        let mut result: Vec<CacheData> = Vec::with_capacity(ids.len());
        for ids in id_chunks {
            let mut v = Self::__find_many(conn, ids).await?;
            result.append(&mut v);
        }
        let list = result.into_iter().map(|v| CacheWrapper::_from_inner(v, conn.shard_id(), conn.begin_time())).collect();
        Ok(list)
    }
    @%- endif %@

    pub async fn find<T>(conn: &mut DbConn, id: T) -> Result<_@{ pascal_name }@>
    where
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        let id: @{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@ = id.into();
        Self::find_optional(conn, id.clone())
            .await?
            .with_context(|| err::RowNotFound::new("@{ table_name }@", id_to_string(&id)))
    }

    pub async fn find_with_trashed<T>(conn: &mut DbConn, id: T) -> Result<_@{ pascal_name }@>
    where
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        let id: @{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@ = id.into();
        Self::find_optional_with_trashed(conn, id.clone())
            .await?
            .with_context(|| err::RowNotFound::new("@{ table_name }@", id_to_string(&id)))
    }
    @%- if def.use_cache() %@

    pub async fn find_from_cache<T>(conn: &DbConn, id: T) -> Result<_@{ pascal_name }@Cache>
    where
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        let id: @{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@ = id.into();
        Self::_find_optional_from_cache(conn, id.clone())
            .await?
@{- def.soft_delete_tpl("","\n            .filter(|data| data._wrapper._inner.deleted_at.is_none())","\n            .filter(|data| data._wrapper._inner.deleted == 0)")}@
            .with_context(|| err::RowNotFound::new("@{ table_name }@", id_to_string(&id)))
    }

    pub async fn find_from_cache_with_trashed<T>(conn: &DbConn, id: T) -> Result<_@{ pascal_name }@Cache>
    where
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        let id: @{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@ = id.into();
        Self::_find_optional_from_cache(conn, id.clone())
            .await?
            .with_context(|| err::RowNotFound::new("@{ table_name }@", id_to_string(&id)))
    }
    @%- endif %@

    pub async fn find_many<I, T>(conn: &mut DbConn, ids: I) -> Result<AHashMap<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@, _@{ pascal_name }@>>
    where
        I: IntoIterator<Item = T>,
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        let ids: Vec<Primary> = ids.into_iter().map(|id| (&id.into()).into()).collect();
        let list = Self::_find_many(conn, &ids).await?;
        let map = list.into_iter()@{ def.soft_delete_tpl("",".filter(|data| data.deleted_at.is_none())",".filter(|data| data.deleted == 0)")}@.fold(AHashMap::default(), |mut map, v| {
            map.insert(@{ def.primaries()|fmt_join_with_paren("v.{var}{clone}.into()", ", ") }@, v.into());
            map
        });
        Ok(map)
    }

    pub async fn find_many_with_trashed<I, T>(conn: &mut DbConn, ids: I) -> Result<AHashMap<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@, _@{ pascal_name }@>>
    where
        I: IntoIterator<Item = T>,
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        let ids: Vec<Primary> = ids.into_iter().map(|id| (&id.into()).into()).collect();
        let list = Self::_find_many(conn, &ids).await?;
        let map = list.into_iter().fold(AHashMap::default(), |mut map, v| {
            map.insert(@{ def.primaries()|fmt_join_with_paren("v.{var}{clone}.into()", ", ") }@, v.into());
            map
        });
        Ok(map)
    }
    @%- if def.use_cache() %@

    pub async fn find_many_from_cache<I, T>(conn: &DbConn, ids: I) -> Result<AHashMap<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@, _@{ pascal_name }@Cache>>
    where
        I: IntoIterator<Item = T>,
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        let list = Self::_find_many_from_cache(conn, ids).await?;
        let map = list.into_iter()@{ def.soft_delete_tpl("",".filter(|data| data._wrapper._inner.deleted_at.is_none())",".filter(|data| data._wrapper._inner.deleted == 0)")}@.fold(AHashMap::default(), |mut map, v| {
            map.insert(@{ def.primaries()|fmt_join_with_paren("v._wrapper._inner.{var}{clone}.into()", ", ") }@, v);
            map
        });
        Ok(map)
    }

    pub async fn find_many_from_cache_with_trashed<I, T>(conn: &DbConn, ids: I) -> Result<AHashMap<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@, _@{ pascal_name }@Cache>>
    where
        I: IntoIterator<Item = T>,
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        let list = Self::_find_many_from_cache(conn, ids).await?;
        let map = list.into_iter().fold(AHashMap::default(), |mut map, v| {
            map.insert(@{ def.primaries()|fmt_join_with_paren("v._wrapper._inner.{var}{clone}.into()", ", ") }@, v);
            map
        });
        Ok(map)
    }

    async fn _find_many_from_cache<I, T>(conn: &DbConn, ids: I) -> Result<Vec<_@{ pascal_name }@Cache>>
    where
        I: IntoIterator<Item = T>,
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        let mut list: Vec<_@{ pascal_name }@Cache> = Vec::new();
        let mut rest_ids = Vec::new();
        let shard_id = conn.shard_id();
        let mut conn = DbConn::_new(shard_id);
        let ids: Vec<_> = ids.into_iter().map(|id| PrimaryHasher((&id.into()).into(), shard_id)).collect();
        let cache_map = Cache::get_many::<CacheWrapper>(&ids.iter().map(|id| id.hash_val(shard_id)).collect(), shard_id, USE_FAST_CACHE).await;
        for id in ids {
            if let Some(obj) = cache_map.get(&id.hash_val(shard_id)).filter(|o| Primary::from(*o) == id.0) {
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
                if let Some(obj) = Cache::get_from_memory::<CacheWrapper>(&id, shard_id, USE_FAST_CACHE).await.filter(|o| Primary::from(o) == id.0) {
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
                let ids: Vec<Primary> = ids.drain().collect();
                #[allow(unused_mut)]
                let mut result = Self::_find_many_for_cache(&mut conn, &ids).await?;
@{- def.relations_cache()|fmt_rel_join("\n                CacheWrapper::fetch_{raw_alias}_4vec(&mut result, &mut conn).await?;", "") }@
                for v in result.into_iter() {
                    let arc = Arc::new(v);
                    let id = PrimaryHasher(Primary::from(&arc), shard_id);
                    @%- if def.versioned %@
                    let vw = VersionWrapper{
                        id: id.0.clone(),
                        shard_id,
                        time: MSec::default(),
                        version: 0,
                    };
                    if MSec::from(CACHE_RESET_TIME.load(Ordering::Relaxed)).less_than(conn.begin_time())
                    {
                        if let Some(ver) = Cache::get_version::<VersionWrapper>(&vw, shard_id).await.filter(|o| o.id == id.0) {
                            if arc._inner.@{ version_col }@.greater_equal(ver.version) {
                                Cache::insert_long(&id, arc.clone(), USE_FAST_CACHE).await;
                            }
                        } else {
                            Cache::insert_long(&id, arc.clone(), USE_FAST_CACHE).await;
                        }
                    }
                    @%- else %@
                    Cache::insert_long(&id, arc.clone(), USE_FAST_CACHE).await;
                    @%- endif %@
                    if rest_ids2.contains(&id) {
                        list.push(arc.into());
                    }
                }
            }
        }
        conn.release_cache_tx();
@{- def.relations_one_only_cache()|fmt_rel_join("\n        list.fetch_{raw_alias}(&mut conn).await?;", "") }@
        Ok(list)
    }
    @%- endif %@

    pub async fn find_optional<T>(conn: &mut DbConn, id: T) -> Result<Option<_@{ pascal_name }@>>
    where
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        let id: Primary = (&id.into()).into();
        let data: Option<Data> = Self::_find_optional(conn, id).await?;
        Ok(data@{ def.soft_delete_tpl("",".filter(|data| data.deleted_at.is_none())",".filter(|data| data.deleted == 0)")}@.map(_@{ pascal_name }@::from))
    }

    pub async fn find_optional_for_update<T>(conn: &mut DbConn, id: T) -> Result<Option<_@{ pascal_name }@ForUpdate>>
    where
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        let id: Primary = (&id.into()).into();
        let result = Self::_find_for_update(conn, &id).await?@{ def.soft_delete_tpl("",".filter(|data| data.deleted_at.is_none())",".filter(|data| data.deleted == 0)")}@;
        Ok(result@{ def.soft_delete_tpl("",".filter(|data| data.deleted_at.is_none())",".filter(|data| data.deleted == 0)")}@.map(|data| ForUpdate {
            _data: data,
            _update: Default::default(),
            _is_new: false,
            _do_delete: false,
            _upsert: false,
            _is_loaded: true,
            _op: Default::default(),
@{- def.relations_one_owner()|fmt_rel_join("\n            {alias}: None,", "") }@
@{- def.relations_many()|fmt_rel_join("\n            {alias}: None,", "") }@
        }))
    }

    pub async fn find_optional_with_trashed<T>(conn: &mut DbConn, id: T) -> Result<Option<_@{ pascal_name }@>>
    where
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        let id: Primary = (&id.into()).into();
        let data: Option<Data> = Self::_find_optional(conn, id).await?;
        Ok(data.map(_@{ pascal_name }@::from))
    }
    @%- if def.use_cache() %@

    pub(crate) async fn find_optional_for_cache<T>(conn: &mut DbConn, id: T) -> Result<Option<_@{ pascal_name }@Cache>>
    where
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        let id: Primary = (&id.into()).into();
        let data: Option<CacheData> = Self::_find_optional(conn, id).await?;
        Ok(data@{ def.soft_delete_tpl("",".filter(|data| data.deleted_at.is_none())",".filter(|data| data.deleted == 0)")}@.map(|v| Arc::new(CacheWrapper::_from_inner(v, conn.shard_id(), conn.begin_time())).into()))
    }
    @%- endif %@

    async fn _find_optional<T>(conn: &mut DbConn, id: Primary) -> Result<Option<T>>
    where
        T: for<'r> sqlx::FromRow<'r, <DbType as sqlx::Database>::Row> + SqlColumns + Send + Unpin,
    {
        let sql = format!(r#"SELECT {} FROM @{ table_name|db_esc }@ WHERE @{ def.inheritance_cond(" AND ") }@@{ def.primaries()|fmt_join("{col_esc}=?", " AND ") }@"#, T::_sql_cols("`"));
        let mut query = sqlx::query_as::<_, T>(&sql);
        let _span = info_span!("query", sql = &query.sql());
        @{- def.primaries()|fmt_join("
        query = query.bind(&id.{index});", "") }@
        Ok(crate::misc::fetch!(conn, query, fetch_optional))
    }
    @%- if def.use_cache() %@

    #[allow(clippy::needless_question_mark)]
    pub async fn find_optional_from_cache<T>(conn: &DbConn, id: T) -> Result<Option<_@{ pascal_name }@Cache>>
    where
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        Ok(Self::_find_optional_from_cache(conn, id).await?@{- def.soft_delete_tpl("",".filter(|data| data._wrapper._inner.deleted_at.is_none())",".filter(|data| data._wrapper._inner.deleted == 0)")}@)
    }

    pub async fn find_optional_from_cache_with_trashed<T>(conn: &DbConn, id: T) -> Result<Option<_@{ pascal_name }@Cache>>
    where
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        Self::_find_optional_from_cache(conn, id).await
    }

    async fn _find_optional_from_cache<T>(conn: &DbConn, id: T) -> Result<Option<_@{ pascal_name }@Cache>>
    where
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        let id: @{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@ = id.into();
        let mut result = Self::_find_many_from_cache(conn, [id]).await?;
        Ok(result.pop())
    }
    @%- endif %@

    pub async fn find_for_update<T>(conn: &mut DbConn, id: T) -> Result<_@{ pascal_name }@ForUpdate>
    where
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        let id: Primary = (&id.into()).into();
        let result = Self::_find_for_update(conn, &id).await?@{ def.soft_delete_tpl("",".filter(|data| data.deleted_at.is_none())",".filter(|data| data.deleted == 0)")}@;
        let data = result.with_context(|| err::RowNotFound::new("@{ table_name }@", id.to_string()))?;
        Ok(ForUpdate {
            _data: data,
            _update: Default::default(),
            _is_new: false,
            _do_delete: false,
            _upsert: false,
            _is_loaded: true,
            _op: Default::default(),
@{- def.relations_one_owner()|fmt_rel_join("\n            {alias}: None,", "") }@
@{- def.relations_many()|fmt_rel_join("\n            {alias}: None,", "") }@
        })
    }

    pub async fn find_for_update_with_trashed<T>(conn: &mut DbConn, id: T) -> Result<_@{ pascal_name }@ForUpdate>
    where
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        let id: Primary = (&id.into()).into();
        let result = Self::_find_for_update(conn, &id).await?;
        let data = result.with_context(|| err::RowNotFound::new("@{ table_name }@", id.to_string()))?;
        Ok(ForUpdate {
            _data: data,
            _update: Default::default(),
            _is_new: false,
            _do_delete: false,
            _upsert: false,
            _is_loaded: true,
            _op: Default::default(),
@{- def.relations_one_owner()|fmt_rel_join("\n            {alias}: None,", "") }@
@{- def.relations_many()|fmt_rel_join("\n            {alias}: None,", "") }@
        })
    }

    async fn _find_for_update(conn: &mut DbConn, id: &Primary) -> Result<Option<Data>> {
        let sql = format!(r#"SELECT {} FROM @{ table_name|db_esc }@ WHERE @{ def.inheritance_cond(" AND ") }@@{ def.primaries()|fmt_join("{col_esc}=?", " AND ") }@ FOR UPDATE"#, Data::_sql_cols("`"));
        let mut query = sqlx::query_as::<_, Data>(&sql);
        let _span = info_span!("query", sql = &query.sql());
        @{- def.primaries()|fmt_join("
        query = query.bind(&id.{index});", "") }@
        Ok(query.fetch_optional(conn.get_tx().await?).await?)
    }
@%- if def.primaries().len() == 1 %@

    pub async fn find_many_for_update<I, T>(conn: &mut DbConn, ids: I) -> Result<FxHashMap<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@, _@{ pascal_name }@ForUpdate>>
    where
        I: IntoIterator<Item = T>,
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        let ids: Vec<Primary> = ids.into_iter().map(|id| (&id.into()).into()).collect();
        if ids.is_empty() {
            return Ok(FxHashMap::default());
        }
        let mut list: Vec<ForUpdate> = Vec::with_capacity(ids.len());
        let id_chunks = ids.chunks(IN_CONDITION_LIMIT);
        for ids in id_chunks {
            let q = "?,".repeat(ids.len());
            let sql = format!(
                r#"SELECT {} FROM @{ table_name|db_esc }@ WHERE {}@{ def.primaries()|fmt_join("{col_esc}", ",") }@ in ({}) FOR UPDATE;"#,
                Data::_sql_cols("`"),
                &q[0..q.len() - 1],
                NOT_TRASHED_SQL
            );
            let mut query = sqlx::query_as::<_, Data>(&sql);
            let _span = info_span!("query", sql = &query.sql());
            for id in ids {
                query = query.bind(&id.0);
            }
            let result = query.fetch_all(conn.get_tx().await?).await?;
            result
                .into_iter()
                .map(|data| ForUpdate {
                    _data: data,
                    _update: Default::default(),
                    _is_new: false,
                    _do_delete: false,
                    _upsert: false,
                    _is_loaded: true,
                    _op: Default::default(),
@{- def.relations_one_owner()|fmt_rel_join("\n                    {alias}: None,", "") }@
@{- def.relations_many()|fmt_rel_join("\n                    {alias}: None,", "") }@
                })
                .for_each(|obj| list.push(obj));
        }
        let map = list.into_iter()@{ def.soft_delete_tpl("",".filter(|data| data._data.deleted_at.is_none())",".filter(|data| data._data.deleted == 0)")}@.fold(FxHashMap::default(), |mut map, v| {
            map.insert(@{ def.primaries()|fmt_join_with_paren("v._data.{var}{clone}.into()", ", ") }@, v);
            map
        });
        Ok(map)
    }
@%- endif %@
@%- for (index_name, index) in def.unique_index() %@

    pub async fn find_by_@{ index_name }@<@{ index.fields(index_name, def)|fmt_index_col("T{index}", ", ") }@>(conn: &mut DbConn, @{ index.fields(index_name, def)|fmt_index_col("_{name}: T{index}", ", ") }@) -> Result<_@{ pascal_name }@>
    where
    @{- index.fields(index_name, def)|fmt_index_col("
        T{index}: Into<{cond_type}>,", "") }@
    {
        @{- index.fields(index_name, def)|fmt_index_col("
        let val{index}: {cond_type} = _{name}.into();", "") }@
        let cond = Cond::And(vec![@{- index.fields(index_name, def)|fmt_index_col("Cond::EqKey(ColKey::{var}(val{index}.clone()))", ", ") }@]);
        Self::query().cond(cond).select(conn).await?.pop()
            .with_context(|| err::RowNotFound::new("@{ table_name }@", format!("@{ index.fields(index_name, def)|fmt_index_col("{col_name}={}", ", ") }@", @{ index.fields(index_name, def)|fmt_index_col("val{index}", ", ") }@)))
    }
@%- endfor  %@
@%- for (index_name, index) in def.unique_index() %@

    pub async fn find_by_@{ index_name }@_from_cache<@{ index.fields(index_name, def)|fmt_index_col("T{index}", ", ") }@>(conn: &DbConn, @{ index.fields(index_name, def)|fmt_index_col("_{name}: T{index}", ", ") }@) -> Result<_@{ pascal_name }@Cache>
    where
    @{- index.fields(index_name, def)|fmt_index_col("
        T{index}: Into<{cond_type}>,", "") }@
    {
        @{- index.fields(index_name, def)|fmt_index_col("
        let val{index}: {cond_type} = _{name}.into();", "") }@
        let key = VecColKey(vec![@{- index.fields(index_name, def)|fmt_index_col("ColKey::{var}(val{index}.clone())", ", ") }@]);
        let cond = Cond::And(vec![@{- index.fields(index_name, def)|fmt_index_col("Cond::EqKey(ColKey::{var}(val{index}.clone()))", ", ") }@]);
        if let Some(id) = Cache::get::<PrimaryWrapper>(&key, conn.shard_id(), true).await {
            if let Some(obj) = Self::find_optional_from_cache(conn, &id.0).await? {
                if @{ index.fields(index_name, def)|fmt_index_col_not_null_or_null("obj.{var}() == val{index}", "matches!(obj.{var}(), Some(v) if v == val{index})", " && ") }@ {
                    return Ok(obj);
                }
            }
        }
        let mut conn = DbConn::_new(conn.shard_id());
        conn.begin_cache_tx().await?;
        let obj = Self::query().cond(cond.clone()).select_from_cache(&mut conn).await?.pop()
            .with_context(|| err::RowNotFound::new("@{ table_name }@", format!("@{ index.fields(index_name, def)|fmt_index_col("{col_name}={}", ", ") }@", @{ index.fields(index_name, def)|fmt_index_col("val{index}", ", ") }@)))?;
        let id = PrimaryWrapper(Primary::from(&obj), conn.shard_id(), conn.begin_time());
        Cache::insert_long(&key, Arc::new(id), true).await;
        Ok(obj)
    }
@%- endfor  %@

    pub async fn insert_dummy_cache(conn: &DbConn, obj: _@{ pascal_name }@ForUpdate) -> Result<()> {
        let cache_msg = CacheOp::Insert {
            shard_id: conn.shard_id(),
            data: obj._data,
@{- def.relations_one_cache()|fmt_rel_join("\n            _{alias}: None,", "") }@
@{- def.relations_many_cache()|fmt_rel_join("\n            _{alias}: None,", "") }@
        };
        cache_msg.handle_cache_msg(MSec::now(), false).await;
        Ok(())
    }

    pub async fn save(conn: &mut DbConn, mut obj: _@{ pascal_name }@ForUpdate) -> Result<Option<_@{ pascal_name }@>> {
        obj._data.validate()?;
        if obj._is_new() {
            obj._set_default_value(conn);
        }
        @%- if def.updated_at_conf().is_some() %@
        if obj._op.updated_at == Op::None {
            obj.updated_at().set(@{(def.updated_at_conf().unwrap() == Timestampable::RealTime)|if_then_else("SystemTime::now()","conn.time()")}@.into());
        }
        @%- endif %@
        Self::_save(conn, obj).await
    }

    async fn _save(conn: &mut DbConn, obj: _@{ pascal_name }@ForUpdate) -> Result<Option<_@{ pascal_name }@>> {
        let (obj, cache_msg) = Self::__save(conn, obj).await?;
        if !conn.clear_all_cache && (USE_CACHE || USE_CACHE_ALL) {
            if let Some(cache_msg) = cache_msg {
                conn.push_cache_op(cache_msg.wrap()).await;
            }
        }
        Ok(obj)
    }

    #[allow(clippy::type_complexity)]
    pub(crate) fn __save<'a>(
        conn: &'a mut DbConn,
        obj: _@{ pascal_name }@ForUpdate,
    ) -> Pin<Box<dyn futures::Future<Output = Result<(Option<_@{ pascal_name }@>, Option<CacheOp>)>> + 'a>>
    {
        Box::pin(async move {
            if obj._will_be_deleted() {
                if obj._is_new() || obj._has_been_deleted() {
                    return Ok((None, None));
                }
                let cache_msg = Self::_delete(conn, obj).await?;
                return Ok((None, cache_msg));
            }
            if obj._is_new() && obj._upsert {
                let (obj, cache_msg) = Self::_save_upsert(conn, obj).await?;
                Ok((Some(obj), cache_msg))
            } else if obj._is_new() {
                let (obj, cache_msg) = Self::_save_insert(conn, obj).await?;
                Ok((Some(obj), Some(cache_msg)))
            } else {
                let (obj, cache_msg) = Self::_save_update(conn, obj).await?;
                Ok((Some(obj), cache_msg))
            }
        })
    }

    async fn _save_insert(conn: &mut DbConn, mut obj: _@{ pascal_name }@ForUpdate) -> Result<(_@{ pascal_name }@, CacheOp)> {
        let sql = "INSERT INTO @{ table_name|db_esc }@ \
            (@{ def.all_columns()|fmt_join("{col_esc}", ",") }@) \
            VALUES (@{ def.all_columns()|fmt_join("{placeholder}", ",") }@)";
        let query = query_bind(sql, &obj._data);
        let _span = info_span!("query", sql = &query.sql());
        let result = if conn.wo_tx() {
            query.execute(&mut conn.acquire_source().await?).await?
        } else {
            query.execute(conn.get_tx().await?).await?
        };
@{- def.auto_increments()|fmt_join("
        if obj._data.{var} == 0 {
            obj._data.{var} = result.last_insert_id() as {inner};
        }", "") }@
        debug!("{}", &obj);
        let mut obj2: _@{ pascal_name }@ = obj._data.clone().into();
        let mut update_cache = true;

        let default = Data::default();
        @{- def.non_primaries()|fmt_join_cache_or_not("", "
        obj._data.{var} = default.{var};", "") }@
        let cache_msg = CacheOp::Insert {
            shard_id: conn.shard_id(),
            data: obj._data,
@{- def.relations_one_cache()|fmt_rel_join("\n            _{alias}: save_{alias}(conn, &mut obj2, obj.{alias}, &mut update_cache).await?,", "") }@
@{- def.relations_many_cache()|fmt_rel_join("\n            _{alias}: save_{alias}(conn, &mut obj2, obj.{alias}, &mut update_cache).await?,", "") }@
        };
        Ok((obj2, cache_msg))
    }

    #[allow(unused_mut)]
    async fn _save_update(conn: &mut DbConn, mut obj: _@{ pascal_name }@ForUpdate) -> Result<(_@{ pascal_name }@, Option<CacheOp>)> {
        if !obj._is_updated() {
            return Ok((obj.into(), None));
        }
        let id = Primary::from(&obj);
        let mut update_cache = false; // To distinguish from updates that do not require cache updates
        let mut vec: Vec<String> = Vec::new();
        @{- def.non_primaries()|fmt_join_cache_or_not("
        assignment_sql!(obj, vec, {var}, \"{col_esc}\", {may_null}, update_cache, \"{placeholder}\");", "
        assignment_sql_no_cache_update!(obj, vec, {var}, \"{col_esc}\", {may_null}, \"{placeholder}\");", "") }@
        @%- if def.versioned %@
        vec.push("`@{ version_col }@` = LAST_INSERT_ID(IF(`@{ version_col }@` < 4294967295, `@{ version_col }@` + 1, 0))".to_string());
        @%- endif %@
        @%- if def.counting.is_some() %@
        vec.push("`@{ def.get_counting_col() }@` = LAST_INSERT_ID(`@{ def.get_counting_col() }@`)".to_string());
        @%- endif %@
        let sql = format!(r#"UPDATE @{ table_name|db_esc }@ SET {} WHERE @{ def.inheritance_cond(" AND ") }@@{ def.primaries()|fmt_join("{col_esc}=?", " AND ") }@"#, &vec.join(","));
        let mut query = sqlx::query(&sql);
        let _span = info_span!("query", sql = &query.sql());
        @{- def.non_primaries()|fmt_join("
        bind_sql!(obj, query, {var}, {may_null});","") }@
        query = query@{ def.primaries()|fmt_join(".bind(id.{index})", "") }@;
        debug!("{}", &obj);
        let result = if conn.wo_tx() {
            query.execute(&mut conn.acquire_source().await?).await?
        } else {
            query.execute(conn.get_tx().await?).await?
        };
        @%- if def.versioned %@
        obj.@{ version_col }@().set(result.last_insert_id() as u32);
        @%- endif %@
        @%- if def.counting.is_some() %@
        if obj._op.@{ def.get_counting() }@ == Op::Add {
            obj._op.@{ def.get_counting() }@ = Op::Max;
            obj._update.@{ def.get_counting() }@ = result.last_insert_id().try_into().unwrap_or(@{ def.get_counting_type() }@::MAX);
        }
        @%- endif %@

        let mut obj2: _@{ pascal_name }@ = obj._data.into();

        let default = Data::default();
        @{- def.non_primaries()|fmt_join_cache_or_not("", "
        obj._op.{var} = Op::None;
        obj._update.{var} = default.{var};", "") }@
        let mut cache_msg = Some(CacheOp::Update {
            id,
            shard_id: conn.shard_id(),
            update:obj._update,
            op: obj._op,
@{- def.relations_one_cache()|fmt_rel_join("\n            _{alias}: save_{alias}(conn, &mut obj2, obj.{alias}, &mut update_cache).await?,", "") }@
@{- def.relations_many_cache()|fmt_rel_join("\n            _{alias}: save_{alias}(conn, &mut obj2, obj.{alias}, &mut update_cache).await?,", "") }@
        });
        if !update_cache {
            cache_msg = None;
        }
        Ok((obj2, cache_msg))
    }

    async fn _save_upsert(conn: &mut DbConn, mut obj: _@{ pascal_name }@ForUpdate) -> Result<(_@{ pascal_name }@, Option<CacheOp>)> {
        let mut update_cache = true;
        let mut vec: Vec<String> = Vec::new();
        @{- def.non_primaries()|fmt_join_cache_or_not("
        assignment_sql!(obj, vec, {var}, \"{col_esc}\", {may_null}, update_cache, \"{placeholder}\");", "
        assignment_sql_no_cache_update!(obj, vec, {var}, \"{col_esc}\", {may_null}, \"{placeholder}\");", "") }@
        @%- if def.versioned %@
        vec.push("`@{ version_col }@` = LAST_INSERT_ID(IF(`@{ version_col }@` < 4294967295, `@{ version_col }@` + 1, 0))".to_string());
        @%- endif %@
        @%- if def.counting.is_some() %@
        vec.push("`@{ def.get_counting_col() }@` = LAST_INSERT_ID(`@{ def.get_counting_col() }@`)".to_string());
        @%- endif %@
        let sql = format!("INSERT INTO @{ table_name|db_esc }@ \
            (@{ def.all_columns()|fmt_join("{col_esc}", ",") }@) \
            VALUES (@{ def.all_columns()|fmt_join("{placeholder}", ",") }@) ON DUPLICATE KEY UPDATE {}", &vec.join(","));
        let mut query = query_bind(&sql, &obj._data);
        let _span = info_span!("query", sql = &query.sql());
@{- def.non_primaries()|fmt_join("
        bind_sql!(obj, query, {var}, {may_null});","") }@
        let result = if conn.wo_tx() {
            query.execute(&mut conn.acquire_source().await?).await?
        } else {
            query.execute(conn.get_tx().await?).await?
        };
        debug!("{}", &obj);
        if result.rows_affected() == 1 {
            @{- def.auto_increments()|fmt_join("
            if obj._data.{var} == 0 {
                obj._data.{var} = result.last_insert_id() as {inner};
            }", "") }@
            let mut obj2: _@{ pascal_name }@ = obj._data.clone().into();
            let cache_msg = CacheOp::Insert {
                shard_id: conn.shard_id(),
                data: obj._data,
@{- def.relations_one_cache()|fmt_rel_join("\n                _{alias}: None,", "") }@
@{- def.relations_many_cache()|fmt_rel_join("\n                _{alias}: None,", "") }@
            };
            Ok((obj2, Some(cache_msg)))
        } else if result.rows_affected() == 2 {
            @%- if def.versioned %@
            obj.@{ version_col }@().set(result.last_insert_id() as u32);
            @%- endif %@
            @%- if def.counting.is_some() %@
            if obj._op.@{ def.get_counting() }@ == Op::Add {
                obj._op.@{ def.get_counting() }@ = Op::Max;
                obj._update.@{ def.get_counting() }@ = result.last_insert_id().try_into().unwrap_or(@{ def.get_counting_type() }@::MAX);
            }
            @%- endif %@
            let id = Primary::from(&obj);
            let mut obj2: _@{ pascal_name }@ = obj._data.into();
            let mut cache_msg = Some(CacheOp::Update {
                id,
                shard_id: conn.shard_id(),
                update:obj._update,
                op: obj._op,
@{- def.relations_one_cache()|fmt_rel_join("\n                _{alias}: None,", "") }@
@{- def.relations_many_cache()|fmt_rel_join("\n                _{alias}: None,", "") }@
            });
            if !update_cache {
                cache_msg = None;
            }
            Ok((obj2, cache_msg))
        } else {
            let mut obj2: _@{ pascal_name }@ = obj._data.into();
            Ok((obj2, None))
        }
    }

    /// insert_delayed does not return an error.
    /// insert_ignore does not save the relations table.
    pub async fn insert_ignore(conn: &mut DbConn, mut obj: _@{ pascal_name }@ForUpdate) -> Result<Option<_@{ pascal_name }@ForUpdate>> {
        obj._data.validate()?;
        ensure!(obj._is_new(), "The obj is not new.");
        obj._set_default_value(conn);
        let sql = "INSERT IGNORE INTO @{ table_name|db_esc }@ (@{ def.all_columns()|fmt_join("{col_esc}", ",") }@) \
            VALUES (@{ def.all_columns()|fmt_join("{placeholder}", ",") }@)";
        let query = query_bind(sql, &obj._data);
        let _span = info_span!("query", sql = &query.sql());
        let result = if conn.wo_tx() {
            query.execute(&mut conn.acquire_source().await?).await?
        } else {
            query.execute(conn.get_tx().await?).await?
        };
        if result.rows_affected() == 0 {
            return Ok(None);
        }
@{- def.auto_increments()|fmt_join("
        if obj._data.{var} == 0 {
            obj._data.{var} = result.last_insert_id() as {inner};
        }", "") }@
        debug!("{}", &obj);
        obj._is_new = false;
        obj._op = Default::default();
        if !conn.clear_all_cache && (USE_CACHE || USE_CACHE_ALL) {
            let cache_msg = CacheOp::Insert {
                shard_id: conn.shard_id(),
                data: obj._data.clone(),
@{- def.relations_one_cache()|fmt_rel_join("\n                _{alias}: None,", "") }@
@{- def.relations_many_cache()|fmt_rel_join("\n                _{alias}: None,", "") }@
            };
            conn.push_cache_op(cache_msg.wrap()).await;
        }
        Ok(Some(obj))
    }

    /// If insert_delayed is used, the data will be collectively registered later.
    /// insert_delayed does not save the relations table.
    pub async fn insert_delayed(conn: &mut DbConn, mut obj: _@{ pascal_name }@ForUpdate) -> Result<()> {
        ensure!(obj._is_new(), "The obj is not new.");
        obj._data.validate()?;
        obj._set_default_value(conn);
        debug!("{}", &obj);
        conn.push_callback(Box::new(|| {
            async move {
                INSERT_DELAYED_QUEUE.push(obj.into());
                if let Some(addr) = DELAYED_ADDR.get() {
                    addr.do_send(DelayedMsg::InsertFromMemory);
                } else {
                    handle_delayed_msg_insert_from_memory().await;
                }
            }.boxed_local()
        })).await;
        Ok(())
    }

    // The data will be updated collectively later.
    // save_delayed does not support relational tables.
    pub async fn save_delayed(conn: &mut DbConn, mut obj: _@{ pascal_name }@ForUpdate) -> Result<()> {
        obj._data.validate()?;
        if obj._is_new() {
            obj._set_default_value(conn);
        }
        @%- if def.updated_at_conf().is_some() %@
        if obj._op.updated_at == Op::None {
            obj.updated_at().set(conn.time().into());
        }
        @%- endif %@
        if obj._will_be_deleted() {
            obj._op = OpData::default();
        }
@{- def.relations_one_owner()|fmt_rel_join("\n        obj.{alias} = None;", "") }@
@{- def.relations_many()|fmt_rel_join("\n        obj.{alias} = None;", "") }@
        let shard_id = conn.shard_id() as usize;
        conn.push_callback(Box::new(move || {
            async move {
                UPDATE_DELAYED_QUEUE.get().unwrap()[shard_id].push(obj);
                if let Some(addr) = DELAYED_ADDR.get() {
                    addr.do_send(DelayedMsg::Update);
                } else {
                    handle_delayed_msg_update().await;
                }
            }.boxed_local()
        })).await;
        Ok(())
    }

    // The data will be updated collectively later.
    // upsert_delayed does not support relational tables.
    pub async fn upsert_delayed(conn: &mut DbConn, mut obj: _@{ pascal_name }@ForUpdate) -> Result<()> {
        obj._data.validate()?;
        if obj._will_be_deleted() {
            panic!("Deletion is not supported.");
        }
        obj._set_default_value(conn);
@{- def.relations_one_owner()|fmt_rel_join("\n        obj.{alias} = None;", "") }@
@{- def.relations_many()|fmt_rel_join("\n        obj.{alias} = None;", "") }@
        let shard_id = conn.shard_id() as usize;
        conn.push_callback(Box::new(move || {
            async move {
                UPSERT_DELAYED_QUEUE.get().unwrap()[shard_id].push(obj);
                if let Some(addr) = DELAYED_ADDR.get() {
                    addr.do_send(DelayedMsg::Upsert);
                } else {
                    handle_delayed_msg_upsert().await;
                }
            }.boxed_local()
        })).await;
        Ok(())
    }

    pub async fn bulk_insert(conn: &mut DbConn, mut list: Vec<_@{ pascal_name }@ForUpdate>, ignore: bool) -> Result<()> {
        let mut vec: Vec<ForInsert> = Vec::new();
        while let Some(mut obj) = list.pop() {
            ensure!(obj._is_new(), "The obj is not new.");
            obj._data.validate()?;
            obj._set_default_value(conn);
            debug!("{}", &obj);
            vec.push(obj.into());
        }
        Self::_bulk_insert(conn, &vec, ignore).await
    }

    async fn _bulk_insert(conn: &mut DbConn, list: &[ForInsert], ignore: bool) -> Result<()> {
        let result = Self::__bulk_insert(conn, list, ignore).await?;
        for list in result {
            if !conn.clear_all_cache && (USE_CACHE || USE_CACHE_ALL) {
                let cache_msg = CacheOp::BulkInsert {
                    shard_id: conn.shard_id(),
                    list,
                };
                conn.push_cache_op(cache_msg.wrap()).await;
            }
        }
        Ok(())
    }

    pub(crate) async fn __bulk_insert(conn: &mut DbConn, list: &[ForInsert], ignore: bool) -> Result<Vec<Vec<ForInsert>>> {
        if list.is_empty() {
            return Ok(Vec::new());
        }
        let total_size: usize = list.iter().map(|v| v._data._size()).sum();
        let ave = total_size / list.len();
        let chunks = list.chunks(cmp::max(1, BULK_INSERT_MAX_SIZE.get().unwrap() / ave));
        let mut result = Vec::new();
        for chunk in chunks {
            result.push(Self::___bulk_insert(conn, chunk, ignore).await?);
        }
        Ok(result)
    }

    fn ___bulk_insert<'a>(conn: &'a mut DbConn, list: &'a [ForInsert], ignore: bool) -> future::LocalBoxFuture<'a, Result<Vec<ForInsert>>> {
        async move {
            if list.is_empty() {
                return Ok(Vec::new());
            }
            const SQL_NORMAL: &str = r#"INSERT "#; 
            const SQL_IGNORE: &str = r#"INSERT IGNORE "#; 
            const SQL1: &str = r#"INTO @{ table_name|db_esc }@ (@{ def.all_columns()|fmt_join("{col_esc}", ",") }@) VALUES "#;
            const SQL2: &str = r#"(@{ def.all_columns()|fmt_join("{placeholder}", ",") }@)"#;
            let mut sql = String::with_capacity(SQL_IGNORE.len() + SQL1.len() + (SQL2.len() + 1) * list.len());
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
            let mut query = sqlx::query(&sql);
            let _span = info_span!("query", sql = &query.sql());
            for data in list {
    @{- def.all_columns()|fmt_join("\n                query = query.bind(&data._data.{var});", "") }@
            }
            let result = if conn.wo_tx() {
                query.execute(&mut conn.acquire_source().await?).await?
            } else {
                query.execute(conn.get_tx().await?).await?
            };
            @{- def.auto_increments()|fmt_join("
            let mut id = result.last_insert_id() as u32;", "") }@
            let mut data_list = Vec::new();
            @{- def.relations_one_owner()|fmt_rel_join("
            let mut _{alias} = Vec::new();", "") }@
            @{- def.relations_many()|fmt_rel_join("
            let mut _{alias} = Vec::new();", "") }@
            for obj in list {
                let mut obj = obj.clone();
                @{- def.auto_increments()|fmt_join("
                if obj._data.{var} == 0 {
                    obj._data.{var} = id;
                    // innodb_autoinc_lock_mode must be 0 or 1
                    id += 1;
                }", "") }@
                @{- def.relations_one_owner()|fmt_rel_join("
                obj.{alias}.map(|v| v.map(|v| {
                    let mut v = v.as_ref().clone();
                    v._data.{foreign} = obj._data.{local_id}.clone();
                    _{alias}.push(v)
                }));", "") }@
                @{- def.relations_many()|fmt_rel_join_foreign_is_not_null_or_null("
                obj.{alias}.map(|v| v.into_iter().map(|mut v| {
                    v._data.{foreign} = obj._data.{local_id}.clone();
                    _{alias}.push(v)
                }));", "
                obj.{alias}.map(|v| v.into_iter().map(|mut v| {
                    v._data.{foreign} = Some(obj._data.{local_id}.clone());
                    _{alias}.push(v)
                }));", "") }@
                data_list.push(obj._data);
            }
            @{- def.relations_one_owner()|fmt_rel_join("
            let mut _{alias} = rel_{class_mod}::{class}::__bulk_insert(conn, &_{alias}, ignore).await?.into_iter().fold(FxHashMap::default(), |mut map, v| {
                for v in v {
                    map.insert(v._data.{foreign}, v);
                }
                map
            });", "") }@
            @{- def.relations_many()|fmt_rel_join_foreign_is_not_null_or_null("
            let mut _{alias} = rel_{class_mod}::{class}::__bulk_insert(conn, &_{alias}, ignore).await?.into_iter().fold(FxHashMap::default(), |mut map, v| {
                for v in v {
                    map.entry(v._data.{foreign})
                        .or_insert_with(Vec::new)
                        .push(v);
                }
                map
            });", "
            let mut _{alias} = rel_{class_mod}::{class}::__bulk_insert(conn, &_{alias}, ignore).await?.into_iter().fold(FxHashMap::default(), |mut map, v| {
                for v in v {
                    map.entry(v._data.{foreign}.unwrap())
                        .or_insert_with(Vec::new)
                        .push(v);
                }
                map
            });", "") }@
            let data_list = data_list.into_iter().map(|v| ForInsert {
                @{- def.relations_one_owner()|fmt_rel_join("
                {alias}: Some(_{alias}.remove(&v.{local_id}).map(Box::new)),", "") }@
                @{- def.relations_many()|fmt_rel_join("
                {alias}: _{alias}.remove(&v.{local_id}),", "") }@
                _data: v,
            }).collect();
            Ok(data_list)
        }.boxed_local()
    }

    pub async fn bulk_upsert(conn: &mut DbConn, mut list: Vec<_@{ pascal_name }@ForUpdate>) -> Result<()> {
        if list.is_empty() {
            return Ok(());
        }
        let mut vec = Vec::new();
        while let Some(mut obj) = list.pop() {
            ensure!(obj._is_new(), "The obj is not new.");
            obj._data.validate()?;
            obj._set_default_value(conn);
            debug!("{}", &obj);
            vec.push(obj._data);
        }
        let mut obj = list[0].clone();
        @%- if def.updated_at_conf().is_some() %@
        if obj._op.updated_at == Op::None {
            obj.updated_at().set(@{(def.updated_at_conf().unwrap() == Timestampable::RealTime)|if_then_else("SystemTime::now()","conn.time()")}@.into());
        }
        @%- endif %@
        Self::_bulk_upsert(conn, &vec, &obj).await
    }

    async fn _bulk_upsert(conn: &mut DbConn, list: &[Data], obj: &ForUpdate) -> Result<()> {
        let total_size: usize = list.iter().map(|v| v._size()).sum();
        let ave = total_size / list.len();
        let chunks = list.chunks(cmp::max(1, BULK_INSERT_MAX_SIZE.get().unwrap() / ave));
        for chunk in chunks {
            Self::__bulk_upsert(conn, chunk, obj).await?;
        }
        Ok(())
    }

    async fn __bulk_upsert(conn: &mut DbConn, list: &[Data], obj: &ForUpdate) -> Result<()> {
        if list.is_empty() {
            return Ok(());
        }
        const SQL1: &str = r#"INSERT INTO @{ table_name|db_esc }@ (@{ def.all_columns()|fmt_join("{col_esc}", ",") }@) VALUES "#;
        const SQL2: &str = r#"(@{ def.all_columns()|fmt_join("{placeholder}", ",") }@)"#;
        let mut sql = String::with_capacity(SQL1.len() + (SQL2.len() + 1) * list.len() + 100);
        sql.push_str(SQL1);
        sql.push_str(SQL2);
        for _i in 0..list.len() - 1 {
            sql.push(',');
            sql.push_str(SQL2);
        }
        let mut _update_cache = true;
        let mut vec: Vec<String> = Vec::new();
        @{- def.non_primaries()|fmt_join_cache_or_not("
        assignment_sql!(obj, vec, {var}, \"{col_esc}\", {may_null}, _update_cache, \"{placeholder}\");", "
        assignment_sql_no_cache_update!(obj, vec, {var}, \"{col_esc}\", {may_null}, \"{placeholder}\");", "") }@
        @%- if def.versioned %@
        vec.push("`@{ version_col }@` = IF(`@{ version_col }@` < 4294967295, `@{ version_col }@` + 1, 0)".to_string());
        @%- endif %@
        let _ = write!(sql, " ON DUPLICATE KEY UPDATE {}", &vec.join(","));
        let mut query = sqlx::query(&sql);
        let _span = info_span!("query", sql = &query.sql());
        for data in list {
@{- def.all_columns()|fmt_join("
            query = query.bind(&data.{var});", "") }@
        }
@{- def.non_primaries()|fmt_join("
        bind_sql!(obj, query, {var}, {may_null});","") }@
        if conn.wo_tx() {
            query.execute(&mut conn.acquire_source().await?).await?;
        } else {
            query.execute(conn.get_tx().await?).await?;
        }
        if !conn.clear_all_cache && (USE_CACHE || USE_CACHE_ALL) {
            let cache_msg = CacheOp::BulkUpsert {
                shard_id: conn.shard_id(),
                data_list: list.to_vec(),
                update: obj._update.clone(),
                op: obj._op.clone(),
            };
            conn.push_cache_op(cache_msg.wrap()).await;
        }
        Ok(())
    }
@%- if def.primaries().len() == 1 %@

    pub async fn delete_by_ids<I, T>(conn: &mut DbConn, ids: I) -> Result<u64>
    where
        I: IntoIterator<Item = T>,
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
@%- if def.soft_delete().is_some() %@
        let ids: Vec<Primary> = ids.into_iter().map(|id| (&id.into()).into()).collect();
        if ids.is_empty() {
            return Ok(0);
        }
        let mut rows_affected = 0u64;
        let id_chunks = ids.chunks(IN_CONDITION_LIMIT);
        @{- def.soft_delete_tpl2("","
        let deleted_at: {cond_type} = {val}.into();","","
        let deleted = cmp::max(1, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as u32);")}@
        @%- if def.updated_at_conf().is_some() %@
        @%- let updated_at = def.get_updated_at() %@
        let updated_at: @{ updated_at.get_cond_type() }@ = @{(def.updated_at_conf().unwrap() == Timestampable::RealTime)|if_then_else("SystemTime::now()","conn.time()")}@.into();
        @%- endif %@
        for ids in id_chunks {
            let q = "?,".repeat(ids.len());
            let sql = format!(
                r#"UPDATE @{ table_name|db_esc }@ SET @{ def.soft_delete_tpl2("","deleted_at=?","deleted=1","deleted=?")}@@% if def.updated_at_conf().is_some() %@, updated_at=?@%- endif %@ WHERE @{ def.inheritance_cond(" AND ") }@@{ def.primaries()|fmt_join("{col_esc}", ",") }@ in ({});"#,
                &q[0..q.len() - 1]
            );
            let mut query = sqlx::query(&sql);
            let _span = info_span!("query", sql = &query.sql());
@{- def.soft_delete_tpl2("","
            query = query.bind(&deleted_at);","","
            query = query.bind(&deleted);")}@
@%- if def.updated_at_conf().is_some() %@
            query = query.bind(&updated_at);
@%- endif %@
            for id in ids {
                query = query.bind(&id.0);
            }
            let result = if conn.wo_tx() {
                query.execute(&mut conn.acquire_source().await?).await?
            } else {
                query.execute(conn.get_tx().await?).await?
            };
            rows_affected += result.rows_affected();
        }
        debug!("DELETE @{ table_name }@ {}", vec_pri_to_str(&ids));
        if !conn.clear_all_cache && (USE_CACHE || USE_CACHE_ALL) {
            let mut for_update = ForUpdate {
                _data: Data::default(),
                _update: Default::default(),
                _is_new: false,
                _do_delete: false,
                _upsert: false,
                _is_loaded: false,
                _op: Default::default(),
@{- def.relations_one_owner()|fmt_rel_join("\n                {alias}: None,", "") }@
@{- def.relations_many()|fmt_rel_join("\n                {alias}: None,", "") }@
            };
            @{- def.soft_delete_tpl2("","
            for_update.deleted_at().set(Some(deleted_at));","
            for_update.deleted().set(true);","
            let deleted = cmp::max(1, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as u32);
            for_update.deleted().set(deleted);")}@
@%- if def.updated_at_conf().is_some() %@
            for_update.updated_at().set(updated_at);
@%- endif %@
            let cache_msg = CacheOp::UpdateMany {
                ids,
                shard_id: conn.shard_id(),
                update: for_update._update,
                op: for_update._op,
            };
            conn.push_cache_op(cache_msg.wrap()).await;
        }
        Ok(rows_affected)
@%- else %@
        Self::force_delete_by_ids(conn, ids).await
@%- endif %@
    }

    pub async fn force_delete_by_ids<I, T>(conn: &mut DbConn, ids: I) -> Result<u64>
    where
        I: IntoIterator<Item = T>,
        T: Into<@{ def.primaries()|fmt_join_with_paren("{outer_owned}", ", ") }@>,
    {
        let ids: Vec<Primary> = ids.into_iter().map(|id| (&id.into()).into()).collect();
        if ids.is_empty() {
            return Ok(0);
        }
        let mut rows_affected = 0u64;
        let id_chunks = ids.chunks(IN_CONDITION_LIMIT);
        for ids in id_chunks {
            let q = "?,".repeat(ids.len());
@%- if def.on_delete_fn %@
            let sql = format!(
                r#"SELECT {} FROM @{ table_name|db_esc }@ WHERE @{ def.inheritance_cond(" AND ") }@@{ def.primaries()|fmt_join("{col_esc}", ",") }@ in ({}) FOR UPDATE;"#,
                Data::_sql_cols("`"),
                &q[0..q.len() - 1]
            );
            let mut query = sqlx::query_as::<_, Data>(&sql);
            let _span = info_span!("query", sql = &query.sql());
            for id in ids {
                query = query.bind(&id.0);
            }
            let result = query.fetch_all(conn.get_tx().await?).await?;
            let list: Vec<Self> = result.into_iter().map(|v| v.into()).collect();
            Self::_before_delete(conn, &list).await?;
            conn.push_callback(Box::new(|| {
                async move {
                    Self::_after_delete(&list).await;
                }.boxed_local()
            })).await;
@%- endif %@
@%- for on_delete_str in def.on_delete_list %@
            crate::@{ on_delete_str }@::on_delete_@{ group_name }@_@{ name }@(conn, ids, false).await?;
@%- endfor %@
            let sql = format!(
                r#"DELETE FROM @{ table_name|db_esc }@ WHERE @{ def.primaries()|fmt_join("{col_esc}", ",") }@ in ({});"#,
                &q[0..q.len() - 1]
            );
            let mut query = sqlx::query(&sql);
            let _span = info_span!("query", sql = &query.sql());
            for id in ids {
                query = query.bind(&id.0);
            }
            let result = query.execute(conn.get_tx().await?).await?;
            rows_affected += result.rows_affected();
        }
        debug!("FORCE DELETE @{ table_name }@ {}", vec_pri_to_str(&ids));
        if !conn.clear_all_cache && (USE_CACHE || USE_CACHE_ALL) {
            let shard_id = conn.shard_id();
            conn.push_cache_op(CacheOp::DeleteMany { ids, shard_id }.wrap()).await;
        }
        Ok(rows_affected)
    }
@% endif %@
    #[allow(unused_mut)]
    pub async fn delete(conn: &mut DbConn, mut obj: _@{ pascal_name }@ForUpdate) -> Result<()> {
        @%- if def.updated_at_conf().is_some() %@
        if obj._op.updated_at == Op::None {
            obj.updated_at().set(@{(def.updated_at_conf().unwrap() == Timestampable::RealTime)|if_then_else("SystemTime::now()","conn.time()")}@.into());
        }
        @%- endif %@
        let cache_msg = Self::_delete(conn, obj).await?;
        if !conn.clear_all_cache && (USE_CACHE || USE_CACHE_ALL) {
            if let Some(cache_msg) = cache_msg {
                conn.push_cache_op(cache_msg.wrap()).await;
            }
        }
        Ok(())
    }

    #[allow(unused_mut)]
    async fn _delete(conn: &mut DbConn, mut obj: _@{ pascal_name }@ForUpdate) -> Result<Option<CacheOp>> {
        if obj._has_been_deleted() {
            return Ok(None);
        }
@{- def.soft_delete_tpl2("
        Self::force_delete(conn, obj).await?;
        Ok(None)","
        obj.deleted_at().set(Some({val}.into()));
        let (_obj, cache_msg) = Self::_save_update(conn, obj).await?;
        Ok(cache_msg)","
        obj.deleted().set(true);
        let (_obj, cache_msg) = Self::_save_update(conn, obj).await?;
        Ok(cache_msg)","
        let deleted = cmp::max(1, SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs() as u32);
        obj.deleted().set(deleted);
        let (_obj, cache_msg) = Self::_save_update(conn, obj).await?;
        Ok(cache_msg)")}@
    }
@%- if def.soft_delete().is_some() %@

    #[allow(unused_mut)]
    pub async fn restore(conn: &mut DbConn, mut obj: _@{ pascal_name }@ForUpdate) -> Result<_@{ pascal_name }@> {
        obj._do_delete = false;
        if !obj._has_been_deleted() {
            return Ok(obj.into());
        }
        @%- if def.updated_at_conf().is_some() %@
        if obj._op.updated_at == Op::None {
            obj.updated_at().set(@{(def.updated_at_conf().unwrap() == Timestampable::RealTime)|if_then_else("SystemTime::now()","conn.time()")}@.into());
        }
        @%- endif %@
@{- def.soft_delete_tpl2("","
        obj.deleted_at().set(None);","
        obj.deleted().set(false);","
        obj.deleted().set(0);")}@
        let (obj, cache_msg) = Self::_save_update(conn, obj).await?;
        if !conn.clear_all_cache && (USE_CACHE || USE_CACHE_ALL) {
            if let Some(cache_msg) = cache_msg {
                conn.push_cache_op(cache_msg.wrap()).await;
            }
        }
        Ok(obj)
    }
@%- endif %@

    pub async fn force_delete(conn: &mut DbConn, obj: _@{ pascal_name }@ForUpdate) -> Result<()> {
        let id: Primary = (&obj).into();
@%- if def.on_delete_fn %@
        let notify_obj: Self = if obj._is_loaded {
            Self::from(obj.clone())
        } else {
            Self::find_for_update(conn, &id).await?.into()
        };
        Self::_before_delete(conn, &[notify_obj.clone()]).await?;
@%- endif %@
@%- for on_delete_str in def.on_delete_list %@
        crate::@{ on_delete_str }@::on_delete_@{ group_name }@_@{ name }@(conn, &[id.clone()], false).await?;
@%- endfor %@
        let mut query = sqlx::query(r#"DELETE FROM @{ table_name|db_esc }@ WHERE @{ def.primaries()|fmt_join("{col_esc}=?", " AND ") }@"#);
        let _span = info_span!("query", sql = &query.sql());
        @{- def.primaries()|fmt_join("
        query = query.bind(id.{index});", "") }@
        query.execute(conn.get_tx().await?).await?;
        debug!("FORCE DELETE @{ table_name }@ {}", id);
@%- if def.on_delete_fn %@
        conn.push_callback(Box::new(|| {
            async {
                Self::_after_delete(&[notify_obj]).await;
            }.boxed_local()
        })).await;
@%- endif %@
        if !conn.clear_all_cache && (USE_CACHE || USE_CACHE_ALL) {
            let shard_id = conn.shard_id();
            conn.push_cache_op(CacheOp::Delete { id, shard_id }.wrap()).await;
        }
        Ok(())
    }

    pub async fn force_delete_relations(conn: &mut DbConn, obj: _@{ pascal_name }@ForUpdate) -> Result<()> {
        let id: Primary = (&obj).into();
@%- for on_delete_str in def.on_delete_list %@
        crate::@{ on_delete_str }@::on_delete_@{ group_name }@_@{ name }@(conn, &[id.clone()], true).await?;
@%- endfor %@
        Ok(())
    }

    pub async fn force_delete_all(conn: &mut DbConn) -> Result<()> {
        let query = sqlx::query(r#"DELETE FROM @{ table_name|db_esc }@"#);
        let _span = info_span!("query", sql = &query.sql());
        if conn.wo_tx() {
            query.execute(&mut conn.acquire_source().await?).await?;
        } else {
            query.execute(conn.get_tx().await?).await?;
        }
        if !conn.clear_all_cache && (USE_CACHE || USE_CACHE_ALL) {
            conn.push_cache_op(CacheOp::DeleteAll.wrap()).await;
        }
        Ok(())
    }
@%- for (base_mod_name,) in def.relations_on_delete_mod() %@

    pub(crate) fn on_delete_@{ base_mod_name }@<'a>(
        conn: &'a mut DbConn,
        ids: &'a [rel_@{ base_mod_name }@::Primary],
        cascade_only: bool,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + 'a>> {
        Box::pin(async move {
            @%- for (mod_name, local) in def.relations_on_delete_cascade() %@
            @%- if base_mod_name == mod_name %@
            Self::on_delete_@{ mod_name }@_for_@{ local }@(conn, ids, cascade_only).await?;
            @%- endif %@
            @%- endfor %@
            @%- for (mod_name, local) in def.relations_on_delete_restrict() %@
            @%- if base_mod_name == mod_name %@
            Self::on_delete_@{ mod_name }@_for_@{ local }@(conn, ids, cascade_only).await?;
            @%- endif %@
            @%- endfor %@
            @%- for (mod_name, local, val, val2) in def.relations_on_delete_not_cascade() %@
            @%- if base_mod_name == mod_name %@
            Self::on_delete_@{ mod_name }@_for_@{ local }@(conn, ids, cascade_only).await?;
            @%- endif %@
            @%- endfor %@
            Ok(())
        })
    }
@%- endfor %@
@%- for (mod_name, local) in def.relations_on_delete_cascade() %@

    async fn on_delete_@{ mod_name }@_for_@{ local }@(
        conn: &mut DbConn,
        ids: &[rel_@{ mod_name }@::Primary],
        cascade_only: bool,
    ) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }
        let id_chunks = ids.chunks(IN_CONDITION_LIMIT);
        for ids in id_chunks {
            let q = "?,".repeat(ids.len());
@%- if def.on_delete_fn %@
            let sql = format!(
                r#"SELECT {} FROM @{ table_name|db_esc }@ WHERE @{ def.inheritance_cond(" AND ") }@@{ local|db_esc }@ in ({}) FOR UPDATE;"#,
                Data::_sql_cols("`"),
                &q[0..q.len() - 1]
            );
            let mut query = sqlx::query_as::<_, Data>(&sql);
            let _span = info_span!("query", sql = &query.sql());
            for id in ids {
                query = query.bind(&id.0);
            }
            let result = query.fetch_all(conn.get_tx().await?).await?;
            let result_num = result.len() as u64;
            let list: Vec<Self> = result.into_iter().map(|v| v.into()).collect();
            let id_list: Vec<Primary> = list.iter().map(|v| v.into()).collect();
            Self::_before_delete(conn, &list).await?;
            conn.push_callback(Box::new(|| {
                async move {
                    Self::_after_delete(&list).await;
                }.boxed_local()
            })).await;
            if !conn.clear_all_cache && (USE_CACHE || USE_CACHE_ALL) {
                let cache_msg = CacheOp::Cascade { ids: id_list.clone(), shard_id: conn.shard_id() };
                conn.push_cache_op(cache_msg.wrap()).await;
            }
@%- for on_delete_str in def.on_delete_list %@
            crate::@{ on_delete_str }@::on_delete_@{ group_name }@_@{ name }@(conn, &id_list, cascade_only).await?;
@%- endfor %@
@%- else %@
            let sql = format!(
                r#"SELECT @{ def.primaries()|fmt_join("{col_esc}", "") }@ FROM @{ table_name|db_esc }@ WHERE @{ def.inheritance_cond(" AND ") }@@{ local|db_esc }@ in ({});"#,
                &q[0..q.len() - 1]
            );
            let mut query = sqlx::query_as::<_, Primary>(&sql);
            let _span = info_span!("query", sql = &query.sql());
            for id in ids {
                query = query.bind(&id.0);
            }
            let id_list = query.fetch_all(conn.get_tx().await?).await?;
            let result_num = id_list.len() as u64;
            if !conn.clear_all_cache && (USE_CACHE || USE_CACHE_ALL) {
                let cache_msg = CacheOp::Cascade { ids: id_list.clone(), shard_id: conn.shard_id() };
                conn.push_cache_op(cache_msg.wrap()).await;
            }
@%- for on_delete_str in def.on_delete_list %@
            crate::@{ on_delete_str }@::on_delete_@{ group_name }@_@{ name }@(conn, &id_list, cascade_only).await?;
@%- endfor %@
@%- endif %@
            let sql = format!(
                r#"DELETE FROM @{ table_name|db_esc }@ WHERE @{ local|db_esc }@ in ({});"#,
                &q[0..q.len() - 1]
            );
            let mut query = sqlx::query(&sql);
            let _span = info_span!("query", sql = &query.sql());
            for id in ids {
                query = query.bind(&id.0);
            }
            let result = query.execute(conn.get_tx().await?).await?;
            ensure!(
                result_num == result.rows_affected(),
                "Mismatch occurred when deleting @{ table_name }@."
            );
        }
        Ok(())
    }
@%- endfor %@
@%- for (mod_name, local) in def.relations_on_delete_restrict() %@

    async fn on_delete_@{ mod_name }@_for_@{ local }@(
        conn: &mut DbConn,
        ids: &[rel_@{ mod_name }@::Primary],
        cascade_only: bool,
    ) -> Result<()> {
        if ids.is_empty() || cascade_only {
            return Ok(());
        }
        let id_chunks = ids.chunks(IN_CONDITION_LIMIT);
        for ids in id_chunks {
            let q = "?,".repeat(ids.len());
            let sql = format!(
                r#"SELECT count(*) as c FROM @{ table_name|db_esc }@ WHERE @{ def.inheritance_cond(" AND ") }@@{ local|db_esc }@ in ({});"#,
                &q[0..q.len() - 1]
            );
            let mut query = sqlx::query_as::<_, Count>(&sql);
            let _span = info_span!("query", sql = &query.sql());
            for id in ids {
                query = query.bind(&id.0);
            }
            let result = query.fetch_one(conn.get_tx().await?).await?;
            ensure!(
                result.c == 0,
                "Cannot delete or update a parent row: a foreign key constraint fails on @{ table_name }@."
            );
        }
        Ok(())
    }
@%- endfor %@
@%- for (mod_name, local, val, val2) in def.relations_on_delete_not_cascade() %@

    async fn on_delete_@{ mod_name }@_for_@{ local }@(
        conn: &mut DbConn,
        ids: &[rel_@{ mod_name }@::Primary],
        cascade_only: bool,
    ) -> Result<()> {
        if ids.is_empty() || cascade_only {
            return Ok(());
        }
        let id_chunks = ids.chunks(IN_CONDITION_LIMIT);
        for ids in id_chunks {
            let q = "?,".repeat(ids.len());
            let sql = format!(
                r#"SELECT @{ def.primaries()|fmt_join("{col_esc}", "") }@ FROM @{ table_name|db_esc }@ WHERE @{ def.inheritance_cond(" AND ") }@@{ local|db_esc }@ in ({});"#,
                &q[0..q.len() - 1]
            );
            let mut query = sqlx::query_as::<_, Primary>(&sql);
            let _span = info_span!("query", sql = &query.sql());
            for id in ids {
                query = query.bind(&id.0);
            }
            let id_list = query.fetch_all(conn.get_tx().await?).await?;
            let result_num = id_list.len() as u64;
            if !conn.clear_all_cache && (USE_CACHE || USE_CACHE_ALL) {
                let cache_msg = CacheOp::Reset@{ local|pascal }@@{ val|pascal }@ { ids: id_list.clone(), shard_id: conn.shard_id() };
                conn.push_cache_op(cache_msg.wrap()).await;
            }
            let sql = format!(
                r#"UPDATE @{ table_name|db_esc }@ SET @{ local|db_esc }@ = @{ val }@ WHERE @{ def.inheritance_cond(" AND ") }@@{ local|db_esc }@ in ({});"#,
                &q[0..q.len() - 1]
            );
            let mut query = sqlx::query(&sql);
            let _span = info_span!("query", sql = &query.sql());
            for id in ids {
                query = query.bind(&id.0);
            }
            let result = query.execute(conn.get_tx().await?).await?;
            ensure!(
                result_num == result.rows_affected(),
                "Mismatch occurred when set @{ local|db_esc }@ = @{ val }@ @{ table_name }@."
            );
        }
        Ok(())
    }
@%- endfor %@

    pub fn eq(base: &_@{ pascal_name }@ForUpdate, update: &_@{ pascal_name }@ForUpdate) -> bool {
        true
        @{- def.for_cmp()|fmt_join("
        && base._data.{var} == update._data.{var}", "") }@
    }

    pub fn set(base: &mut _@{ pascal_name }@ForUpdate, update: _@{ pascal_name }@ForUpdate) {
        @{- def.for_cmp()|fmt_join("
        base._op.{var} = Op::Set;
        base._data.{var} = update._data.{var}.clone();
        base._update.{var} = update._data.{var};", "") }@
    }
}

fn query_bind<'a>(sql: &'a str, data: &'a Data) -> Query<'a, DbType, DbArguments> {
    let mut query = sqlx::query(sql);
    @{- def.all_columns()|fmt_join("
    query = query.bind(&data.{var});", "") }@
    query
}

impl _@{ pascal_name }@Factory {
    #[allow(clippy::needless_update)]
    pub fn create(self, conn: &DbConn) -> _@{ pascal_name }@ForUpdate {
        _@{ pascal_name }@ForUpdate {
            _data: Data {
@{ def.for_factory()|fmt_join("                {var}: self.{var}{convert_factory},", "\n") }@
                ..Data::default()
            },
            _update: Default::default(),
            _is_new: true,
            _do_delete: false,
            _upsert: false,
            _is_loaded: true,
            _op: Default::default(),
@{- def.relations_one_owner()|fmt_rel_join("\n            {alias}: Some(None),", "") }@
@{- def.relations_many()|fmt_rel_join("\n            {alias}: Some(Vec::new()),", "") }@
        }
    }
}

impl From<_@{ pascal_name }@ForUpdate> for _@{ pascal_name }@ {
    fn from(from: _@{ pascal_name }@ForUpdate) -> Self {
        let mut to: _@{ pascal_name }@ = from._data.into();
@{- def.relations_one_owner()|fmt_rel_join("
        to.{alias} = from.{alias}.map(|v| v.map(|v| v.into()));", "") }@
@{- def.relations_many()|fmt_rel_join("
        to.{alias} = from.{alias}.map(|v| v.into_iter().map(|v| v.into()).collect());", "") }@
        to
    }
}
impl From<Box<_@{ pascal_name }@ForUpdate>> for Box<_@{ pascal_name }@> {
    fn from(from: Box<_@{ pascal_name }@ForUpdate>) -> Self {
        let mut to: _@{ pascal_name }@ = from._data.into();
@{- def.relations_one_owner()|fmt_rel_join("
        to.{alias} = from.{alias}.map(|v| v.map(|v| v.into()));", "") }@
@{- def.relations_many()|fmt_rel_join("
        to.{alias} = from.{alias}.map(|v| v.into_iter().map(|v| v.into()).collect());", "") }@
        Box::new(to)
    }
}

pub(crate) async fn _seed(seed: &serde_yaml::Value, conns: &mut [DbConn]) -> Result<()> {
    if let Some(mapping) = seed.as_mapping() {
        for (name, factory) in mapping {
            let seed: _@{ pascal_name }@Factory = serde_yaml::from_str(&serde_yaml::to_string(&factory)?)?;
            let shard_id = seed._shard_id().await as usize;
            let conn = &mut conns[shard_id];
            let obj = seed.create(conn);
            if let Some(obj) = _@{ pascal_name }@::save(conn, obj).await? {
                @{- def.auto_increments()|fmt_join("
                let id = obj.{var}();
                if GENERATED_IDS.get().is_none() {
                    let _ = GENERATED_IDS.set(RwLock::new(HashMap::new()));
                }
                let name = name.as_str().unwrap().to_string();
                GENERATED_IDS.get().unwrap().write().unwrap().insert(name, id);", "") }@
            }
        }
    }
    Ok(())
}
@{- def.relations_one_owner()|fmt_rel_join("

async fn save_{alias}(
    conn: &mut DbConn,
    obj: &mut _{pascal_name},
    data: Option<Option<Box<rel_{class_mod}::{class}ForUpdate>>>,
    update_cache: &mut bool,
) -> Result<Option<Option<rel_{class_mod}::CacheOp>>> {
    if let Some(row) = data {
        if let Some(mut row) = row {
            row._data.{foreign} = obj._inner.{local_id}.clone();
            let (obj2, msg) = rel_{class_mod}::{class}::__save(conn, *row).await?;
            if obj2.is_some() {
                obj.{alias} = Some(obj2.map(Box::new));
            }
            *update_cache = *update_cache || msg.is_some();
            return Ok(Some(msg));
        }
        return Ok(Some(None));
    }
    Ok(None)
}", "") }@
@{- def.relations_many()|fmt_rel_join_foreign_is_not_null_or_null("

async fn save_{alias}(
    conn: &mut DbConn,
    obj: &mut _{pascal_name},
    data: Option<Vec<rel_{class_mod}::{class}ForUpdate>>,
    update_cache: &mut bool,
) -> Result<Option<Vec<rel_{class_mod}::CacheOp>>> {
    if let Some(list) = data {
        let mut msgs = Vec::new();
        let src = _save_{alias}(conn, obj._inner.{local_id}.clone().into(), list);
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
        obj.{alias} = Some(vec);
        return Ok(Some(msgs));
    }
    Ok(None)
}
fn _save_{alias}(
    conn: &mut DbConn,
    id: {id_name},
    list: Vec<rel_{class_mod}::{class}ForUpdate>,
) -> impl futures::Stream<
    Item = Result<(
        Option<rel_{class_mod}::{class}>,
        Option<rel_{class_mod}::CacheOp>,
    )>,
> + '_ {
    async_stream::try_stream! {
        let mut update_list = Vec::new();
        let mut insert_list = Vec::new();
        for row in list.into_iter() {
            if row._is_new() {
                insert_list.push(row);
            }else if row._will_be_deleted() {
                yield rel_{class_mod}::{class}::__save(conn, row).await?;
            } else{
                update_list.push(row);
            }
        }
        for row in update_list.into_iter() {
            yield rel_{class_mod}::{class}::__save(conn, row).await?;
        }
        for mut row in insert_list.into_iter() {
            row._data.{foreign} = id.clone().into();
            yield rel_{class_mod}::{class}::__save(conn, row).await?;
        }
    }
}", "

async fn save_{alias}(
    conn: &mut DbConn,
    obj: &mut _{pascal_name},
    data: Option<Vec<rel_{class_mod}::{class}ForUpdate>>,
    update_cache: &mut bool,
) -> Result<Option<Vec<rel_{class_mod}::CacheOp>>> {
    if let Some(list) = data {
        let mut msgs = Vec::new();
        let src = _save_{alias}(conn, obj._inner.{local_id}.clone().into(), list);
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
        obj.{alias} = Some(vec);
        return Ok(Some(msgs));
    }
    Ok(None)
}
fn _save_{alias}(
    conn: &mut DbConn,
    id: {id_name},
    list: Vec<rel_{class_mod}::{class}ForUpdate>,
) -> impl futures::Stream<
    Item = Result<(
        Option<rel_{class_mod}::{class}>,
        Option<rel_{class_mod}::CacheOp>,
    )>,
> + '_ {
    async_stream::try_stream! {
        let mut update_list = Vec::new();
        let mut insert_list = Vec::new();
        for row in list.into_iter() {
            if row._is_new() {
                insert_list.push(row);
            }else if row._will_be_deleted() {
                yield rel_{class_mod}::{class}::__save(conn, row).await?;
            } else{
                update_list.push(row);
            }
        }
        for row in update_list.into_iter() {
            yield rel_{class_mod}::{class}::__save(conn, row).await?;
        }
        for mut row in insert_list.into_iter() {
            row._data.{foreign} = Some(id.clone().into());
            yield rel_{class_mod}::{class}::__save(conn, row).await?;
        }
    }
}", "") }@
@{-"\n"}@