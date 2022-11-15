use crossbeam::epoch::{self, Atomic, Owned, Shared};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

use super::msec::{get_cache_time, MSec};
use crate::cache::db_cache::CacheVal;

const SET_ASSOCIATIVE: u64 = 16;
const HASH_SHIFT: usize = 24;
const TIME_MASK: u64 = (1 << HASH_SHIFT) - 1;

pub struct FastCache {
    index: Vec<AtomicU64>,
    data: Vec<Atomic<Data>>,
    ttl: u64,
}

struct Data {
    hash: u128,
    value: Arc<dyn CacheVal>,
}

impl Data {
    fn is_timeout(&self, msec: MSec, ttl: u64) -> bool {
        self.value._time().less_than_ttl(msec, ttl)
    }
}

impl FastCache {
    pub fn new(size: u64, ttl: u64) -> FastCache {
        let size = (size
            / (std::mem::size_of::<AtomicU64>() + std::mem::size_of::<Atomic<Data>>()) as u64)
            as usize;
        let size = 1usize << (std::mem::size_of::<usize>() as u32 * 8 - size.leading_zeros() - 1);
        let mut index = Vec::with_capacity(size);
        let mut data = Vec::with_capacity(size);
        for _i in 0..size {
            index.push(AtomicU64::default());
            data.push(Atomic::null());
        }
        FastCache { index, data, ttl }
    }

    pub fn insert(
        &self,
        hash: u128,
        value: Arc<dyn CacheVal>,
    ) -> Option<(u128, Arc<dyn CacheVal>)> {
        let (now, msec) = get_cache_time();
        let index_mask = self.index.len() as u64 - 1;
        let hash_idx = (hash as u64) & index_mask;
        let mut idx = 0;
        let mut time = self.index[hash_idx as usize].load(Ordering::Relaxed) & TIME_MASK;
        for i in 0..SET_ASSOCIATIVE {
            let candidate =
                self.index[((hash_idx + i) & index_mask) as usize].load(Ordering::Relaxed);
            if candidate == 0 || (candidate & !TIME_MASK) == ((hash as u64) & !TIME_MASK) {
                idx = i;
                break;
            }
            if u24_less_than(candidate & TIME_MASK, time) {
                time = candidate & TIME_MASK;
                idx = i;
            }
        }
        let hash_time = ((hash as u64) & !TIME_MASK) | (now & TIME_MASK);
        let pos = ((hash_idx + idx) & index_mask) as usize;
        let guard = &epoch::pin();
        let old = self.data[pos].swap(Owned::new(Data { hash, value }), Ordering::SeqCst, guard);
        self.index[pos].store(hash_time, Ordering::Release);
        if !old.is_null() {
            let ret = unsafe { old.as_ref() }
                .filter(|v| !v.is_timeout(msec, self.ttl))
                .map(|v| (v.hash, v.value.clone()));
            unsafe { guard.defer_destroy(old) };
            guard.flush();
            return ret;
        }
        None
    }

    pub fn get(&self, hash: u128, now: u64, msec: MSec) -> Option<Arc<dyn CacheVal>> {
        let index_mask = self.index.len() as u64 - 1;
        let hash_idx = (hash as u64) & index_mask;
        for i in 0..SET_ASSOCIATIVE {
            let pos = ((hash_idx + i) & index_mask) as usize;
            let candidate = self.index[pos].load(Ordering::Relaxed);
            if candidate != 0 && (candidate & !TIME_MASK) == ((hash as u64) & !TIME_MASK) {
                let guard = &epoch::pin();
                let ptr = self.data[pos].load_consume(guard);
                if let Some(data) = unsafe { ptr.as_ref() } {
                    if data.hash == hash {
                        if data.is_timeout(msec, self.ttl) {
                            let old = self.data[pos].swap(Shared::null(), Ordering::SeqCst, guard);
                            if !old.is_null() {
                                unsafe { guard.defer_destroy(old) };
                                guard.flush();
                            }
                            self.index[pos].store(0, Ordering::Release);
                            return None;
                        }
                        let hash_time = ((hash as u64) & !TIME_MASK) | (now & TIME_MASK);
                        if candidate != hash_time {
                            let _ = self.index[pos].compare_exchange_weak(
                                candidate,
                                hash_time,
                                Ordering::Relaxed,
                                Ordering::Relaxed,
                            );
                        }
                        return Some(Arc::clone(&data.value));
                    }
                }
            }
        }
        None
    }

