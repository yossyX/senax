use anyhow::Result;
use rand::RngCore;
use std::{path::Path, sync::Arc};
use tokio::sync::mpsc;

pub fn init() {
    // Do not modify this line. (DbInit)
}

pub async fn start(
    is_hot_deploy: bool,
    exit_tx: mpsc::Sender<i32>,
    db_guard: &Arc<mpsc::Sender<u8>>,
    db_dir: &Path,
    linker_port: &Option<String>,
    pw: &Option<String>,
) -> Result<()> {
    let mut uuid_node = [0u8; 6];
    rand::rng().fill_bytes(&mut uuid_node);
    let uuid_node = Some(uuid_node);
    // Do not modify this line. (DbStart)
    Ok(())
}

#[cfg(test)]
pub async fn start_test() -> Result<Vec<tokio::sync::MutexGuard<'static, u8>>> {
    init();
    let mut guard = Vec::new();
    // Do not modify this line. (DbStartTest)
    Ok(guard)
}

pub fn stop() {
    // Do not modify this line. (DbStop)
}

#[rustfmt::skip]
pub async fn migrate(db: Option<&str>, use_test: bool, clean: bool, ignore_missing: bool, remove_missing: bool) -> Result<()> {
    let mut join_set = tokio::task::JoinSet::new();
    // Do not modify this line. (migrate)
    let mut error = None;
    while let Some(res) = join_set.join_next().await {
        if let Err(e) = res? 
            && let Some(e) = error.replace(e) {
                log::error!("{}", e);
            }
    }
    if let Some(e) = error {
        return Err(e);
    }
    Ok(())
}

pub async fn seed(_db: Option<&str>, _use_test: bool) -> Result<()> {
    // Do not modify this line. (seed)
    Ok(())
}

#[rustfmt::skip]
pub async fn check(use_test: bool) -> Result<()> {
    tokio::try_join!(
        // Do not modify this line. (check)
    )?;
    Ok(())
}
@{-"\n"}@