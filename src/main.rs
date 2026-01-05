mod config;
mod db;
mod handlers;

use crate::config::Config;
use crate::handlers::heartbeat;
use axum::Router;
use axum::routing::post;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let config = Config::from_env();

    let pool = db::create_pool(config.database_url())
        .await
        .expect("Failed to create database pool");
    db::run_migrations(&pool)
        .await
        .expect("Failed to run migrations");

    let app = Router::new()
        .route("/beat", post(heartbeat::beat))
        .layer(TraceLayer::new_for_http())
        .with_state(pool);

    let addr = format!("0.0.0.0:{}", config.port());
    tracing::info!("Server running on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Server error");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("Received Ctrl+C, shutting down"),
        _ = terminate => tracing::info!("Received SIGTERM, shutting down"),
    }
}
