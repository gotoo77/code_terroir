use crate::api::auth::AuthenticatedUser;
use crate::api::{auth, AppState};
use crate::models::producer::{Producer, UpdateProducerRequest};
use crate::models::user::UserRole;
use std::convert::Infallible;
use uuid::Uuid;
use warp::{Filter, Rejection, Reply};

pub fn routes(state: AppState) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let api_prefix = warp::path("api")
        .and(warp::path("v1"))
        .and(warp::path("producers"));

    let list_producers = api_prefix
        .clone()
        .and(warp::get())
        .and(warp::path::end())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(list_producers_handler);

    let get_producer = api_prefix
        .clone()
        .and(warp::get())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(get_producer_handler);

    let update_producer = api_prefix
        .and(warp::put())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::body::json())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(update_producer_handler);

    list_producers.or(get_producer).or(update_producer)
}

fn with_state(state: AppState) -> impl Filter<Extract = (AppState,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}

async fn list_producers_handler(
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    match sqlx::query_as::<_, Producer>("SELECT * FROM producers WHERE id = $1")
        .bind(authenticated.producer_id)
        .fetch_all(&state.db.pool)
        .await
    {
        Ok(producers) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": true,
                "count": producers.len(),
                "producers": producers
            })),
            warp::http::StatusCode::OK,
        )),
        Err(e) => {
            tracing::error!("Erreur lors de la récupération des producteurs: {:?}", e);
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "Erreur lors de la récupération des producteurs",
                    "details": "Erreur interne"
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn get_producer_handler(
    id: String,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    let producer_id = match Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "ID producteur invalide",
                    "details": "L'ID doit être un UUID valide"
                })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
    };

    match sqlx::query_as::<_, Producer>("SELECT * FROM producers WHERE id = $1 AND id = $2")
        .bind(producer_id)
        .bind(authenticated.producer_id)
        .fetch_optional(&state.db.pool)
        .await
    {
        Ok(Some(producer)) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": true,
                "producer": producer
            })),
            warp::http::StatusCode::OK,
        )),
        Ok(None) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": false,
                "error": "Producteur non trouvé",
                "producer_id": id
            })),
            warp::http::StatusCode::NOT_FOUND,
        )),
        Err(e) => {
            tracing::error!(
                "Erreur lors de la récupération du producteur {}: {:?}",
                id,
                e
            );
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "Erreur lors de la récupération du producteur",
                    "details": "Erreur interne"
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn update_producer_handler(
    id: String,
    update_request: UpdateProducerRequest,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    if authenticated.role != UserRole::Admin {
        return Ok(forbidden("Droits administrateur requis"));
    }
    let producer_id = match Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "ID producteur invalide",
                    "details": "L'ID doit être un UUID valide"
                })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
    };

    match sqlx::query_as::<_, Producer>(
        r#"
        UPDATE producers SET
            raison_sociale = COALESCE($2, raison_sociale),
            agrement_sanitaire = COALESCE($3, agrement_sanitaire),
            siret = COALESCE($4, siret),
            adresse = COALESCE($5, adresse),
            code_postal = COALESCE($6, code_postal),
            ville = COALESCE($7, ville),
            pays = COALESCE($8, pays),
            email = COALESCE($9, email),
            telephone = COALESCE($10, telephone),
            site_web = COALESCE($11, site_web),
            logo_url = COALESCE($12, logo_url),
            photo_url = COALESCE($13, photo_url),
            categorie_principale = COALESCE($14, categorie_principale),
            updated_at = NOW()
        WHERE id = $1 AND id = $15
        RETURNING *
        "#,
    )
    .bind(&producer_id)
    .bind(&update_request.raison_sociale)
    .bind(&update_request.agrement_sanitaire)
    .bind(&update_request.siret)
    .bind(&update_request.adresse)
    .bind(&update_request.code_postal)
    .bind(&update_request.ville)
    .bind(&update_request.pays)
    .bind(&update_request.email)
    .bind(&update_request.telephone)
    .bind(&update_request.site_web)
    .bind(&update_request.logo_url)
    .bind(&update_request.photo_url)
    .bind(&update_request.categorie_principale)
    .bind(authenticated.producer_id)
    .fetch_optional(&state.db.pool)
    .await
    {
        Ok(Some(producer)) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": true,
                "producer": producer
            })),
            warp::http::StatusCode::OK,
        )),
        Ok(None) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": false,
                "error": "Producteur non trouvé",
                "producer_id": id
            })),
            warp::http::StatusCode::NOT_FOUND,
        )),
        Err(e) => {
            tracing::error!(
                "Erreur lors de la mise à jour du producteur {}: {:?}",
                id,
                e
            );
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "Erreur lors de la mise à jour du producteur",
                    "details": "Erreur interne"
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

fn forbidden(message: &str) -> warp::reply::WithStatus<warp::reply::Json> {
    warp::reply::with_status(
        warp::reply::json(&serde_json::json!({
            "success": false,
            "error": message
        })),
        warp::http::StatusCode::FORBIDDEN,
    )
}