    pub fn invalidate(&self, hash: u128) {
        let index_mask = self.index.len() as u64 - 1;
        let hash_idx = (hash as u64) & index_mask;
        for i in 0..SET_ASSOCIATIVE {
            let pos = ((hash_idx + i) & index_mask) as usize;
            let candidate = self.index[pos].load(Ordering::Relaxed);
            if candidate != 0 && (candidate & !TIME_MASK) == ((hash as u64) & !TIME_MASK) {
                let guard = &epoch::pin();
                let ptr = self.data[pos].load_consume(guard);
                if let Some(data) = unsafe { ptr.as_ref() } {
                    if data.hash == hash {
                        let old = self.data[pos].swap(Shared::null(), Ordering::SeqCst, guard);
                        if !old.is_null() {
                            unsafe { guard.defer_destroy(old) };
                            guard.flush();
                        }
                        self.index[pos].store(0, Ordering::Release);
                    }
                }
                return;
            }
        }
    }

    pub fn invalidate_all_of(&self, type_id: u64) {
        let guard = &epoch::pin();
        for i in 0..self.data.len() {
            let ptr = self.data[i].load_consume(guard);
            if let Some(data) = unsafe { ptr.as_ref() } {
                if data.value._type_id() == type_id {
                    let old = self.data[i].swap(Shared::null(), Ordering::SeqCst, guard);
                    if !old.is_null() {
                        unsafe { guard.defer_destroy(old) };
                    }
                    self.index[i].store(0, Ordering::Release);
                }
            }
        }
        guard.flush();
    }

    pub fn invalidate_all(&self) {
        let guard = &epoch::pin();
        for i in 0..self.data.len() {
            let ptr = self.data[i].load_consume(guard);
            if !ptr.is_null() {
                let old = self.data[i].swap(Shared::null(), Ordering::SeqCst, guard);
                if !old.is_null() {
                    unsafe { guard.defer_destroy(old) };
                }
                self.index[i].store(0, Ordering::Release);
            }
        }
        guard.flush();
    }
}

fn u24_less_than(lhs: u64, rhs: u64) -> bool {
    let lhs = lhs & 0xFFFFFF;
    let rhs = rhs & 0xFFFFFF;
    let lhs = lhs | ((lhs & 0x800000) * 0x1FFFFFFFFFE);
    let rhs = rhs | ((rhs & 0x800000) * 0x1FFFFFFFFFE);
    lhs.wrapping_sub(rhs) > u64::MAX / 2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct A(u32);
    impl CacheVal for A {
        fn _size(&self) -> u32 {
            10
        }
        fn _type_id(&self) -> u64 {
            Self::__type_id()
        }
        fn __type_id() -> u64 {
            1
        }
        fn _shard_id(&self) -> crate::ShardId {
            1
        }
        fn _time(&self) -> MSec {
            MSec::now()
        }
        fn _estimate() -> usize {
            10
        }
        fn _encode(&self) -> anyhow::Result<Vec<u8>> {
            Ok(Vec::new())
        }
        fn _decode(_v: &[u8]) -> anyhow::Result<Self> {
            Ok(Self(1))
        }
    }

    #[test]
    fn test_u40_less_than() {
        assert!(u24_less_than(1, 2));
        assert!(!u24_less_than(1, 1));
        assert!(!u24_less_than(2, 1));
        assert!(u24_less_than(0xffffff, 0));
        assert!(u24_less_than(0xfffffe, 0xffffff));
    }
    #[test]
    fn test() {
        let cache = Arc::new(FastCache::new(16, 1000));
        let cache2 = cache.clone();
        let (now, msec) = get_cache_time();
        std::thread::spawn(move || {
            cache.insert(1, Arc::new(A(1)));
            cache.insert(1, Arc::new(A(1)));
            let _result = cache.get(1, now, msec);
            // println!("{:?}", result);
        })
        .join()
        .unwrap();
        std::thread::spawn(move || {
            cache2.insert(2, Arc::new(A(2)));
            let _result = cache2.get(2, now, msec);
            // println!("{:?}", result);
        })
        .join()
        .unwrap();
    }
}
