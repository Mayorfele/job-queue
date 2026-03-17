CREATE TABLE IF NOT EXISTS jobs (
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
);

CREATE TABLE IF NOT EXISTS dead_letter_queue (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL,
    job_type TEXT NOT NULL,
    payload TEXT NOT NULL,
    priority INTEGER NOT NULL,
    total_attempts INTEGER NOT NULL,
    last_error TEXT,
    died_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status);
CREATE INDEX IF NOT EXISTS idx_jobs_run_at ON jobs(run_at);
CREATE INDEX IF NOT EXISTS idx_jobs_priority ON jobs(priority DESC);