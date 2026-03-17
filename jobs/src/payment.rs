use async_trait::async_trait;
use serde_json::Value;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};
use crate::{JobHandler, JobError};

pub struct PaymentHandler;

#[async_trait]
impl JobHandler for PaymentHandler {
    async fn execute(&self, payload: Value) -> Result<(), JobError> {
        let payment_id = payload.get("payment_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JobError::fatal("missing payment_id in payload"))?;

        let amount = payload.get("amount")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| JobError::fatal("missing amount in payload"))?;

        info!(payment_id = %payment_id, amount = %amount, "processing payment");

        sleep(Duration::from_millis(1200)).await;

        let failure_rate = std::env::var("PAYMENT_FAILURE_RATE")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.2);

        if rand::random::<f64>() < failure_rate {
            warn!(payment_id = %payment_id, "payment processing failed — retryable");
            return Err(JobError::retryable(
                format!("payment processor unavailable for payment {}", payment_id)
            ));
        }

        info!(payment_id = %payment_id, amount = %amount, "payment processed successfully");
        Ok(())
    }

    fn job_type(&self) -> &'static str { "payment" }
}