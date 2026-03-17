use crate::store::JobStore;
use crate::job::Job;
use chrono::Utc;
use tracing::info;

pub struct Scheduler {
    store: JobStore,
    starvation_threshold_secs: i64,
}

impl Scheduler {
    pub fn new(store: JobStore, starvation_threshold_secs: i64) -> Self {
        Self {
            store,
            starvation_threshold_secs,
        }
    }

    pub async fn prevent_starvation(&self) -> Result<(), sqlx::Error> {
        let cutoff = (Utc::now()
            - chrono::Duration::seconds(self.starvation_threshold_secs))
            .to_rfc3339();

        let result = sqlx::query(
            "UPDATE jobs SET
                priority = MIN(priority + 1, 3),
                updated_at = ?
            WHERE status IN ('pending', 'retrying')
            AND priority < 3
            AND created_at < ?"
        )
        .bind(Utc::now().to_rfc3339())
        .bind(&cutoff)
        .execute(self.store.pool())
        .await?;

        let count = result.rows_affected();
        if count > 0 {
            info!(count = count, "promoted starving jobs");
        }

        Ok(())
    }
}