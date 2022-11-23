// This code is auto-generated and will always be overwritten.

use anyhow::Result;
use futures::future::join_all;
use fxhash::FxHashMap;
use once_cell::sync::OnceCell;
use senax_common::{
    cache::db_cache::{CacheVal, DbCache, HashVal},
    ShardId,
};
use std::{path::Path, sync::Arc};

use crate::DB_UPPER_NAME;

static CACHE: OnceCell<DbCache> = OnceCell::new();
pub struct Cache;
impl Cache {
    pub fn start(
        is_hot_deploy: bool,
        path: Option<&Path>,
        use_fast_cache: bool,
        use_disk_cache: bool,
    ) -> Result<()> {
        if CACHE.get().is_none() {
            let _ = CACHE.set(DbCache::start(
                DB_UPPER_NAME,
                is_hot_deploy,
                path,
                use_fast_cache,
                use_disk_cache,
            )?);
        }
        Ok(())
    }

    pub fn stop() {
        if let Some(c) = CACHE.get() {
            c.stop()
        }
    }

    pub async fn insert_short(id: &dyn HashVal, value: Arc<dyn CacheVal>) {
        if let Some(cache) = CACHE.get() {
            cache.insert_short(id, value).await;
        }
    }

    pub async fn insert_version(id: &dyn HashVal, value: Arc<dyn CacheVal>) {
        if let Some(cache) = CACHE.get() {
            cache.insert_version(id, value).await;
        }
    }

    pub async fn insert_long(id: &dyn HashVal, value: Arc<dyn CacheVal>, use_fast_cache: bool) {
        if let Some(cache) = CACHE.get() {
            cache.insert_long(id, value, use_fast_cache).await;
        }
    }

    pub async fn get<T>(id: &dyn HashVal, shard_id: ShardId, use_fast_cache: bool) -> Option<Arc<T>>
    where
        T: CacheVal,
    {
        if let Some(cache) = CACHE.get() {
            cache
                .get::<T>(id.hash_val(shard_id), shard_id, use_fast_cache, false)
                .await
        } else {
            None
        }
    }

    pub async fn get_from_memory<T>(
        id: &dyn HashVal,
        shard_id: ShardId,
        use_fast_cache: bool,
    ) -> Option<Arc<T>>
    where
        T: CacheVal,
    {
        if let Some(cache) = CACHE.get() {
            cache
                .get::<T>(id.hash_val(shard_id), shard_id, use_fast_cache, true)
                .await
        } else {
            None
        }
    }

    #[allow(clippy::ptr_arg)]
    pub async fn get_many<T>(
        hashes: &Vec<u128>,
        shard_id: ShardId,
        use_fast_cache: bool,
    ) -> FxHashMap<u128, Arc<T>>
    where
        T: CacheVal,
    {
        let mut map = FxHashMap::default();
        if let Some(cache) = CACHE.get() {
            let v = join_all(
                hashes
                    .iter()
                    .map(|hash| cache.get::<T>(*hash, shard_id, use_fast_cache, false)),
            )
            .await;
            for (idx, val) in v.into_iter().enumerate() {
                if let Some(val) = val {
                    map.insert(*hashes.get(idx).unwrap(), val);
                }
            }
        }
        map
    }

    pub async fn get_version<T>(id: &dyn HashVal, shard_id: ShardId) -> Option<Arc<T>>
    where
        T: CacheVal,
    {
        if let Some(cache) = CACHE.get() {
            cache
                .get_version::<T>(id.hash_val(shard_id), shard_id)
                .await
        } else {
            None
        }
    }

    pub async fn invalidate(id: &dyn HashVal, shard_id: ShardId) {
        if let Some(cache) = CACHE.get() {
            cache.invalidate(id, shard_id).await;
        }
    }

    pub fn invalidate_all_of<T>()
    where
        T: CacheVal,
    {
        if let Some(c) = CACHE.get() {
            c.invalidate_all_of::<T>()
        }
    }

    pub fn invalidate_all_of_version<T>()
    where
        T: CacheVal,
    {
        if let Some(c) = CACHE.get() {
            c.invalidate_all_of_version::<T>()
        }
    }

    pub fn invalidate_all() {
        if let Some(c) = CACHE.get() {
            c.invalidate_all()
        }
    }

    pub fn long_cache_hit() -> Option<u64> {
        CACHE.get().map(|c| c.long_cache_hit())
    }
    pub fn short_cache_hit() -> Option<u64> {
        CACHE.get().map(|c| c.short_cache_hit())
    }
    pub fn disk_cache_hit() -> Option<u64> {
        CACHE.get().map(|c| c.disk_cache_hit())
    }
    pub fn cache_request_count() -> Option<u64> {
        CACHE.get().map(|c| c.cache_request_count())
    }
    pub fn long_cache_evicted() -> Option<u64> {
        CACHE.get().map(|c| c.long_cache_evicted())
    }
    pub fn short_cache_evicted() -> Option<u64> {
        CACHE.get().map(|c| c.short_cache_evicted())
    }
}
@{-"\n"}@