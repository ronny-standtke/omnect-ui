use log::{error, info};
use std::sync::Arc;
use tokio::{
    sync::{RwLock, broadcast},
    task::AbortHandle,
};

static SERVER_RESTART_TX: std::sync::OnceLock<broadcast::Sender<()>> = std::sync::OnceLock::new();

static ROLLBACK_TIMER: std::sync::OnceLock<Arc<RwLock<Option<AbortHandle>>>> =
    std::sync::OnceLock::new();

pub fn trigger_server_restart() {
    if let Some(tx) = SERVER_RESTART_TX.get() {
        if let Err(e) = tx.send(()) {
            error!("Failed to trigger server restart: {e:#}");
        }
    }
}

pub async fn cancel_rollback_timer() {
    if let Some(timer_handle) = ROLLBACK_TIMER.get() {
        if let Some(handle) = timer_handle.write().await.take() {
            handle.abort();
            info!("Rollback timer cancelled - network change confirmed");
        }
    }
}

pub async fn set_rollback_timer(handle: AbortHandle) {
    if let Some(timer_handle) = ROLLBACK_TIMER.get() {
        *timer_handle.write().await = Some(handle);
    }
}

pub fn set_server_restart_tx(tx: broadcast::Sender<()>) -> Result<(), broadcast::Sender<()>> {
    SERVER_RESTART_TX.set(tx)
}

pub fn set_rollback_timer_handle(
    handle: Arc<RwLock<Option<AbortHandle>>>,
) -> Result<(), Arc<RwLock<Option<AbortHandle>>>> {
    ROLLBACK_TIMER.set(handle)
}
