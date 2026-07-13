use redis::aio::ConnectionManager;
use serde::de::DeserializeOwned;
use std::convert::Infallible;
use uuid::Uuid;
use warp::{Filter, Rejection, Reply};

use crate::config::AppConfig;
use crate::database::DatabasePool;

pub mod auth;
pub mod batches;
pub mod catalog;
pub mod ingredients;
pub mod producers;
pub mod products;
// pub mod qr;  // Obsolète - remplacé par qr_tags
pub mod public;
pub mod qa_checks;
pub mod qr_tags;
pub mod recipes;
pub mod suppliers;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabasePool,
    #[allow(dead_code)]
    pub redis: ConnectionManager,
    pub config: AppConfig,
}

pub fn json_body<T>(state: AppState) -> impl Filter<Extract = (T,), Error = Rejection> + Clone
where
    T: DeserializeOwned + Send,
{
    warp::body::content_length_limit(state.config.security.max_request_body_bytes)
        .and(warp::body::json())
}

// Routes principales
pub fn routes(state: AppState) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let auth_routes = auth::routes(state.clone()).boxed();
    let product_routes = products::routes(state.clone()).boxed();
    let producer_routes = producers::routes(state.clone()).boxed();
    let supplier_routes = suppliers::routes(state.clone()).boxed();
    let ingredient_routes = ingredients::routes(state.clone()).boxed();
    let catalog_routes = catalog::routes(state.clone()).boxed();
    let qa_check_routes = qa_checks::routes(state.clone()).boxed();
    let recipe_routes = recipes::routes(state.clone()).boxed();
    let batch_routes = batches::routes(state.clone()).boxed();
    // let qr_routes = qr::routes(state.clone());  // Obsolète
    let qr_tag_routes = qr_tags::routes(state.clone()).boxed();
    let public_routes = public::routes(state.clone()).boxed();

    let private_routes = product_routes
        .or(producer_routes)
        .or(supplier_routes)
        .or(ingredient_routes)
        .or(catalog_routes)
        .or(qa_check_routes)
        .or(recipe_routes)
        .or(batch_routes)
        // .or(qr_routes)  // Obsolète
        .or(qr_tag_routes)
        .boxed();

    auth_routes.or(public_routes).or(private_routes).boxed()
}

// Gestionnaire d'erreurs global
pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let error_id = Uuid::new_v4();
    let code;
    let message;

    if err.find::<auth::AuthenticationRequired>().is_some() {
        code = warp::http::StatusCode::UNAUTHORIZED;
        message = "Authentication required";
    } else if err.is_not_found() {
        code = warp::http::StatusCode::NOT_FOUND;
        message = "Resource not found";
    } else if err
        .find::<warp::filters::body::BodyDeserializeError>()
        .is_some()
    {
        code = warp::http::StatusCode::BAD_REQUEST;
        message = "Invalid request body";
    } else if err.find::<warp::reject::PayloadTooLarge>().is_some() {
        code = warp::http::StatusCode::PAYLOAD_TOO_LARGE;
        message = "Request payload is too large";
    } else if err.find::<warp::reject::LengthRequired>().is_some() {
        code = warp::http::StatusCode::LENGTH_REQUIRED;
        message = "Content-Length header is required";
    } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
        code = warp::http::StatusCode::METHOD_NOT_ALLOWED;
        message = "Method not allowed";
    } else {
        tracing::error!(error_id = %error_id, rejection = ?err, "Unhandled rejection");
        code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
        message = "Internal server error";
    }

    let json = warp::reply::json(&serde_json::json!({
        "error": message,
        "code": code.as_u16(),
        "error_id": error_id
    }));

    let mut response = warp::reply::with_status(json, code).into_response();
    response.headers_mut().insert(
        warp::http::HeaderName::from_static("x-request-id"),
        warp::http::HeaderValue::from_str(&error_id.to_string())
            .expect("UUID is a valid HTTP header value"),
    );
    if code == warp::http::StatusCode::UNAUTHORIZED {
        response.headers_mut().insert(
            warp::http::header::WWW_AUTHENTICATE,
            warp::http::HeaderValue::from_static("Bearer"),
        );
    }

    Ok(response)
}
