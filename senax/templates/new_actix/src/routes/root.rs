use actix_web::web;

mod index;

pub fn route_config(cfg: &mut web::ServiceConfig) {
    cfg.service(index::handler);
}
@{-"\n"}@