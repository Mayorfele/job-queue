use chrono::{DateTime, Utc};

pub struct RetryPolicy {
    pub base_delay_secs: u64,
    pub max_delay_secs: u64,
}

impl RetryPolicy {
    pub fn default() -> Self {
        Self {
            base_delay_secs: 2,
            max_delay_secs: 60,
        }
    }

    pub fn next_run_at(&self, attempt: i32) -> DateTime<Utc> {
        let exp_delay = self.base_delay_secs * 2u64.pow(attempt as u32);
        let capped = exp_delay.min(self.max_delay_secs);
        let jitter = rand::random::<u64>() % (capped / 2).max(1);
        let delay = capped + jitter;

        Utc::now() + chrono::Duration::seconds(delay as i64)
    }

    pub fn should_retry(&self, attempt: i32, max_attempts: i32) -> bool {
        attempt < max_attempts
    }
}