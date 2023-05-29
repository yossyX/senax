use anyhow::Result;
use byte_unit::Byte;
use downcast_rs::{impl_downcast, DowncastSync};
use fxhash::FxBuildHasher;
use log::error;
use moka::{future::Cache, notification::RemovalCause};
use std::{
    fs,
    path::Path,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use crate::ShardId;
use crate::{cache::disk_cache::DiskCache, cache::fast_cache::FastCache};

use super::msec::{get_cache_time, MSec, MSEC_SHR};

const MOKA_BASE_MEMORY: u32 = 200;
const DEFAULT_FAST_CACHE_INDEX_SIZE: &str = "1MiB";
const DEFAULT_SHORT_CACHE_CAPACITY: &str = "1MiB";
const DEFAULT_SHORT_CACHE_TIME: &str = "60";
const DEFAULT_LONG_CACHE_CAPACITY: &str = "10MiB";
const DEFAULT_LONG_CACHE_TIME: &str = "3600";
const DEFAULT_LONG_CACHE_IDLE_TIME: &str = "600";
const DEFAULT_DISK_CACHE_INDEX_SIZE: &str = "8MiB";
const DEFAULT_DISK_CACHE_FILE_NUM: &str = "1";
const DEFAULT_DISK_CACHE_FILE_SIZE: &str = "100MiB";
const DEFAULT_CACHE_TTL: &str = "3600";
const DISK_CACHE_FILE_NAME: &str = "cache-%Y%m%d%H%M%S";

pub trait CacheVal: DowncastSync + std::fmt::Debug {
    fn _size(&self) -> u32;
    fn _type_id(&self) -> u64;
    fn __type_id() -> u64
    where
        Self: Sized;
    fn _shard_id(&self) -> ShardId;
    fn _time(&self) -> MSec;
    fn _estimate() -> usize
    where
        Self: Sized;
    fn _encode(&self) -> Result<Vec<u8>>;
    fn _decode(v: &[u8]) -> Result<Self>
    where
        Self: Sized;
}
impl_downcast!(sync CacheVal);

pub trait HashVal: Send + Sync {
    fn hash_val(&self, shard_id: ShardId) -> u128;
}

fn get_fast_cache(name: &str, time_to_live: u64) -> FastCache {
    let index_size = Byte::from_str(
        std::env::var(format!("{}_FAST_CACHE_INDEX_SIZE", name))
            .unwrap_or_else(|_| DEFAULT_FAST_CACHE_INDEX_SIZE.to_owned()),
    )
    .unwrap_or_else(|e| panic!("{}_FAST_CACHE_INDEX_SIZE has an error:{:?}", name, e))
    .get_bytes();
    FastCache::new(index_size, time_to_live)
}

fn get_short_cache(
    name: &str,
    short_cache_evicted: Arc<AtomicU64>,
) -> Cache<u128, Arc<dyn CacheVal>, FxBuildHasher> {
    let capacity = Byte::from_str(
        std::env::var(format!("{}_SHORT_CACHE_CAPACITY", name))
            .unwrap_or_else(|_| DEFAULT_SHORT_CACHE_CAPACITY.to_owned()),
    )
    .unwrap_or_else(|e| panic!("{}_SHORT_CACHE_CAPACITY has an error:{:?}", name, e))
    .get_bytes();

    let time_to_live = std::env::var(format!("{}_SHORT_CACHE_TIME", name))
        .unwrap_or_else(|_| DEFAULT_SHORT_CACHE_TIME.to_owned())
        .parse::<u64>()
        .unwrap_or_else(|e| panic!("{}_SHORT_CACHE_TIME has an error:{:?}", name, e));

    Cache::builder()
        .weigher(|_key, value: &Arc<dyn CacheVal>| -> u32 {
            value._size().saturating_add(MOKA_BASE_MEMORY)
        })
        .max_capacity(capacity)
        .time_to_live(std::time::Duration::from_secs(time_to_live))
        .support_invalidation_closures()
        .eviction_listener_with_queued_delivery_mode(move |_k, _v, cause| {
            if cause == RemovalCause::Size {
                short_cache_evicted.fetch_add(1, Ordering::Relaxed);
            }
        })
        .build_with_hasher(FxBuildHasher::default())
}

fn get_long_cache(
    name: &str,
    long_cache_evicted: Arc<AtomicU64>,
    disk_cache: Option<Arc<DiskCache>>,
) -> Cache<u128, Arc<dyn CacheVal>, FxBuildHasher> {
    let capacity = Byte::from_str(
        std::env::var(format!("{}_LONG_CACHE_CAPACITY", name))
            .unwrap_or_else(|_| DEFAULT_LONG_CACHE_CAPACITY.to_owned()),
    )
    .unwrap_or_else(|e| panic!("{}_LONG_CACHE_CAPACITY has an error:{:?}", name, e))
    .get_bytes();

    let time_to_live = std::env::var(format!("{}_LONG_CACHE_TIME", name))
        .unwrap_or_else(|_| DEFAULT_LONG_CACHE_TIME.to_owned())
        .parse::<u64>()
        .unwrap_or_else(|e| panic!("{}_LONG_CACHE_TIME has an error:{:?}", name, e));

    let time_to_idle = std::env::var(format!("{}_LONG_CACHE_IDLE_TIME", name))
        .unwrap_or_else(|_| DEFAULT_LONG_CACHE_IDLE_TIME.to_owned())
        .parse::<u64>()
        .unwrap_or_else(|e| panic!("{}_LONG_CACHE_IDLE_TIME has an error:{:?}", name, e));

    Cache::builder()
        .weigher(|_key, value: &Arc<dyn CacheVal>| -> u32 {
            value._size().saturating_add(MOKA_BASE_MEMORY)
        })
        .max_capacity(capacity)
        .time_to_live(std::time::Duration::from_secs(time_to_live))
        .time_to_idle(std::time::Duration::from_secs(time_to_idle))
        .support_invalidation_closures()
        .eviction_listener_with_queued_delivery_mode(move |k, v, cause| {
            if cause == RemovalCause::Size {
                long_cache_evicted.fetch_add(1, Ordering::Relaxed);
            }
            if cause.was_evicted() {
                if let Some(ref disk_cache) = disk_cache {
                    if let Ok(buf) = v._encode() {
                        disk_cache.write(*k, v._type_id(), &buf, v._time());
                    }
                }
            }
        })
        .build_with_hasher(FxBuildHasher::default())
}

fn get_disk_cache(
    name: &str,
    is_hot_deploy: bool,
    path: &Path,
    time_to_live: u64,
) -> Result<DiskCache> {
    let index_size = Byte::from_str(
        std::env::var(format!("{}_DISK_CACHE_INDEX_SIZE", name))
            .unwrap_or_else(|_| DEFAULT_DISK_CACHE_INDEX_SIZE.to_owned()),
    )
    .unwrap_or_else(|e| panic!("{}_DISK_CACHE_INDEX_SIZE has an error:{:?}", name, e))
    .get_bytes();

    let file_num = std::env::var(format!("{}_DISK_CACHE_FILE_NUM", name))
        .unwrap_or_else(|_| DEFAULT_DISK_CACHE_FILE_NUM.to_owned())
        .parse::<usize>()
        .unwrap_or_else(|e| panic!("{}_DISK_CACHE_FILE_NUM has an error:{:?}", name, e));

    let file_size = Byte::from_str(
        std::env::var(format!("{}_DISK_CACHE_FILE_SIZE", name))
            .unwrap_or_else(|_| DEFAULT_DISK_CACHE_FILE_SIZE.to_owned()),
    )
    .unwrap_or_else(|e| panic!("{}_DISK_CACHE_FILE_SIZE has an error:{:?}", name, e))
    .get_bytes();

    if !is_hot_deploy && path.is_dir() {
        for entry in path.read_dir()? {
            let entry = entry?;
            if entry.metadata()?.is_file() {
                fs::remove_file(entry.path())?;
            }
        }
    }
    fs::create_dir_all(path)?;
    let path = path.join(
        chrono::Local::now()
            .format(DISK_CACHE_FILE_NAME)
            .to_string(),
    );
    DiskCache::start(path, index_size, file_num, file_size, time_to_live)
}

pub struct DbCache {
    fast_cache: Option<FastCache>,
    short_cache: Cache<u128, Arc<dyn CacheVal>, FxBuildHasher>,
    version_cache: Cache<u128, Arc<dyn CacheVal>, FxBuildHasher>,
    long_cache: Cache<u128, Arc<dyn CacheVal>, FxBuildHasher>,
    disk_cache: Option<Arc<DiskCache>>,
    fast_cache_hit: AtomicU64,
    long_cache_hit: AtomicU64,
    short_cache_hit: AtomicU64,
    version_cache_hit: AtomicU64,
    disk_cache_hit: AtomicU64,
    cache_request_count: AtomicU64,
    long_cache_evicted: Arc<AtomicU64>,
    short_cache_evicted: Arc<AtomicU64>,
    version_cache_evicted: Arc<AtomicU64>,
    ttl: u64,
}

impl DbCache {
    pub fn start(
        name: &str,
        is_hot_deploy: bool,
        path: Option<&Path>,
        use_fast_cache: bool,
        use_disk_cache: bool,
    ) -> Result<DbCache> {
        let ttl = std::env::var(format!("{}_CACHE_TTL", name))
            .unwrap_or_else(|_| DEFAULT_CACHE_TTL.to_owned())
            .parse::<u64>()
            .unwrap_or_else(|e| panic!("{}_CACHE_TTL has an error:{:?}", name, e));
        let ttl = ttl.saturating_mul(1_000_000_000 / (1 << MSEC_SHR));

        let fast_cache = if use_fast_cache {
            Some(get_fast_cache(name, ttl))
        } else {
            None
        };
        let disk_cache = if use_disk_cache && path.is_some() {
            Some(Arc::new(get_disk_cache(
                name,
                is_hot_deploy,
                path.unwrap(),
                ttl,
            )?))
        } else {
            None
        };
        let short_cache_evicted = Arc::new(AtomicU64::new(0));
        let short_cache = get_short_cache(name, Arc::clone(&short_cache_evicted));
        let version_cache_evicted = Arc::new(AtomicU64::new(0));
        let version_cache = get_short_cache(name, Arc::clone(&version_cache_evicted));
        let long_cache_evicted = Arc::new(AtomicU64::new(0));
        let long_cache = get_long_cache(name, Arc::clone(&long_cache_evicted), disk_cache.clone());
        Ok(DbCache {
            fast_cache,
            short_cache,
            version_cache,
            long_cache,
            disk_cache,
            fast_cache_hit: AtomicU64::new(0),
            long_cache_hit: AtomicU64::new(0),
            short_cache_hit: AtomicU64::new(0),
            version_cache_hit: AtomicU64::new(0),
            disk_cache_hit: AtomicU64::new(0),
            cache_request_count: AtomicU64::new(0),
            long_cache_evicted,
            short_cache_evicted,
            version_cache_evicted,
            ttl,
        })
    }

    pub fn stop(&self) {
        if let Some(ref disk_cache) = self.disk_cache {
            disk_cache.stop();
        }
    }

    pub async fn insert_short(&self, id: &dyn HashVal, value: Arc<dyn CacheVal>) {
        let hash = id.hash_val(value._shard_id());
        self.short_cache.insert(hash, value).await
    }

    pub async fn insert_version(&self, id: &dyn HashVal, value: Arc<dyn CacheVal>) {
        let hash = id.hash_val(value._shard_id());
        self.version_cache.insert(hash, value).await
    }

    pub async fn insert_long(
        &self,
        id: &dyn HashVal,
        value: Arc<dyn CacheVal>,
        use_fast_cache: bool,
    ) {
        let hash = id.hash_val(value._shard_id());
        if use_fast_cache {
            if let Some(ref fast_cache) = self.fast_cache {
                let old = fast_cache.insert(hash, value);
                if let Some(old) = old {
                    self.long_cache.insert(old.0, old.1).await;
                }
                return;
            }
        }
        self.long_cache.insert(hash, value).await;
    }

    pub async fn get<T>(
        &self,
        hash: u128,
        shard_id: ShardId,
        use_fast_cache: bool,
        from_memory: bool,
    ) -> Option<Arc<T>>
    where
        T: CacheVal,
    {
        let (now, msec) = get_cache_time();
        self.cache_request_count.fetch_add(1, Ordering::Relaxed);

        if use_fast_cache {
            if let Some(ref fast_cache) = self.fast_cache {
                let val = fast_cache
                    .get(hash, now, msec)
                    .filter(|v| v._shard_id() == shard_id)
                    .map(|v| v.downcast_arc::<T>().ok())
                    .unwrap_or(None);
                if val.is_some() {
                    self.fast_cache_hit.fetch_add(1, Ordering::Relaxed);
                    return val;
                }
            }
        }

        let val = self
            .long_cache
            .get(&hash)
            .filter(|v| v._shard_id() == shard_id)
            .map(|v| v.downcast_arc::<T>().ok())
            .unwrap_or(None);
        if let Some(val) = val {
            if val._time().less_than_ttl(msec, self.ttl) {
                return None;
            }
            if use_fast_cache {
                if let Some(ref fast_cache) = self.fast_cache {
                    fast_cache.insert(hash, val.clone());
                }
            }
            self.long_cache_hit.fetch_add(1, Ordering::Relaxed);
            return Some(val);
        }

        let val = self
            .short_cache
            .get(&hash)
            .filter(|v| v._shard_id() == shard_id)
            .map(|v| v.downcast_arc::<T>().ok())
            .unwrap_or(None);
        if let Some(val) = val {
            self.short_cache_hit.fetch_add(1, Ordering::Relaxed);
            self.long_cache.insert(hash, val.clone()).await;
            return Some(val);
        }

        if from_memory {
            return None;
        }

        if let Some(ref disk_cache) = self.disk_cache {
            if let Some(buf) = disk_cache.read(hash, T::__type_id(), T::_estimate()).await {
                match T::_decode(&buf) {
                    Ok(v) => {
                        if v._shard_id() == shard_id {
                            let val = Arc::new(v);
                            self.disk_cache_hit.fetch_add(1, Ordering::Relaxed);
                            self.long_cache.insert(hash, val.clone()).await;
                            return Some(val);
                        }
                    }
                    Err(e) => error!("{}", e),
                }
            }
        }
        None
    }

    pub async fn get_version<T>(&self, hash: u128, shard_id: ShardId) -> Option<Arc<T>>
    where
        T: CacheVal,
    {
        self.version_cache
            .get(&hash)
            .filter(|v| v._shard_id() == shard_id)
            .map(|v| v.downcast_arc::<T>().ok())
            .unwrap_or(None)
    }

    pub async fn invalidate(&self, id: &dyn HashVal, shard_id: ShardId) {
        if let Some(ref fast_cache) = self.fast_cache {
            fast_cache.invalidate(id.hash_val(shard_id));
        }
        self.short_cache.invalidate(&id.hash_val(shard_id)).await;
        self.long_cache.invalidate(&id.hash_val(shard_id)).await;
    }

    pub async fn invalidate_version(&self, id: &dyn HashVal, shard_id: ShardId) {
        self.version_cache.invalidate(&id.hash_val(shard_id)).await;
    }

    pub fn invalidate_all_of<T>(&self)
    where
        T: CacheVal,
    {
        self.short_cache
            .invalidate_entries_if(|_k, v| v.clone().downcast_arc::<T>().is_ok())
            .unwrap();
        self.long_cache
            .invalidate_entries_if(|_k, v| v.clone().downcast_arc::<T>().is_ok())
            .unwrap();
        if let Some(ref disk_cache) = self.disk_cache {
            disk_cache.invalidate_all_of(T::__type_id());
        }
        if let Some(ref fast_cache) = self.fast_cache {
            fast_cache.invalidate_all_of(T::__type_id());
        }
    }

    pub fn invalidate_all_of_version<T>(&self)
    where
        T: CacheVal,
    {
        self.version_cache
            .invalidate_entries_if(|_k, v| v.clone().downcast_arc::<T>().is_ok())
            .unwrap();
    }

    pub fn invalidate_all(&self) {
        self.short_cache.invalidate_all();
        self.version_cache.invalidate_all();
        self.long_cache.invalidate_all();
        if let Some(ref disk_cache) = self.disk_cache {
            disk_cache.invalidate_all();
        }
        if let Some(ref fast_cache) = self.fast_cache {
            fast_cache.invalidate_all();
        }
    }

    pub fn fast_cache_hit(&self) -> u64 {
        self.fast_cache_hit.load(Ordering::Relaxed)
    }
    pub fn long_cache_hit(&self) -> u64 {
        self.long_cache_hit.load(Ordering::Relaxed)
    }
    pub fn short_cache_hit(&self) -> u64 {
        self.short_cache_hit.load(Ordering::Relaxed)
    }
    pub fn version_cache_hit(&self) -> u64 {
        self.version_cache_hit.load(Ordering::Relaxed)
    }
    pub fn disk_cache_hit(&self) -> u64 {
        self.disk_cache_hit.load(Ordering::Relaxed)
    }
    pub fn cache_request_count(&self) -> u64 {
        self.cache_request_count.load(Ordering::Relaxed)
    }
    pub fn long_cache_evicted(&self) -> u64 {
        self.long_cache_evicted.load(Ordering::Relaxed)
    }
    pub fn short_cache_evicted(&self) -> u64 {
        self.short_cache_evicted.load(Ordering::Relaxed)
    }
    pub fn version_cache_evicted(&self) -> u64 {
        self.version_cache_evicted.load(Ordering::Relaxed)
    }
}
