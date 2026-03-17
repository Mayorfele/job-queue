use sqlx::{SqlitePool, Row};
use chrono::{DateTime, Utc};
use tracing::{info, error};
use crate::job::{Job, JobType, JobStatus, Priority, DeadLetterJob};

#[derive(Clone)]
pub struct JobStore {
    pool: SqlitePool,
}

impl JobStore {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect(database_url).await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY,
                job_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                priority INTEGER NOT NULL DEFAULT 1,
                attempt INTEGER NOT NULL DEFAULT 0,
                max_attempts INTEGER NOT NULL DEFAULT 3,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                run_at TEXT NOT NULL,
                claimed_at TEXT,
                completed_at TEXT,
                last_heartbeat TEXT,
                error TEXT
            )"
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS dead_letter_queue (
                id TEXT PRIMARY KEY,
                job_id TEXT NOT NULL,
                job_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                priority INTEGER NOT NULL,
                total_attempts INTEGER NOT NULL,
                last_error TEXT,
                died_at TEXT NOT NULL
            )"
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status)"
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_jobs_run_at ON jobs(run_at)"
        )
        .execute(&self.pool)
        .await?;

        info!("database migrations complete");
        Ok(())
    }

    pub async fn enqueue(&self, job: &Job) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO jobs
            (id, job_type, payload, status, priority, attempt, max_attempts,
             created_at, updated_at, run_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&job.id)
        .bind(job.job_type.as_str())
        .bind(job.payload.to_string())
        .bind(job.status.as_str())
        .bind(job.priority.as_i32())
        .bind(job.attempt)
        .bind(job.max_attempts)
        .bind(job.created_at.to_rfc3339())
        .bind(job.updated_at.to_rfc3339())
        .bind(job.run_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        info!(job_id = %job.id, job_type = %job.job_type.as_str(), "job enqueued");
        Ok(())
    }

    pub async fn claim_next(&self) -> Result<Option<Job>, sqlx::Error> {
        let now = Utc::now().to_rfc3339();

        let row = sqlx::query(
            "UPDATE jobs SET
                status = 'running',
                claimed_at = ?,
                last_heartbeat = ?,
                updated_at = ?
            WHERE id = (
                SELECT id FROM jobs
                WHERE status IN ('pending', 'retrying')
                AND run_at <= ?
                ORDER BY priority DESC, created_at ASC
                LIMIT 1
            )
            RETURNING *"
        )
        .bind(&now)
        .bind(&now)
        .bind(&now)
        .bind(&now)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(Some(row_to_job(&r))),
            None => Ok(None),
        }
    }

    pub async fn mark_completed(&self, job_id: &str) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE jobs SET
                status = 'completed',
                completed_at = ?,
                updated_at = ?
            WHERE id = ?"
        )
        .bind(&now)
        .bind(&now)
        .bind(job_id)
        .execute(&self.pool)
        .await?;

        info!(job_id = %job_id, "job completed");
        Ok(())
    }

    pub async fn mark_failed(
        &self,
        job_id: &str,
        error: &str,
        retry_at: Option<DateTime<Utc>>,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();

        match retry_at {
            Some(run_at) => {
                sqlx::query(
                    "UPDATE jobs SET
                        status = 'retrying',
                        attempt = attempt + 1,
                        error = ?,
                        run_at = ?,
                        updated_at = ?
                    WHERE id = ?"
                )
                .bind(error)
                .bind(run_at.to_rfc3339())
                .bind(&now)
                .bind(job_id)
                .execute(&self.pool)
                .await?;

                info!(job_id = %job_id, retry_at = %run_at, "job scheduled for retry");
            }
            None => {
                sqlx::query(
                    "UPDATE jobs SET
                        status = 'dead',
                        attempt = attempt + 1,
                        error = ?,
                        updated_at = ?
                    WHERE id = ?"
                )
                .bind(error)
                .bind(&now)
                .bind(job_id)
                .execute(&self.pool)
                .await?;

                self.move_to_dlq(job_id).await?;
                error!(job_id = %job_id, "job exhausted all retries — moved to DLQ");
            }
        }

        Ok(())
    }

    pub async fn update_heartbeat(&self, job_id: &str) -> Result<(), sqlx::Error> {
        let now = Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE jobs SET last_heartbeat = ? WHERE id = ?"
        )
        .bind(&now)
        .bind(job_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn reclaim_abandoned(&self, timeout_secs: i64) -> Result<u64, sqlx::Error> {
        let cutoff = (Utc::now() - chrono::Duration::seconds(timeout_secs)).to_rfc3339();

        let result = sqlx::query(
            "UPDATE jobs SET
                status = 'pending',
                claimed_at = NULL,
                last_heartbeat = NULL,
                updated_at = ?
            WHERE status = 'running'
            AND last_heartbeat < ?"
        )
        .bind(Utc::now().to_rfc3339())
        .bind(&cutoff)
        .execute(&self.pool)
        .await?;

        let count = result.rows_affected();
        if count > 0 {
            info!(count = count, "reclaimed abandoned jobs");
        }
        Ok(count)
    }

    async fn move_to_dlq(&self, job_id: &str) -> Result<(), sqlx::Error> {
        let row = sqlx::query("SELECT * FROM jobs WHERE id = ?")
            .bind(job_id)
            .fetch_one(&self.pool)
            .await?;

        let dlq_id = uuid::Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO dead_letter_queue
            (id, job_id, job_type, payload, priority, total_attempts, last_error, died_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&dlq_id)
        .bind(job_id)
        .bind(row.get::<String, _>("job_type"))
        .bind(row.get::<String, _>("payload"))
        .bind(row.get::<i32, _>("priority"))
        .bind(row.get::<i32, _>("attempt"))
        .bind(row.get::<Option<String>, _>("error"))
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_job(&self, job_id: &str) -> Result<Option<Job>, sqlx::Error> {
        let row = sqlx::query("SELECT * FROM jobs WHERE id = ?")
            .bind(job_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| row_to_job(&r)))
    }

    pub async fn get_all_jobs(&self) -> Result<Vec<Job>, sqlx::Error> {
        let rows = sqlx::query("SELECT * FROM jobs ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.iter().map(row_to_job).collect())
    }

    pub async fn get_dead_letter_jobs(&self) -> Result<Vec<DeadLetterJob>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT * FROM dead_letter_queue ORDER BY died_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(|r| DeadLetterJob {
            id: r.get("id"),
            job_id: r.get("job_id"),
            job_type: r.get("job_type"),
            payload: serde_json::from_str(&r.get::<String, _>("payload"))
                .unwrap_or(serde_json::Value::Null),
            priority: r.get("priority"),
            total_attempts: r.get("total_attempts"),
            last_error: r.get("last_error"),
            died_at: DateTime::parse_from_rfc3339(&r.get::<String, _>("died_at"))
                .unwrap()
                .with_timezone(&Utc),
        }).collect())
    }
    
    pub fn pool(&self) -> &SqlitePool {
    &self.pool
    }

    pub async fn requeue_from_dlq(&self, job_id: &str) -> Result<bool, sqlx::Error> {
        let row = sqlx::query(
            "SELECT * FROM dead_letter_queue WHERE job_id = ?"
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            None => Ok(false),
            Some(r) => {
                let now = Utc::now().to_rfc3339();
                sqlx::query(
                    "UPDATE jobs SET
                        status = 'pending',
                        attempt = 0,
                        error = NULL,
                        run_at = ?,
                        updated_at = ?
                    WHERE id = ?"
                )
                .bind(&now)
                .bind(&now)
                .bind(job_id)
                .execute(&self.pool)
                .await?;

                sqlx::query(
                    "DELETE FROM dead_letter_queue WHERE job_id = ?"
                )
                .bind(job_id)
                .execute(&self.pool)
                .await?;

                info!(job_id = %job_id, "job requeued from DLQ");
                Ok(true)
            }
        }
    }
}

