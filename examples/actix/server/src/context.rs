use actix_web::{HttpMessage, HttpRequest};
#[allow(unused_imports)]
use anyhow::{Context as _, Result};
use chrono::{DateTime, Local};
use log::info;
use once_cell::sync::OnceCell;
use rand::prelude::*;
use std::fmt;
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
    pub fn get(http_req: &HttpRequest) -> Ctx {
        *http_req.extensions().get::<Ctx>().unwrap()
    }

    pub fn req_no(&self) -> u64 {
        self.req_no
    }

    pub fn time(&self) -> SystemTime {
        self.time
    }

    pub fn log(&self, http_req: &HttpRequest) {
        let time: DateTime<Local> = Local::now();
        let time = time.to_rfc3339_opts(chrono::SecondsFormat::Millis, false);
        info!(target: "request",
            "time:{}\treq_no:{}\tmethod:{}\tpath:{}\tquery:{}",
            time, self.req_no(),
            http_req.method().as_str(),
            http_req.path(),
            http_req.query_string());
    }

    pub fn log_with_data(&self, http_req: &HttpRequest, data: &impl fmt::Debug) {
        let time: DateTime<Local> = Local::now();
        let time = time.to_rfc3339_opts(chrono::SecondsFormat::Millis, false);
        info!(target: "request",
            "time:{}\treq_no:{}\tmethod:{}\tpath:{}\tquery:{}\tdata:{:?}",
            time, self.req_no(),
            http_req.method().as_str(),
            http_req.path(),
            http_req.query_string(),
            data);
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
