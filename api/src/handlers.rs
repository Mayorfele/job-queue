use actix_web::{web, HttpResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::time::{sleep, Duration};
use tracing::{info, warn};
use uuid::Uuid;

use queue::job::{Job, JobType, Priority};
use queue::store::JobStore;
use crate::AppState;

#[derive(Deserialize)]
pub struct OrderRequest {
    pub product_id: u32,
    pub quantity: u32,
    pub customer_email: String,
}

#[derive(Deserialize)]
pub struct PaymentRequest {
    pub order_id: String,
    pub amount: f64,
    pub currency: String,
}

#[derive(Deserialize)]
pub struct NotificationRequest {
    pub recipient: String,
    pub message: String,
}

pub async fn sync_order(body: web::Json<OrderRequest>) -> HttpResponse {
    info!("processing order synchronously — client is waiting");

    sleep(Duration::from_millis(2500)).await;

    let failure_rate = 0.3;
    if rand::random::<f64>() < failure_rate {
        warn!("sync order failed");
        return HttpResponse::InternalServerError().json(json!({
            "error": "order processing failed",
            "mode": "sync",
            "note": "client waited and got an error"
        }));
    }

    HttpResponse::Ok().json(json!({
        "order_id": Uuid::new_v4().to_string(),
        "product_id": body.product_id,
        "quantity": body.quantity,
        "status": "processed",
        "mode": "sync",
        "note": "client waited 2.5 seconds for this response"
    }))
}

pub async fn enqueue_order(
    state: web::Data<AppState>,
    body: web::Json<OrderRequest>,
) -> HttpResponse {
    let payload = json!({
        "order_id": Uuid::new_v4().to_string(),
        "product_id": body.product_id,
        "quantity": body.quantity,
        "customer_email": body.customer_email,
    });

    let job = Job::new(JobType::Order, payload, Priority::High, 3);
    let job_id = job.id.clone();

    match state.store.enqueue(&job).await {
        Ok(_) => HttpResponse::Accepted().json(json!({
            "job_id": job_id,
            "status": "pending",
            "mode": "async",
            "note": "job queued, processing in background"
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("failed to enqueue job: {}", e)
        })),
    }
}

pub async fn enqueue_payment(
    state: web::Data<AppState>,
    body: web::Json<PaymentRequest>,
) -> HttpResponse {
    let payload = json!({
        "payment_id": Uuid::new_v4().to_string(),
        "order_id": body.order_id,
        "amount": body.amount,
        "currency": body.currency,
    });

    let job = Job::new(JobType::Payment, payload, Priority::High, 5);
    let job_id = job.id.clone();

    match state.store.enqueue(&job).await {
        Ok(_) => HttpResponse::Accepted().json(json!({
            "job_id": job_id,
            "status": "pending",
            "mode": "async",
            "note": "payment queued, processing in background"
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("failed to enqueue job: {}", e)
        })),
    }
}

pub async fn enqueue_notification(
    state: web::Data<AppState>,
    body: web::Json<NotificationRequest>,
) -> HttpResponse {
    let payload = json!({
        "recipient": body.recipient,
        "message": body.message,
    });

    let job = Job::new(JobType::Notification, payload, Priority::Normal, 2);
    let job_id = job.id.clone();

    match state.store.enqueue(&job).await {
        Ok(_) => HttpResponse::Accepted().json(json!({
            "job_id": job_id,
            "status": "pending",
            "mode": "async",
            "note": "notification queued, processing in background"
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("failed to enqueue job: {}", e)
        })),
    }
}

pub async fn get_job(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let job_id = path.into_inner();

    match state.store.get_job(&job_id).await {
        Ok(Some(job)) => HttpResponse::Ok().json(job),
        Ok(None) => HttpResponse::NotFound().json(json!({
            "error": "job not found"
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("{}", e)
        })),
    }
}

pub async fn get_all_jobs(state: web::Data<AppState>) -> HttpResponse {
    match state.store.get_all_jobs().await {
        Ok(jobs) => HttpResponse::Ok().json(jobs),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("{}", e)
        })),
    }
}

pub async fn get_dead_jobs(state: web::Data<AppState>) -> HttpResponse {
    match state.store.get_dead_letter_jobs().await {
        Ok(jobs) => HttpResponse::Ok().json(jobs),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("{}", e)
        })),
    }
}

pub async fn requeue_job(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let job_id = path.into_inner();

    match state.store.requeue_from_dlq(&job_id).await {
        Ok(true) => HttpResponse::Ok().json(json!({
            "job_id": job_id,
            "status": "requeued"
        })),
        Ok(false) => HttpResponse::NotFound().json(json!({
            "error": "job not found in dead letter queue"
        })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "error": format!("{}", e)
        })),
    }
}