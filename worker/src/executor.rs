use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn, error};
use queue::store::JobStore;
use queue::retry::RetryPolicy;
use jobs::JobHandler;

pub struct Executor {
    store: JobStore,
    handlers: Arc<HashMap<String, Arc<dyn JobHandler>>>,
    retry_policy: RetryPolicy,
}

impl Executor {
    pub fn new(
        store: JobStore,
        handlers: Arc<HashMap<String, Arc<dyn JobHandler>>>,
        retry_policy: RetryPolicy,
    ) -> Self {
        Self { store, handlers, retry_policy }
    }

    pub async fn run_once(&self) -> bool {
        let job = match self.store.claim_next().await {
            Ok(Some(job)) => job,
            Ok(None) => return false,
            Err(e) => {
                error!("failed to claim job: {}", e);
                return false;
            }
        };

        let job_id = job.id.clone();
        let job_type = job.job_type.as_str().to_string();

        info!(job_id = %job_id, job_type = %job_type, attempt = job.attempt, "claimed job");

        let handler = match self.handlers.get(&job_type) {
            Some(h) => h.clone(),
            None => {
                error!(job_id = %job_id, job_type = %job_type, "no handler registered");
                let _ = self.store.mark_failed(
                    &job_id,
                    &format!("no handler for job type: {}", job_type),
                    None,
                ).await;
                return true;
            }
        };

        let store = self.store.clone();
        let job_id_hb = job_id.clone();
        let heartbeat_handle = tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(3)
            );
            loop {
                interval.tick().await;
                if let Err(e) = store.update_heartbeat(&job_id_hb).await {
                    error!("heartbeat failed: {}", e);
                    break;
                }
            }
        });

        let result = handler.execute(job.payload.clone()).await;
        heartbeat_handle.abort();

        match result {
            Ok(_) => {
                if let Err(e) = self.store.mark_completed(&job_id).await {
                    error!(job_id = %job_id, "failed to mark completed: {}", e);
                }
                info!(job_id = %job_id, "job completed");
            }
            Err(job_error) => {
                warn!(
                    job_id = %job_id,
                    error = %job_error.message,
                    retryable = job_error.retryable,
                    attempt = job.attempt,
                    max_attempts = job.max_attempts,
                    "job failed"
                );

                let exhausted = job.is_exhausted();
                let retry_at = if job_error.retryable && !exhausted {
                    Some(self.retry_policy.next_run_at(job.attempt))
                } else {
                    None
                };

                if let Err(e) = self.store.mark_failed(
                    &job_id,
                    &job_error.message,
                    retry_at,
                ).await {
                    error!(job_id = %job_id, "failed to mark failed: {}", e);
                }
            }
        }

        true
    }
}