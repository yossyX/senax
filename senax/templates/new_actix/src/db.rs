use anyhow::Result;
use std::{path::Path, sync::Arc};
use tokio::sync::mpsc;

pub async fn start(
    is_hot_deploy: bool,
    exit_tx: mpsc::Sender<i32>,
    db_guard: &Arc<mpsc::Sender<u8>>,
    db_dir: &Path,
    linker_port: &Option<String>,
    pw: &Option<String>,
) -> Result<()> {
    // Do not modify this line. (DbStart)
    db_session::start(
        is_hot_deploy,
        exit_tx.clone(),
        Arc::downgrade(db_guard),
        db_dir,
        linker_port,
        pw,
    )
    .await?;
    Ok(())
}

pub fn stop() {
    // Do not modify this line. (DbStop)
    db_session::stop();
}
@{-"\n"}@