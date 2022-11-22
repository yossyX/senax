use actix_web::{HttpMessage, HttpRequest};
use chrono::{DateTime, Local};
use log::info;
use std::fmt;

use crate::context::Ctx;

pub fn get_ctx_and_log(http_req: &HttpRequest) -> Ctx {
    let ctx = *http_req.extensions().get::<Ctx>().unwrap();
    let time: DateTime<Local> = Local::now();
    let time = time.to_rfc3339_opts(chrono::SecondsFormat::Millis, false);
    info!(target: "request",
        "time:{}\treq_no:{}\tmethod:{}\tpath:{}\tquery:{}",
        time, ctx.req_no(),
        http_req.method().as_str(),
        http_req.path(),
        http_req.query_string());
    ctx
}

pub fn get_ctx_and_log_with_data(http_req: &HttpRequest, data: &impl fmt::Debug) -> Ctx {
    let ctx = *http_req.extensions().get::<Ctx>().unwrap();
    let time: DateTime<Local> = Local::now();
    let time = time.to_rfc3339_opts(chrono::SecondsFormat::Millis, false);
    info!(target: "request",
        "time:{}\treq_no:{}\tmethod:{}\tpath:{}\tquery:{}\tdata:{:?}",
        time, ctx.req_no(),
        http_req.method().as_str(),
        http_req.path(),
        http_req.query_string(),
        data);
    ctx
}
