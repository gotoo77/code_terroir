use redis::aio::ConnectionManager;
use std::convert::Infallible;
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

// Routes principales
pub fn routes(state: AppState) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let auth_routes = auth::routes(state.clone());
    let product_routes = products::routes(state.clone());
    let producer_routes = producers::routes(state.clone());
    let supplier_routes = suppliers::routes(state.clone());
    let ingredient_routes = ingredients::routes(state.clone());
    let catalog_routes = catalog::routes(state.clone());
    let qa_check_routes = qa_checks::routes(state.clone());
    let recipe_routes = recipes::routes(state.clone());
    let batch_routes = batches::routes(state.clone());
    // let qr_routes = qr::routes(state.clone());  // Obsolète
    let qr_tag_routes = qr_tags::routes(state.clone());
    let public_routes = public::routes(state.clone());

    let private_routes = product_routes
        .or(producer_routes)
        .or(supplier_routes)
        .or(ingredient_routes)
        .or(catalog_routes)
        .or(qa_check_routes)
        .or(recipe_routes)
        .or(batch_routes)
        // .or(qr_routes)  // Obsolète
        .or(qr_tag_routes);

    auth_routes.or(public_routes).or(private_routes)
}

// Gestionnaire d'erreurs global
pub async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
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
    } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
        code = warp::http::StatusCode::METHOD_NOT_ALLOWED;
        message = "Method not allowed";
    } else {
        tracing::error!("Unhandled rejection: {:?}", err);
        code = warp::http::StatusCode::INTERNAL_SERVER_ERROR;
        message = "Internal server error";
    }

    let json = warp::reply::json(&serde_json::json!({
        "error": message,
        "code": code.as_u16()
    }));

    let mut response = warp::reply::with_status(json, code).into_response();
    if code == warp::http::StatusCode::UNAUTHORIZED {
        response.headers_mut().insert(
            warp::http::header::WWW_AUTHENTICATE,
            warp::http::HeaderValue::from_static("Bearer"),
        );
    }

    Ok(response)
}
