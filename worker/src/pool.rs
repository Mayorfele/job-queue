use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::info;
use queue::store::JobStore;
use queue::retry::RetryPolicy;
use jobs::JobHandler;
use crate::executor::Executor;

pub struct WorkerPool {
    store: JobStore,
    handlers: Arc<HashMap<String, Arc<dyn JobHandler>>>,
    num_workers: usize,
}

impl WorkerPool {
    pub fn new(
        store: JobStore,
        handlers: HashMap<String, Arc<dyn JobHandler>>,
        num_workers: usize,
    ) -> Self {
        Self {
            store,
            handlers: Arc::new(handlers),
            num_workers,
        }
    }

    pub async fn start(self) {
        info!("starting worker pool with {} workers", self.num_workers);

        let mut handles = Vec::new();

        for worker_id in 0..self.num_workers {
            let store = self.store.clone();
            let handlers = self.handlers.clone();

            let handle = tokio::spawn(async move {
                info!(worker_id = worker_id, "worker started");
                let executor = Executor::new(
                    store,
                    handlers,
                    RetryPolicy::default(),
                );

                loop {
                    let found = executor.run_once().await;
                    if !found {
                        sleep(Duration::from_millis(500)).await;
                    }
                }
            });

            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.await;
        }
    }
}