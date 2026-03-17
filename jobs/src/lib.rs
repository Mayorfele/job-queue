pub mod order;
pub mod payment;
pub mod notification;

use async_trait::async_trait;
use serde_json::Value;

#[async_trait]
pub trait JobHandler: Send + Sync {
    async fn execute(&self, payload: Value) -> Result<(), JobError>;
    fn job_type(&self) -> &'static str;
}

#[derive(Debug)]
pub struct JobError {
    pub message: String,
    pub retryable: bool,
}

impl JobError {
    pub fn retryable(message: impl Into<String>) -> Self {
        Self { message: message.into(), retryable: true }
    }

    pub fn fatal(message: impl Into<String>) -> Self {
        Self { message: message.into(), retryable: false }
    }
}

impl std::fmt::Display for JobError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}