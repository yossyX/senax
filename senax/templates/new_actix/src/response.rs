use actix_web::http::header::TryIntoHeaderValue;
use actix_web::web::{Bytes, BytesMut};
use actix_web::{error, HttpRequest, HttpResponse, Responder};
use futures::{Stream, StreamExt};
use serde::Serialize;

use crate::context::Ctx;

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum ApiError {
    #[error("Not Found")]
    NotFound,

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Forbidden")]
    Forbidden,

    #[error("Bad Request: {0}")]
    BadRequest(String),

    #[error("Bad Request: {0}")]
    BadRequestJson(serde_json::Value),

    #[error("Internal Server Error")]
    InternalServerError(String),
}

#[allow(dead_code)]
pub fn json_response<T: Serialize>(r: Result<T, anyhow::Error>, ctx: &Ctx) -> impl Responder {
    match r {
        Ok(data) => {
            let mut writer = Vec::with_capacity(65536);
            match serde_json::to_writer(&mut writer, &data) {
                Ok(_) => {
                    let response = unsafe { String::from_utf8_unchecked(writer) };
                    info!(target:"response", ctx = ctx.ctx_no(), response = &response; "");
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
                    let mut writer = Vec::with_capacity(65536);
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
                    let mut writer = Vec::with_capacity(65536);
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
            info!(target:"response", ctx = ctx.ctx_no(); "stream");
            HttpResponse::Ok()
                .content_type(content_type)
                .streaming(stream)
        }
        Err(err) => error_response(err, ctx),
    }
}

#[allow(dead_code)]
pub fn json_error_handler(err: error::JsonPayloadError, http_req: &HttpRequest) -> error::Error {
    use actix_web::error::JsonPayloadError;
    let detail = err.to_string();
    let ctx = Ctx::get(http_req);
    info!(target: "server::json_error", ctx = ctx.ctx_no(); "{}", &detail);
    let resp = match &err {
        JsonPayloadError::ContentType => HttpResponse::UnsupportedMediaType().body(detail),
        _ => HttpResponse::BadRequest().body(detail),
    };
    error::InternalError::from_response(err, resp).into()
}

fn error_response(err: anyhow::Error, ctx: &Ctx) -> HttpResponse {
    if let Some(e) = err.downcast_ref::<validator::ValidationErrors>() {
        info!(target: "server::validation_errors", ctx = ctx.ctx_no(); "{}", e);
        HttpResponse::BadRequest().json(e)
    } else if let Some(e) = err.downcast_ref::<ApiError>() {
        match e {
            ApiError::NotFound => {
                info!(target: "server::not_found", ctx = ctx.ctx_no(); "{}", e);
                HttpResponse::NotFound().body("not found")
            }
            ApiError::Unauthorized => {
                info!(target: "server::unauthorized", ctx = ctx.ctx_no(); "{}", e);
                HttpResponse::Unauthorized().body("unauthorized")
            }
            ApiError::Forbidden => {
                warn!(target: "server::forbidden", ctx = ctx.ctx_no(); "{}", e);
                HttpResponse::Forbidden().body("forbidden")
            }
            ApiError::BadRequest(msg) => {
                info!(target: "server::bad_request", ctx = ctx.ctx_no(); "{}", msg);
                HttpResponse::BadRequest().body(msg.to_string())
            }
            ApiError::BadRequestJson(value) => {
                info!(target: "server::bad_request", ctx = ctx.ctx_no(); "{}", value);
                HttpResponse::BadRequest().json(value)
            }
            ApiError::InternalServerError(err) => {
                error!(target: "server::internal_error", ctx = ctx.ctx_no(); "{}", err);
                HttpResponse::InternalServerError().body("Internal Server Error")
            }
        }
    } else if let Some(e) = err.downcast_ref::<senax_common::err::RowNotFound>() {
        warn!(target: "server::row_not_found", ctx = ctx.ctx_no(), table = e.table; "{}", e.id);
        HttpResponse::BadRequest().body("Bad Request")
    } else if let Some(e) = err.downcast_ref::<sqlx::Error>() {
        match e {
            sqlx::Error::Database(e) => {
                use sqlx::error::ErrorKind;
                match e.kind() {
                    ErrorKind::UniqueViolation => {
                        warn!(target: "server::bad_request", ctx = ctx.ctx_no(); "{}", err);
                        HttpResponse::Conflict().body("Conflict")
                    }
                    ErrorKind::Other => {
                        error!(target: "server::internal_error", ctx = ctx.ctx_no(); "{}", err);
                        HttpResponse::InternalServerError().body("Internal Server Error")
                    }
                    _ => {
                        warn!(target: "server::bad_request", ctx = ctx.ctx_no(); "{}", err);
                        HttpResponse::BadRequest().body("Bad Request")
                    }
                }
            }
            sqlx::Error::RowNotFound => {
                warn!(target: "server::bad_request", ctx = ctx.ctx_no(); "{}", err);
                HttpResponse::BadRequest().body("Bad Request")
            }
            _ => {
                error!(target: "server::internal_error", ctx = ctx.ctx_no(); "{}", err);
                HttpResponse::InternalServerError().body("Internal Server Error")
            }
        }
    } else {
        error!(target: "server::internal_error", ctx = ctx.ctx_no(); "{}", err.root_cause());
        HttpResponse::InternalServerError().body("Internal Server Error")
    }
}
@{-"\n"}@