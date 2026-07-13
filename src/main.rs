use std::env;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use warp::Filter;

mod api;
mod auth;
mod config;
mod database;
mod logging;
mod models;
mod pdf;
mod qr;
mod services;
mod utils;
use gwl_logger::{Logger, LoggerConfig};
use logging::TracingToGwlLayer;

use config::AppConfig;
use database::DatabasePool;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    init_tracing();

    info!("🌱 Starting Code Terroir API server...");

    // Load configuration
    let config = AppConfig::load().await?;
    info!("✅ Configuration loaded");

    // Initialize database pool
    let db_pool = DatabasePool::new(&config.database_url).await?;
    info!("✅ Database connection established");

    // Run migrations
    sqlx::migrate!("./migrations").run(&db_pool.pool).await?;
    info!("✅ Database migrations applied");

    // Initialize Redis connection
    let redis_client = redis::Client::open(config.redis_url.clone())?;
    let redis_conn = redis_client.get_connection_manager().await?;
    info!("✅ Redis connection established");

    // Create shared application state
    let app_state = api::AppState {
        db: db_pool,
        redis: redis_conn,
        config: config.clone(),
    };

    // Build API routes
    let api_routes = api::routes(app_state).with(warp::log("code_terroir::api"));

    // Health check route
    let health = warp::path("health").and(warp::get()).map(|| {
        warp::reply::json(&serde_json::json!({
            "status": "healthy",
            "service": "code-terroir-api",
            "version": env!("CARGO_PKG_VERSION")
        }))
    });

    // Combine all routes
    let routes = health
        .or(api_routes)
        .with(
            warp::cors()
                .allow_any_origin()
                .allow_headers(vec!["authorization", "content-type"])
                .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "PATCH"]),
        )
        .with(warp::compression::gzip())
        .recover(api::handle_rejection);

    let port = config.server_port;
    let addr = ([0, 0, 0, 0], port);

    info!(
        "🚀 Code Terroir API server started on http://0.0.0.0:{}",
        port
    );
    info!(
        "📋 Health check available at http://0.0.0.0:{}/health",
        port
    );

    warp::serve(routes).run(addr).await;

    Ok(())
}
/*
fn init_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            "code_terroir=debug,warp=info,sqlx=info,tower=info".into()
        });

    let formatting_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(formatting_layer)
        .init();

    info!("🔍 Tracing initialized");
}
*/
fn init_tracing() {
    // ✅ 1. Initialiser gwl_logger AVANT tout log
    let cfg = LoggerConfig::default();
    let _ = Logger::init_global(cfg);

    // ✅ 2. Config tracing
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "code_terroir=debug,warp=info,sqlx=info,tower=info".into());

    let formatting_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    // ✅ 3. Brancher la Layer
    tracing_subscriber::registry()
        .with(env_filter)
        .with(formatting_layer)
        .with(TracingToGwlLayer)
        .init();
}