fn row_to_job(r: &sqlx::sqlite::SqliteRow) -> Job {
    let parse_dt = |s: String| {
        DateTime::parse_from_rfc3339(&s)
            .unwrap()
            .with_timezone(&Utc)
    };

    let parse_opt_dt = |s: Option<String>| {
        s.map(|v| DateTime::parse_from_rfc3339(&v).unwrap().with_timezone(&Utc))
    };

    Job {
        id: r.get("id"),
        job_type: JobType::from_str(&r.get::<String, _>("job_type")),
        payload: serde_json::from_str(&r.get::<String, _>("payload"))
            .unwrap_or(serde_json::Value::Null),
        status: JobStatus::from_str(&r.get::<String, _>("status")),
        priority: Priority::from_i32(r.get::<i32, _>("priority")),
        attempt: r.get("attempt"),
        max_attempts: r.get("max_attempts"),
        created_at: parse_dt(r.get("created_at")),
        updated_at: parse_dt(r.get("updated_at")),
        run_at: parse_dt(r.get("run_at")),
        claimed_at: parse_opt_dt(r.get("claimed_at")),
        completed_at: parse_opt_dt(r.get("completed_at")),
        last_heartbeat: parse_opt_dt(r.get("last_heartbeat")),
        error: r.get("error"),
    }
}