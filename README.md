# job-queue

A persistent job queue in Rust with priority scheduling, exponential backoff, poison pill detection, and a live dashboard.

---

## Architecture

```
POST /jobs/*
      ↓
API — writes job to SQLite, returns immediately
      ↓
SQLite (jobs table + dead_letter_queue table)
      ↓
Worker Pool (3 concurrent workers)
  ├── claims jobs atomically
  ├── dispatches to handler by job_type
  ├── sends heartbeats while processing
  └── schedules retries or moves to DLQ on failure
      ↓
Heartbeat Monitor — reclaims abandoned jobs
Starvation Scheduler — promotes starving low-priority jobs
```

---

## Job State Machine

```
Pending → Running → Completed
                 ↘ Retrying → Running
                           ↘ Dead → DLQ
```

Dead jobs are inspectable and requeuable via the dashboard or API.

---

## Features

**Priority queuing** — High / Normal / Low. Workers claim highest priority first, oldest within the same tier.

**Exponential backoff with jitter** — `base * 2^attempt + jitter`. Prevents thundering herd on retry storms.

**Poison pill detection** — jobs exceeding `max_attempts` move to the dead letter queue automatically.

**Heartbeat monitor** — running jobs with stale heartbeats are reclaimed and re-queued. Handles crashed workers.

**Starvation prevention** — jobs waiting beyond threshold get promoted one priority level. Runs every 15 seconds.

**Sync vs async comparison** — `POST /sync/orders` blocks for 2.5s. `POST /jobs/orders` returns in ~10ms. Same operation, visible latency difference under load.

---

## Endpoints

```
POST /jobs/orders          → enqueue order job       (Priority: High,   max 3 attempts)
POST /jobs/payments        → enqueue payment job     (Priority: High,   max 5 attempts)
POST /jobs/notifications   → enqueue notification    (Priority: Normal, max 2 attempts)
POST /sync/orders          → process synchronously   (comparison endpoint)

GET  /jobs                 → all jobs
GET  /jobs/:id             → single job status
GET  /jobs/dead            → dead letter queue
POST /jobs/:id/requeue     → requeue from DLQ

GET  /dashboard            → live job timeline UI
```

---

## Running

```bash
touch jobs.db
DATABASE_URL="sqlite:///absolute/path/to/jobs.db" cargo run -p api
```

**Tune failure rates:**
```bash
ORDER_FAILURE_RATE=0.4 \
PAYMENT_FAILURE_RATE=0.3 \
NOTIFICATION_FAILURE_RATE=0.2 \
DATABASE_URL="sqlite:///absolute/path/to/jobs.db" cargo run -p api
```

**Force all jobs to DLQ:**
```bash
ORDER_FAILURE_RATE=1.0 DATABASE_URL="..." cargo run -p api
```

---

## Sync vs Async — Observed

```bash
# 10 concurrent requests
sync:  2563ms total
async:  1664ms total — returns immediately, workers process in background
```

---

## Stack

Actix-web, SQLx, SQLite, Tokio, Serde, Chrono, UUID, Tracing