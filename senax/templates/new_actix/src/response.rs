use actix_web::http::header::TryIntoHeaderValue;
use actix_web::web::{Bytes, BytesMut};
use actix_web::{error, HttpRequest, HttpResponse, Responder};
use chrono::{DateTime, Local};
use derive_more::Display;
use futures::{Stream, StreamExt};
use senax_common::err;
use serde::Serialize;

use crate::context::Ctx;

#[derive(Debug, Display, Serialize)]
pub struct BadRequest {
    msg: String,
}
impl std::error::Error for BadRequest {}
impl BadRequest {
    pub fn new(msg: String) -> BadRequest {
        BadRequest { msg }
    }
}

#[derive(Debug)]
pub struct NotFound {
    pub path: String,
}
impl NotFound {
    #[allow(dead_code)]
    pub fn new(http_req: &HttpRequest) -> NotFound {
        NotFound {
            path: http_req.path().to_string(),
        }
    }
}
impl std::fmt::Display for NotFound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Not Found: path={}", self.path)
    }
}
impl std::error::Error for NotFound {}

#[allow(dead_code)]
pub fn json_response<T: Serialize>(r: Result<T, anyhow::Error>, ctx: &Ctx) -> impl Responder {
    match r {
        Ok(data) => {
            let mut writer = Vec::with_capacity(4096);
            match serde_json::to_writer(&mut writer, &data) {
                Ok(_) => {
                    let response = unsafe { String::from_utf8_unchecked(writer) };
                    let time: DateTime<Local> = Local::now();
                    let time = time.to_rfc3339_opts(chrono::SecondsFormat::Millis, false);
                    info!(target:"response", "time:{}\treq_no:{}\tresponse:{}", time, ctx.req_no(), &response);
                    HttpResponse::Ok()
                        .content_type(mime::APPLICATION_JSON)
                        .body(response)
                }
                Err(err) => error_response(err.into(), ctx),
            }
        }
        Err(err) => error_response(err, ctx),
    }
}

#[allow(dead_code)]
pub fn json_stream_response<T: Serialize>(
    r: Result<impl Stream<Item = T> + 'static, anyhow::Error>,
    ctx: &Ctx,
) -> impl Responder {
    let result = {
        r.map(|stream| {
            let stream_tasks = async_stream::stream! {
                futures::pin_mut!(stream);
                let mut bytes = BytesMut::new();
                bytes.extend_from_slice("[".as_bytes());
                let mut first = true;
                while let Some(v) = stream.next().await {
                    if !first {
                        bytes.extend_from_slice(",".as_bytes());
                    }
                    first = false;
                    let mut writer = Vec::with_capacity(4096);
                    match serde_json::to_writer(&mut writer, &v) {
                        Ok(_) => {
                            bytes.extend_from_slice(&writer);
                            let byte = bytes.split().freeze();
                            yield Ok::<Bytes, actix_web::http::Error>(byte)
                        },
                        Err(err) => error!("Tasks list stream error: {}", err)
                    }
                }
                bytes.extend_from_slice("]".as_bytes());
                let byte = bytes.split().freeze();
                yield Ok::<Bytes, actix_web::http::Error>(byte);
            };
            Box::pin(stream_tasks)
        })
    };
    stream_response(result, mime::APPLICATION_JSON, ctx)
}

#[allow(dead_code)]
pub fn ndjson_stream_response<T: Serialize>(
    r: Result<impl Stream<Item = T> + 'static, anyhow::Error>,
    ctx: &Ctx,
) -> impl Responder {
    let result = {
        r.map(|stream| {
            let stream_tasks = async_stream::stream! {
                futures::pin_mut!(stream);
                let mut bytes = BytesMut::new();
                while let Some(v) = stream.next().await {
                    let mut writer = Vec::with_capacity(4096);
                    match serde_json::to_writer(&mut writer, &v) {
                        Ok(_) => {
                            bytes.extend_from_slice(&writer);
                            bytes.extend_from_slice("\n".as_bytes());
                            let byte = bytes.split().freeze();
                            yield Ok::<Bytes, actix_web::http::Error>(byte)
                        },
                        Err(err) => error!("Tasks list stream error: {}", err)
                    }
                }
            };
            Box::pin(stream_tasks)
        })
    };
    stream_response(result, "application/x-ndjson", ctx)
}

pub fn stream_response<S, V>(
    e: Result<S, anyhow::Error>,
    content_type: V,
    ctx: &Ctx,
) -> impl Responder
where
    S: Stream<Item = Result<Bytes, actix_web::http::Error>> + 'static,
    V: TryIntoHeaderValue,
{
    match e {
        Ok(stream) => {
            let time: DateTime<Local> = Local::now();
            let time = time.to_rfc3339_opts(chrono::SecondsFormat::Millis, false);
            info!(target:"response", "time:{}\treq_no:{}", time, ctx.req_no());
            HttpResponse::Ok()
                .content_type(content_type)
                .streaming(stream)
        }
        Err(err) => error_response(err, ctx),
    }
}

pub fn json_error_handler(err: error::JsonPayloadError, _req: &HttpRequest) -> error::Error {
    use actix_web::error::JsonPayloadError;

    let detail = err.to_string();
    let resp = match &err {
        JsonPayloadError::ContentType => HttpResponse::UnsupportedMediaType().body(detail),
        JsonPayloadError::Deserialize(json_err) if json_err.is_data() => {
            HttpResponse::UnprocessableEntity().json(crate::response::BadRequest::new(detail))
        }
        _ => HttpResponse::BadRequest().body(detail),
    };
    error::InternalError::from_response(err, resp).into()
}

fn error_response(err: anyhow::Error, ctx: &Ctx) -> HttpResponse {
    if let Some(e) = err.downcast_ref::<validator::ValidationErrors>() {
        info!(target: "server::validation_errors", req_no = ctx.req_no(); "{}", e);
        HttpResponse::BadRequest().json(e)
    } else if let Some(e) = err.downcast_ref::<BadRequest>() {
        info!(target: "server::bad_request", req_no = ctx.req_no(); "{}", e);
        HttpResponse::BadRequest().json(e)
    } else if let Some(e) = err.downcast_ref::<err::RowNotFound>() {
        info!(target: "server::row_not_found", req_no = ctx.req_no(), table = e.table; "{}", e.id);
        HttpResponse::NotFound().body("not found")
    } else if let Some(e) = err.downcast_ref::<NotFound>() {
        info!(target: "server::not_found", req_no = ctx.req_no(); "{}", e);
        HttpResponse::NotFound().body("not found")
    } else {
        warn!(req_no = ctx.req_no(); "{}", err.root_cause());
        HttpResponse::InternalServerError().body("Internal Server Error")
    }
}
@{-"\n"}@