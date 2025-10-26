use actix_web::{HttpMessage, HttpRequest};
#[allow(unused_imports)]
use anyhow::{Context as _, Result};
use chrono::{DateTime, Utc};
use log::info;
use once_cell::sync::OnceCell;
use rand::prelude::*;
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

pub static DEBUG_TIME: AtomicI64 = AtomicI64::new(0);

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct Ctx {
    ctx_no: u64,
    time: SystemTime,
}

#[allow(clippy::new_without_default)]
impl Ctx {
    pub fn new() -> Ctx {
        let mut time = SystemTime::now();
        if cfg!(debug_assertions) {
            let debug_time = DEBUG_TIME.load(Ordering::Relaxed);
            match debug_time.cmp(&0) {
                std::cmp::Ordering::Greater => {
                    time += Duration::from_secs(debug_time as u64);
                }
                std::cmp::Ordering::Less => {
                    time -= Duration::from_secs((-debug_time) as u64);
                }
                std::cmp::Ordering::Equal => {}
            }
        }
        Ctx {
            ctx_no: get_ctx_no(),
            time,
        }
    }
    pub fn get(http_req: &HttpRequest) -> Ctx {
        if let Some(ctx) = http_req.extensions().get::<Ctx>() {
            *ctx
        } else if cfg!(test) {
            Ctx::new()
        } else {
            panic!("Ctx required")
        }
    }

    pub fn ctx_no(&self) -> u64 {
        self.ctx_no
    }

    #[allow(dead_code)]
    pub fn set_time(&mut self, time: SystemTime) {
        self.time = time;
    }

    #[allow(dead_code)]
    pub fn time(&self) -> SystemTime {
        self.time
    }

    #[allow(dead_code)]
    pub fn utc(&self) -> DateTime<Utc> {
        self.time.into()
    }

    #[allow(dead_code)]
    pub fn log(&self, http_req: &HttpRequest) {
        info!(target: "request",
                ctx = self.ctx_no(),
                method = http_req.method().as_str(),
                path = http_req.path(),
                query = http_req.query_string(); "");
    }

    #[allow(dead_code)]
    pub fn log_with_data<T>(&self, http_req: &HttpRequest, data: &T)
    where
        T: serde::Serialize,
    {
        info!(target: "request",
                ctx = self.ctx_no(),
                method = http_req.method().as_str(),
                path = http_req.path(),
                query = http_req.query_string(),
                data = log::kv::Value::from_serde(data); "");
    }
}

impl log::kv::ToValue for Ctx {
    fn to_value(&self) -> log::kv::Value<'_> {
        self.ctx_no.into()
    }
}

fn get_ctx_no() -> u64 {
    static CTX_NO: OnceCell<AtomicU64> = OnceCell::new();
    CTX_NO
        .get_or_init(|| {
            let mut x: u32 = random::<u32>();
            if x == 0 {
                x = 1;
            }
            AtomicU64::new((x as u64) << 30)
        })
        .fetch_add(1, Ordering::SeqCst)
}
@{-"\n"}@