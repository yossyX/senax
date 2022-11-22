use actix_web::web;

mod cache;
mod no_cache;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(cache::handler)
        .service(no_cache::handler);
}
