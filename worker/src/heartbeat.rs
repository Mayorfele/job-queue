use tokio::time::{sleep, Duration};
use tracing::info;
use queue::store::JobStore;

pub struct HeartbeatMonitor {
    store: JobStore,
    timeout_secs: i64,
    interval_secs: u64,
}

impl HeartbeatMonitor {
    pub fn new(store: JobStore, timeout_secs: i64, interval_secs: u64) -> Self {
        Self { store, timeout_secs, interval_secs }
    }

    pub async fn run(self) {
        info!(
            timeout_secs = self.timeout_secs,
            "heartbeat monitor started"
        );

        loop {
            match self.store.reclaim_abandoned(self.timeout_secs).await {
                Ok(count) if count > 0 => {
                    info!(count = count, "reclaimed abandoned jobs");
                }
                Err(e) => {
                    tracing::error!("heartbeat monitor error: {}", e);
                }
                _ => {}
            }

            sleep(Duration::from_secs(self.interval_secs)).await;
        }
    }
}