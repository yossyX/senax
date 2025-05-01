use crate::_base::context::Ctx;
use actix_web::{get, HttpRequest, HttpResponse, Responder};
#[allow(unused_imports)]
use anyhow::{Context as _, Result};
#[allow(unused_imports)]
use tracing::trace_span;

#[get("/")]
pub async fn handler(http_req: HttpRequest) -> impl Responder {
    let ctx = Ctx::get(&http_req);
    ctx.log(&http_req);

    let span = trace_span!("handler");
    let _ = span.enter();
    HttpResponse::Ok().body(
        r#"
        Welcome to Senax example.
        "#,
    )
}
@{-"\n"}@