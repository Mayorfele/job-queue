mod routes;
mod handlers;
mod dashboard;

use actix_web::{web, App, HttpServer};
use std::sync::Arc;
use std::collections::HashMap;
use tracing::info;
use tracing_subscriber::EnvFilter;

use queue::store::JobStore;
use queue::scheduler::Scheduler;
use worker::pool::WorkerPool;
use worker::heartbeat::HeartbeatMonitor;
use jobs::JobHandler;
use jobs::order::OrderHandler;
use jobs::payment::PaymentHandler;
use jobs::notification::NotificationHandler;

pub struct AppState {
    pub store: JobStore,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive("info".parse().unwrap()))
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://jobs.db".to_string());

    let store = JobStore::new(&database_url).await
        .expect("failed to connect to database");

    store.migrate().await
        .expect("failed to run migrations");

    info!("database ready");

    let mut handlers: HashMap<String, Arc<dyn JobHandler>> = HashMap::new();
    handlers.insert("order".to_string(),        Arc::new(OrderHandler));
    handlers.insert("payment".to_string(),      Arc::new(PaymentHandler));
    handlers.insert("notification".to_string(), Arc::new(NotificationHandler));

    let worker_store = store.clone();
    let worker_handlers = handlers.clone();
    tokio::spawn(async move {
        WorkerPool::new(worker_store, worker_handlers, 3).start().await;
    });

    let monitor_store = store.clone();
    tokio::spawn(async move {
        HeartbeatMonitor::new(monitor_store, 30, 10).run().await;
    });

    let scheduler_store = store.clone();
    tokio::spawn(async move {
        let scheduler = Scheduler::new(scheduler_store, 60);
        loop {
            if let Err(e) = scheduler.prevent_starvation().await {
                tracing::error!("scheduler error: {}", e);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
        }
    });

    let state = web::Data::new(AppState { store });

    info!("api listening on 0.0.0.0:8080");

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .route("/sync/orders",       web::post().to(handlers::sync_order))
            .route("/jobs/orders",       web::post().to(handlers::enqueue_order))
            .route("/jobs/payments",     web::post().to(handlers::enqueue_payment))
            .route("/jobs/notifications",web::post().to(handlers::enqueue_notification))
            .route("/jobs/{id}",         web::get().to(handlers::get_job))
            .route("/jobs/{id}/requeue", web::post().to(handlers::requeue_job))
            .route("/jobs",              web::get().to(handlers::get_all_jobs))
            .route("/jobs/dead",         web::get().to(handlers::get_dead_jobs))
            .route("/dashboard",         web::get().to(dashboard::serve))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}