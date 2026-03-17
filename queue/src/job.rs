use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Retrying,
    Dead,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Pending   => "pending",
            JobStatus::Running   => "running",
            JobStatus::Completed => "completed",
            JobStatus::Failed    => "failed",
            JobStatus::Retrying  => "retrying",
            JobStatus::Dead      => "dead",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "pending"   => JobStatus::Pending,
            "running"   => JobStatus::Running,
            "completed" => JobStatus::Completed,
            "failed"    => JobStatus::Failed,
            "retrying"  => JobStatus::Retrying,
            "dead"      => JobStatus::Dead,
            _           => JobStatus::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    High   = 3,
    Normal = 2,
    Low    = 1,
}

impl Priority {
    pub fn as_i32(&self) -> i32 {
        match self {
            Priority::High   => 3,
            Priority::Normal => 2,
            Priority::Low    => 1,
        }
    }

    pub fn from_i32(n: i32) -> Self {
        match n {
            3 => Priority::High,
            2 => Priority::Normal,
            _ => Priority::Low,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JobType {
    Order,
    Payment,
    Notification,
}

impl JobType {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobType::Order        => "order",
            JobType::Payment      => "payment",
            JobType::Notification => "notification",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "order"        => JobType::Order,
            "payment"      => JobType::Payment,
            "notification" => JobType::Notification,
            _              => JobType::Order,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub job_type: JobType,
    pub payload: serde_json::Value,
    pub status: JobStatus,
    pub priority: Priority,
    pub attempt: i32,
    pub max_attempts: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub run_at: DateTime<Utc>,
    pub claimed_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

impl Job {
    pub fn new(
        job_type: JobType,
        payload: serde_json::Value,
        priority: Priority,
        max_attempts: i32,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            job_type,
            payload,
            status: JobStatus::Pending,
            priority,
            attempt: 0,
            max_attempts,
            created_at: now,
            updated_at: now,
            run_at: now,
            claimed_at: None,
            completed_at: None,
            last_heartbeat: None,
            error: None,
        }
    }

    pub fn is_exhausted(&self) -> bool {
        self.attempt >= self.max_attempts
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetterJob {
    pub id: String,
    pub job_id: String,
    pub job_type: String,
    pub payload: serde_json::Value,
    pub priority: i32,
    pub total_attempts: i32,
    pub last_error: Option<String>,
    pub died_at: DateTime<Utc>,
}