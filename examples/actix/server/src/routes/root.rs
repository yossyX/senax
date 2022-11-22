use actix_web::web;

mod index;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(index::handler);
}
