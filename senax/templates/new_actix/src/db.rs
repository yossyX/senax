use anyhow::Result;
use rand::RngCore;
use std::{path::Path, sync::Arc};
use tokio::sync::mpsc;

fn init() {
    @%- if session %@
    db_session_session::init();
    @%- endif %@
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
    init();
    let mut uuid_node = [0u8; 6];
    rand::rng().fill_bytes(&mut uuid_node);
    let uuid_node = Some(uuid_node);
    @%- if session %@
    db_session::start(
        is_hot_deploy,
        exit_tx.clone(),
        Arc::downgrade(db_guard),
        db_dir,
        linker_port,
        pw,
        &uuid_node,
    )
    .await?;
    @%- endif %@
    // Do not modify this line. (DbStart)
    Ok(())
}

#[cfg(test)]
pub async fn start_test() -> Result<Vec<tokio::sync::MutexGuard<'static, u8>>> {
    init();
    let mut guard = Vec::new();
    @%- if session %@
    guard.push(db_session::start_test().await?);
    @%- endif %@
    // Do not modify this line. (DbStartTest)
    Ok(guard)
}

pub fn stop() {
    @%- if session %@
    db_session::stop();
    @%- endif %@
    // Do not modify this line. (DbStop)
}
@{-"\n"}@