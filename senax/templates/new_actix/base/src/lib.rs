@% if non_snake_case -%@
#![allow(non_snake_case)]
@% endif -%@
#[macro_use]
extern crate log;

use once_cell::sync::OnceCell;
use std::sync::{Arc, Weak};
use tokio::sync::mpsc;

pub mod auth;
pub mod auto_api;
pub mod common;
pub mod context;
pub mod db;
pub mod response;
pub mod validator;

pub static SHUTDOWN_GUARD: OnceCell<Weak<mpsc::Sender<u8>>> = OnceCell::new();

pub fn get_shutdown_guard() -> Option<Arc<mpsc::Sender<u8>>> {
    SHUTDOWN_GUARD.wait().upgrade()
}
@{-"\n"}@