use crate::request::*;
use crate::response::*;
use actix_web::{get, web, HttpMessage, HttpRequest, HttpResponse, Responder};
#[allow(unused_imports)]
use anyhow::{Context as _, Result};
use chrono::Local;
use db_sample::misc::ForUpdateTr;
use db_sample::note::counters::*;
use db_sample::note::note::*;
#[allow(unused_imports)]
use db_sample::DbConn as SampleConn;
use serde::Serialize;
#[allow(unused_imports)]
use tracing::trace_span;

#[derive(Serialize)]
pub struct Response {
    pub id: _NoteId,
    pub category: Option<String>,
    pub article: String,
    pub tags: Vec<String>,
    pub count: u64,
}

#[get("/cache/{key}")]
async fn handler(key: web::Path<String>, http_req: HttpRequest) -> impl Responder {
    let ctx = get_ctx_and_log(&http_req);
    let result = async move {
        let mut conn = SampleConn::new();
        let mut note = _Note::find_by_key_from_cache(&conn, &*key)
            .await
            .with_context(|| NotFound::new(&http_req))?;
        note.fetch_category(&mut conn).await?;

        let category = match note.category() {
            None => None,
            Some(v) => Some(v.name().to_owned()),
        };

        let date = Local::now().date_naive();
        let counter = _Counters::find_optional_from_cache(&conn, (note.id(), date)).await?;
        let count = counter.map(|v| v.counter()).unwrap_or_default() + 1;
        let mut counter_for_update = _CountersFactory {
            note_id: note.id(),
            date,
            counter: 0,
        }
        .create(&conn);
        let _ = counter_for_update.counter().add(1);
        counter_for_update._upsert();
        _Counters::save_delayed(&mut conn, counter_for_update).await?;

        Ok(Response {
            id: note.id(),
            category,
            article: note.content().to_string(),
            tags: note.tags().iter().map(|v| v.name().to_string()).collect(),
            count,
        })
    }
    .await;
    json_response(result, &ctx)
}
