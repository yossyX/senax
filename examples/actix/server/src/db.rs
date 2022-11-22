use actix::ArbiterHandle;
use anyhow::Result;
use std::{path::Path, sync::Arc};
use tokio::sync::mpsc;

pub async fn start(
    handle: &ArbiterHandle,
    is_hot_deploy: bool,
    exit_tx: mpsc::Sender<i32>,
    db_guard: &Arc<mpsc::Sender<u8>>,
    db_dir: &Path,
    linker_tcp_port: &Option<String>,
    linker_unix_port: &Option<String>,
) -> Result<()> {
    db_sample::start(
        handle,
        is_hot_deploy,
        exit_tx.clone(),
        Arc::downgrade(db_guard),
        db_dir,
        linker_tcp_port,
        linker_unix_port,
    )
    .await?;
    Ok(())
}

pub fn stop() {
    db_sample::stop();
}
