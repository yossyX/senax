use crate::context::Ctx;
use crate::response::*;
use actix_web::{get, web, HttpRequest, Responder};
#[allow(unused_imports)]
use anyhow::{Context as _, Result};
use chrono::Local;
use db_sample::note::counter::*;
use db_sample::note::note::*;
use db_sample::note::tag::_TagTr;
#[allow(unused_imports)]
use db_sample::DbConn as SampleConn;
use db_session::session::session::_SessionStore;
use senax_actix_session::Session;
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

#[get("/no_cache/{key}")]
async fn handler(
    key: web::Path<String>,
    http_req: HttpRequest,
    _session: Session<_SessionStore>,
) -> impl Responder {
    let ctx = Ctx::get(&http_req);
    ctx.log(&http_req);
    let result = async move {
        let mut conn = SampleConn::new();
        let mut note = _Note::find_by_key(&mut conn, &*key)
            .await
            .with_context(|| NotFound::new(&http_req))?;
        note.fetch_category(&mut conn).await?;
        note.fetch_tags(&mut conn).await?;

        let category = match note.category() {
            None => None,
            Some(v) => Some(v.name().to_owned()),
        };
        let date = Local::now().date_naive();
        let counter = _Counter::find_optional(&mut conn, (note.id(), date)).await?;
        let count = counter.map(|v| v.counter()).unwrap_or_default() + 1;

        let note_id = note.id();
        let cond = db_sample::cond_note_counter!((note_id=note_id) AND (date=date));
        conn.begin().await?;
        let mut update = _Counter::for_update(&mut conn);
        let _ = update.counter().add(1);
        _Counter::query()
            .cond(cond)
            .update(&mut conn, update)
            .await?;
        conn.commit().await?;

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
