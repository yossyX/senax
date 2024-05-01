use fxhash::FxHashMap;
use std::{
    borrow::Borrow,
    hash::Hash,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::RwLock;

pub struct StreamCounter<K> {
    tick: u64,
    divisor: u64,
    counter: RwLock<FxHashMap<u64, FxHashMap<K, usize>>>,
}

impl<K> StreamCounter<K>
where
    K: Hash + Eq,
{
    pub fn new(span: Duration, divisor: u64) -> Self {
        let span = span.as_millis() as u64;
        let tick = std::cmp::max(1, span / divisor);
        Self {
            tick,
            divisor,
            counter: Default::default(),
        }
    }

    pub async fn add(&self, key: K) {
        self.add_with_time(key, SystemTime::now()).await;
    }

    pub async fn add_with_time(&self, key: K, time: SystemTime) {
        let mut counter = self.counter.write().await;
        let time = time.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64 / self.tick;
        let c = counter.entry(time).or_default().entry(key).or_default();
        *c += 1;
        let limit = time - self.divisor;
        counter.retain(|t, _| *t > limit);
    }

    pub async fn count<Q>(&self, key: &Q)
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.count_with_time(key, SystemTime::now()).await;
    }

    pub async fn count_with_time<Q>(&self, key: &Q, time: SystemTime) -> usize
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        let counter = self.counter.read().await;
        let time =
            time.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64 / self.tick - self.divisor;
        let mut count = 0;
        for (t, map) in counter.iter() {
            if *t > time {
                if let Some(c) = map.get(key.borrow()) {
                    count += c;
                }
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test() {
        let counter = StreamCounter::new(Duration::from_secs(60), 10);
        let now = UNIX_EPOCH + Duration::from_secs(600);
        counter.add_with_time("key".to_owned(), now).await;
        assert_eq!(counter.count_with_time("key", now).await, 1);
        assert_eq!(counter.count_with_time("dummy", now).await, 0);
        let now = now + Duration::from_secs(10);
        counter.add_with_time("key".to_owned(), now).await;
        assert_eq!(counter.count_with_time("key", now).await, 2);
        let now = now + Duration::from_secs(10);
        counter.add_with_time("key".to_owned(), now).await;
        assert_eq!(counter.count_with_time("key", now).await, 3);
        let now = now + Duration::from_secs(10);
        counter.add_with_time("key".to_owned(), now).await;
        assert_eq!(counter.count_with_time("key", now).await, 4);
        let now = now + Duration::from_secs(10);
        counter.add_with_time("key".to_owned(), now).await;
        assert_eq!(counter.count_with_time("key", now).await, 5);
        let now = now + Duration::from_secs(10);
        counter.add_with_time("key".to_owned(), now).await;
        assert_eq!(counter.count_with_time("key", now).await, 6);
        let now = now + Duration::from_secs(9);
        counter.add_with_time("key".to_owned(), now).await;
        assert_eq!(counter.count_with_time("key", now).await, 7);
        let now = now + Duration::from_secs(2);
        counter.add_with_time("key".to_owned(), now).await;
        counter.add_with_time("key".to_owned(), now).await;
        assert_eq!(counter.count_with_time("key", now).await, 8);
        let now = now + Duration::from_secs(10);
        counter.add_with_time("key".to_owned(), now).await;
        counter.add_with_time("key".to_owned(), now).await;
        assert_eq!(counter.count_with_time("key", now).await, 9);
        let now = now + Duration::from_secs(10);
        counter.add_with_time("key".to_owned(), now).await;
        counter.add_with_time("key".to_owned(), now).await;
        assert_eq!(counter.count_with_time("key", now).await, 10);
    }
}
