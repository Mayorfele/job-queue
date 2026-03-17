use async_trait::async_trait;
use serde_json::Value;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};
use crate::{JobHandler, JobError};

pub struct NotificationHandler;

#[async_trait]
impl JobHandler for NotificationHandler {
    async fn execute(&self, payload: Value) -> Result<(), JobError> {
        let recipient = payload.get("recipient")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JobError::fatal("missing recipient in payload"))?;

        let message = payload.get("message")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JobError::fatal("missing message in payload"))?;

        info!(recipient = %recipient, "sending notification");

        sleep(Duration::from_millis(400)).await;

        let failure_rate = std::env::var("NOTIFICATION_FAILURE_RATE")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.1);

        if rand::random::<f64>() < failure_rate {
            warn!(recipient = %recipient, "notification failed — retryable");
            return Err(JobError::retryable(
                format!("notification service unavailable for {}", recipient)
            ));
        }

        info!(recipient = %recipient, message = %message, "notification sent successfully");
        Ok(())
    }

    fn job_type(&self) -> &'static str { "notification" }
}