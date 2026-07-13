use std::env;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use warp::Filter;

mod api;
mod config;
mod database;
mod logging;
mod models;
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

    let cors_allowed_origins = config.cors_allowed_origins.clone();

    // Create shared application state
    let app_state = api::AppState {
        db: db_pool,
        redis: redis_conn,
        config: config.clone(),
    };

    // Build API routes
    let api_routes = api::routes(app_state);

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
        .recover(api::handle_rejection)
        .with(
            warp::cors()
                .allow_origins(cors_allowed_origins.iter().map(String::as_str))
                .allow_headers(vec!["authorization", "content-type"])
                .expose_headers(vec!["x-request-id", "retry-after"])
                .allow_methods(vec!["GET", "POST", "PUT", "DELETE", "PATCH"]),
        )
        .with(warp::reply::with::headers(security_headers()))
        .with(warp::compression::gzip())
        .map(add_request_id)
        .with(warp::log("code_terroir::api"));

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

fn security_headers() -> warp::http::HeaderMap {
    let mut headers = warp::http::HeaderMap::new();
    headers.insert(
        warp::http::HeaderName::from_static("x-content-type-options"),
        warp::http::HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        warp::http::HeaderName::from_static("x-frame-options"),
        warp::http::HeaderValue::from_static("DENY"),
    );
    headers.insert(
        warp::http::HeaderName::from_static("referrer-policy"),
        warp::http::HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        warp::http::HeaderName::from_static("permissions-policy"),
        warp::http::HeaderValue::from_static("camera=(), microphone=(), geolocation=()"),
    );
    headers.insert(
        warp::http::HeaderName::from_static("content-security-policy"),
        warp::http::HeaderValue::from_static(
            "default-src 'none'; img-src 'self' data: http: https:; style-src 'unsafe-inline'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'",
        ),
    );
    headers
}

fn add_request_id(reply: impl warp::Reply) -> warp::reply::Response {
    let mut response = reply.into_response();
    if !response.headers().contains_key("x-request-id") {
        let request_id = uuid::Uuid::new_v4().to_string();
        response.headers_mut().insert(
            warp::http::HeaderName::from_static("x-request-id"),
            warp::http::HeaderValue::from_str(&request_id)
                .expect("UUID is a valid HTTP header value"),
        );
    }
    response
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

#[cfg(test)]
mod tests {
    use super::*;
    use warp::Filter;

    #[tokio::test]
    async fn cors_headers_are_added_to_recovered_errors() {
        let routes = warp::path("available")
            .map(|| warp::reply::json(&serde_json::json!({"ok": true})))
            .recover(api::handle_rejection)
            .with(
                warp::cors()
                    .allow_origin("http://localhost:8081")
                    .allow_headers(vec!["authorization", "content-type"])
                    .allow_methods(vec!["GET", "POST"]),
            )
            .with(warp::reply::with::headers(security_headers()))
            .map(add_request_id);

        let response = warp::test::request()
            .path("/missing")
            .header("origin", "http://localhost:8081")
            .reply(&routes)
            .await;

        assert_eq!(response.status(), warp::http::StatusCode::NOT_FOUND);
        assert_eq!(
            response.headers().get("access-control-allow-origin"),
            Some(&warp::http::HeaderValue::from_static(
                "http://localhost:8081"
            ))
        );
        assert_eq!(
            response.headers().get("x-content-type-options"),
            Some(&warp::http::HeaderValue::from_static("nosniff"))
        );
        assert!(response.headers().contains_key("x-request-id"));

        let success = warp::test::request()
            .path("/available")
            .reply(&routes)
            .await;
        assert_eq!(success.status(), warp::http::StatusCode::OK);
        assert!(success.headers().contains_key("x-request-id"));
    }
}
