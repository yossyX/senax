use actix_web::web::{Bytes, BytesMut};
use actix_web::{HttpRequest, HttpResponse, Responder, error};
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

    #[error("Bad Request: {0}")]
    ValidationError(validator::ValidationErrors),

    #[error("Internal Server Error")]
    InternalServerError(String),
}

#[allow(dead_code)]
pub fn json_response<T: Serialize>(r: Result<T, anyhow::Error>, ctx: Ctx) -> impl Responder {
    match r {
        Ok(data) => {
            let mut writer = Vec::with_capacity(65536);
            match serde_json::to_writer(&mut writer, &data) {
                Ok(_) => {
                    let response = unsafe { String::from_utf8_unchecked(writer) };
                    log::info!(target:"response", ctx = ctx.ctx_no(), response = &response; "");
                    HttpResponse::Ok()
                        .content_type(mime::APPLICATION_JSON)
                        .body(response)
                }
                Err(err) => error_response(err.into(), &ctx),
            }
        }
        Err(err) => error_response(err, &ctx),
    }
}

#[allow(dead_code)]
/// The response is returned as a stream in either JSON or NDJSON format.
/// The end of an NDJSON file is marked by a newline.
/// If the stream is interrupted due to an error, there will be no newline, allowing the error to be detected.
pub fn json_stream_response<T: Serialize>(
    result: Result<impl Stream<Item = Result<T, anyhow::Error>> + 'static, anyhow::Error>,
    ctx: Ctx,
    ndjson: bool,
) -> impl Responder {
    let (c1, c2, c3, content_type) = if ndjson {
        ("", "\n", "\n", "application/x-ndjson")
    } else {
        ("[", ",", "]", "application/json")
    };
    match result {
        Ok(stream) => {
            let ctx_no = ctx.ctx_no();
            let stream = Box::pin(async_stream::stream! {
                futures::pin_mut!(stream);
                let mut sep = "";
                let mut line = 0;
                yield Ok(Bytes::from_static(c1.as_bytes()));
                while let Some(v) = stream.next().await {
                    match v {
                        Ok(v) => {
                            match serde_json::to_string(&v) {
                                Ok(json) => {
                                    line += 1;
                                    let mut bytes = BytesMut::with_capacity(json.len() + 1);
                                    bytes.extend_from_slice(sep.as_bytes());
                                    sep = c2;
                                    bytes.extend_from_slice(json.as_bytes());
                                    info!(target:"response", ctx = ctx_no, stream = line; "{}", json);
                                    yield Ok::<Bytes, anyhow::Error>(bytes.freeze())
                                },
                                Err(err) => {
                                    error!("stream error: {}", err);
                                    return;
                                }
                            }
                        }
                        Err(err) => {
                            error!("stream error: {}", err);
                            return;
                        }
                    }
                }
                yield Ok(Bytes::from_static(c3.as_bytes()));
            });
            HttpResponse::Ok()
                .content_type(content_type)
                .streaming(stream)
        }
        Err(err) => error_response(err, &ctx),
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
                HttpResponse::NotFound().body("Not Found")
            }
            ApiError::Unauthorized => {
                info!(target: "server::unauthorized", ctx = ctx.ctx_no(); "{}", e);
                HttpResponse::Unauthorized().body("Unauthorized")
            }
            ApiError::Forbidden => {
                warn!(target: "server::forbidden", ctx = ctx.ctx_no(); "{}", e);
                HttpResponse::Forbidden().body("Forbidden")
            }
            ApiError::BadRequest(msg) => {
                info!(target: "server::bad_request", ctx = ctx.ctx_no(); "{}", msg);
                HttpResponse::BadRequest().body(msg.to_string())
            }
            ApiError::BadRequestJson(value) => {
                info!(target: "server::bad_request", ctx = ctx.ctx_no(); "{}", value);
                HttpResponse::BadRequest().json(value)
            }
            ApiError::ValidationError(errors) => {
                info!(target: "server::bad_request", ctx = ctx.ctx_no(); "{}", errors);
                HttpResponse::BadRequest().json(errors)
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

impl From<&ApiError> for actix_web::Error {
    fn from(e: &ApiError) -> Self {
        match e {
            ApiError::NotFound => actix_web::error::ErrorNotFound("Not Found"),
            ApiError::Unauthorized => actix_web::error::ErrorUnauthorized("Unauthorized"),
            ApiError::Forbidden => actix_web::error::ErrorForbidden("Forbidden"),
            ApiError::BadRequest(msg) => actix_web::error::ErrorBadRequest(msg.to_string()),
            ApiError::BadRequestJson(value) => actix_web::error::ErrorBadRequest(value.to_string()),
            ApiError::ValidationError(errors) => {
                actix_web::error::ErrorBadRequest(errors.to_string())
            }
            ApiError::InternalServerError(err) => {
                error!(target: "server::internal_error", "{}", err);
                actix_web::error::ErrorInternalServerError("Internal Server Error")
            }
        }
    }
}
@{-"\n"}@