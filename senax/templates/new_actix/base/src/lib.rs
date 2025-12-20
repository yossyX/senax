@% if non_snake_case -%@
#![allow(non_snake_case)]
@% endif -%@
#[macro_use]
extern crate log;

pub mod auth;
pub mod auto_api;
pub mod common;
pub mod context;
pub mod db;
pub mod maybe_undefined;
pub mod response;
pub mod validator;

pub use maybe_undefined::MaybeUndefined;

pub async fn start() -> anyhow::Result<()> {
    #[cfg(feature = "v8")]
    {
        let platform = v8::Platform::new(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    }
    Ok(())
}

pub async fn end() -> anyhow::Result<()> {
    #[cfg(feature = "v8")]
    {
        unsafe {
            v8::V8::dispose();
        }
        v8::V8::dispose_platform();
    }
    Ok(())
}
@{-"\n"}@