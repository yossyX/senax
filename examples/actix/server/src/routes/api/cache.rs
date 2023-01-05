use crate::context::Ctx;
use crate::response::*;
use actix_web::{get, web, HttpRequest, Responder};
#[allow(unused_imports)]
use anyhow::{Context as _, Result};
use chrono::Local;
use db_sample::misc::ForUpdateTr;
use db_sample::note::counter::*;
use db_sample::note::note::*;
#[allow(unused_imports)]
use db_sample::DbConn as SampleConn;
use db_session::session::session::_SessionStore;
use senax_actix_session::Session;
use serde::Serialize;
#[allow(unused_imports)]
use tracing::trace_span;

const SESSION_KEY: &str = "count";

#[derive(Serialize)]
pub struct Response {
    pub id: _NoteId,
    pub category: Option<String>,
    pub article: String,
    pub tags: Vec<String>,
    pub count: u64,
    pub session_count: u64,
}

#[get("/cache/{key}")]
async fn handler(
    key: web::Path<String>,
    http_req: HttpRequest,
    session: Session<_SessionStore>,
) -> impl Responder {
    let ctx = Ctx::get(&http_req);
    ctx.log(&http_req);
    let result = async move {
        let session_count = session
            .update(|s| {
                let v: Option<u64> = s.get_from_base(SESSION_KEY)?;
                match v {
                    Some(mut v) => {
                        v += 1;
                        s.insert_to_base(SESSION_KEY, v)?;
                        Ok(v)
                    }
                    None => {
                        s.insert_to_base(SESSION_KEY, 1)?;
                        Ok(1)
                    }
                }
            })
            .await?;

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
        let counter = _Counter::find_optional_from_cache(&conn, (note.id(), date)).await?;
        let count = counter.map(|v| v.counter()).unwrap_or_default() + 1;
        let mut counter_for_update = _CounterFactory {
            note_id: note.id(),
            date,
            counter: 0,
        }
        .create(&conn);
        let _ = counter_for_update.counter().add(1);
        counter_for_update._upsert();
        _Counter::save_delayed(&mut conn, counter_for_update).await?;

        Ok(Response {
            id: note.id(),
            category,
            article: note.content().to_string(),
            tags: note.tags().iter().map(|v| v.name().to_string()).collect(),
            count,
            session_count,
        })
    }
    .await;
    json_response(result, &ctx)
}
