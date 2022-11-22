#[allow(unused_imports)]
use anyhow::{Context as _, Result};
use once_cell::sync::OnceCell;
use rand::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

#[derive(Clone, Copy)]
pub struct Ctx {
    req_no: u64,
    time: SystemTime,
}

impl Ctx {
    pub fn new() -> Ctx {
        Ctx {
            req_no: get_req_no(),
            time: SystemTime::now(),
        }
    }
}
impl Ctx {
    pub fn req_no(&self) -> u64 {
        self.req_no
    }

    pub fn time(&self) -> SystemTime {
        self.time
    }
}

impl log::kv::ToValue for Ctx {
    fn to_value(&self) -> log::kv::Value {
        self.req_no.into()
    }
}

fn get_req_no() -> u64 {
    static REQ_ID: OnceCell<AtomicU64> = OnceCell::new();
    REQ_ID
        .get_or_init(|| {
            let mut x: u32 = random::<u32>();
            if x == 0 {
                x = 1;
            }
            AtomicU64::new((x as u64) << 30)
        })
        .fetch_add(1, Ordering::SeqCst)
}
