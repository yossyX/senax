use senax_encoder::{Pack, Unpack};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

use super::CycleCounter;

pub const MSEC_SHR: u8 = 20;

#[derive(Deserialize, Serialize, Pack, Unpack, Clone, Copy, Debug, Default)]
pub struct MSec(u64);

impl MSec {
    pub fn now() -> MSec {
        MSec::from(
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap(),
        )
    }
    pub fn inner(&self) -> u64 {
        self.0
    }
    #[deprecated]
    pub fn get(&self) -> u64 {
        self.0
    }
    pub fn less_than_ttl(&self, time: MSec, ttl: u64) -> bool {
        self.0.less_than(time.0.wrapping_sub(ttl))
    }
    pub fn add(&self, v: u64) -> MSec {
        MSec(self.0.wrapping_add(v))
    }
    pub fn add_sec(&self, v: u64) -> MSec {
        MSec(
            self.0
                .wrapping_add(v.saturating_mul(1_000_000_000 / (1 << MSEC_SHR))),
        )
    }
    pub fn sub(&self, v: u64) -> MSec {
        MSec(self.0.wrapping_sub(v))
    }
    pub fn less_than(&self, time: MSec) -> bool {
        self.0.less_than(time.0)
    }
}
impl From<Duration> for MSec {
    fn from(time: Duration) -> Self {
        MSec((time.as_nanos() >> MSEC_SHR) as u64)
    }
}
impl From<u64> for MSec {
    fn from(time: u64) -> Self {
        MSec(time)
    }
}

pub(crate) fn get_cache_time() -> (u64, MSec) {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();
    (duration.as_secs(), duration.into())
}
