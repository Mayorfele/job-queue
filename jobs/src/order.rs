use async_trait::async_trait;
use serde_json::Value;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};
use crate::{JobHandler, JobError};

pub struct OrderHandler;

#[async_trait]
impl JobHandler for OrderHandler {
    async fn execute(&self, payload: Value) -> Result<(), JobError> {
        let order_id = payload.get("order_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| JobError::fatal("missing order_id in payload"))?;

        let product_id = payload.get("product_id")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| JobError::fatal("missing product_id in payload"))?;

        info!(order_id = %order_id, product_id = %product_id, "processing order");

        sleep(Duration::from_millis(800)).await;

        let failure_rate = std::env::var("ORDER_FAILURE_RATE")
            .ok()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.3);

        if rand::random::<f64>() < failure_rate {
            warn!(order_id = %order_id, "order processing failed — retryable");
            return Err(JobError::retryable(
                format!("payment gateway timeout for order {}", order_id)
            ));
        }

        info!(order_id = %order_id, "order processed successfully");
        Ok(())
    }

    fn job_type(&self) -> &'static str { "order" }
}