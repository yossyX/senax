@% if non_snake_case -%@
#![allow(non_snake_case)]
@% endif -%@
pub mod events;
#[allow(clippy::module_inception)]
pub mod models;
pub mod services;
pub mod use_cases;
pub mod value_objects;
@{-"\n"}@