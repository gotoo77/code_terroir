use crate::api::auth::AuthenticatedUser;
use crate::api::authorization::{is_allowed, Action};
use crate::api::{auth, AppState};
use crate::models::batch::{
    generate_lot_code, Batch, BatchState, CreateBatchRequest, RecallBatchRequest,
    UpdateBatchRequest,
};
use chrono::Utc;
use std::convert::Infallible;
use uuid::Uuid;
use warp::{Filter, Rejection, Reply};

pub fn routes(state: AppState) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let api_prefix = warp::path("api")
        .and(warp::path("v1"))
        .and(warp::path("batches"));

    // List batches
    let list_batches = api_prefix
        .and(warp::get())
        .and(warp::path::end())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(list_batches_handler);

    // Get single batch
    let get_batch = api_prefix
        .and(warp::get())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(get_batch_handler);

    // Create batch
    let create_batch = api_prefix
        .and(warp::post())
        .and(warp::path::end())
        .and(crate::api::json_body(state.clone()))
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(create_batch_handler);

    // Update batch
    let update_batch = api_prefix
        .and(warp::put())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(crate::api::json_body(state.clone()))
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(update_batch_handler);

    // Recall batch
    let recall_batch = api_prefix
        .and(warp::post())
        .and(warp::path::param::<String>())
        .and(warp::path("recall"))
        .and(warp::path::end())
        .and(crate::api::json_body(state.clone()))
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(recall_batch_handler);

    list_batches
        .or(get_batch)
        .or(create_batch)
        .or(update_batch)
        .or(recall_batch)
}

// Helper pour extraire l'état de l'application
fn with_state(state: AppState) -> impl Filter<Extract = (AppState,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}

// Handler pour lister tous les lots
async fn list_batches_handler(
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    match sqlx::query_as::<_, Batch>(
        "SELECT * FROM batches WHERE producer_id = $1 ORDER BY production_date DESC",
    )
    .bind(authenticated.producer_id)
    .fetch_all(&state.db.pool)
    .await
    {
        Ok(batches) => {
            let response = serde_json::json!({
                "success": true,
                "count": batches.len(),
                "batches": batches
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::OK,
            ))
        }
        Err(e) => {
            tracing::error!("Erreur lors de la récupération des lots: {:?}", e);
            let response = serde_json::json!({
                "success": false,
                "error": "Erreur lors de la récupération des lots",
                "details": "Erreur interne"
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

// Handler pour récupérer un lot par ID
async fn get_batch_handler(
    id: String,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    // Valider que l'ID est un UUID valide
    let batch_id = match Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => {
            let response = serde_json::json!({
                "success": false,
                "error": "ID lot invalide",
                "details": "L'ID doit être un UUID valide"
            });
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
    };

    match sqlx::query_as::<_, Batch>("SELECT * FROM batches WHERE id = $1 AND producer_id = $2")
        .bind(batch_id)
        .bind(authenticated.producer_id)
        .fetch_optional(&state.db.pool)
        .await
    {
        Ok(Some(batch)) => {
            let response = serde_json::json!({
                "success": true,
                "batch": batch
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::OK,
            ))
        }
        Ok(None) => {
            let response = serde_json::json!({
                "success": false,
                "error": "Lot non trouvé",
                "batch_id": id
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::NOT_FOUND,
            ))
        }
        Err(e) => {
            tracing::error!("Erreur lors de la récupération du lot {}: {:?}", id, e);
            let response = serde_json::json!({
                "success": false,
                "error": "Erreur lors de la récupération du lot",
                "details": "Erreur interne"
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

// Handler pour créer un nouveau lot
async fn create_batch_handler(
    create_request: CreateBatchRequest,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    if !is_allowed(&authenticated.role, Action::ManageBatches) {
        return Ok(forbidden());
    }
    // Générer un nouvel ID
    let batch_id = Uuid::new_v4();
    let now = Utc::now();

    let producer_id = match sqlx::query_scalar::<_, Uuid>(
        "SELECT producer_id FROM products WHERE id = $1 AND producer_id = $2",
    )
    .bind(create_request.product_id)
    .bind(authenticated.producer_id)
    .fetch_optional(&state.db.pool)
    .await
    {
        Ok(None) => {
            let response = serde_json::json!({
                "success": false,
                "error": "Produit non trouvé",
                "details": format!("Aucun produit trouvé avec l'ID: {}", create_request.product_id)
            });
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
        Ok(Some(producer_id)) => producer_id,
        Err(e) => {
            tracing::error!("Erreur lors de la vérification du produit: {:?}", e);
            let response = serde_json::json!({
                "success": false,
                "error": "Erreur lors de la vérification du produit",
                "details": "Erreur interne"
            });
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    // Générer le code de lot si non fourni
    let lot_code = create_request
        .lot_code
        .unwrap_or_else(|| generate_lot_code("LOT", &now));

    // Convertir les paramètres de production en JSON
    let production_params_json = create_request
        .production_parameters
        .as_ref()
        .and_then(|params| serde_json::to_value(params).ok());

    // Insérer le lot dans la base de données
    match sqlx::query_as::<_, Batch>(
        r#"
        INSERT INTO batches (
            id, product_id, producer_id, lot_code, production_date, dluo_ddm,
            quantity_produced, production_site, operators, production_parameters,
            state, notes, created_at, updated_at
        )
        VALUES ($1, $2, $3, $4, NOW(), $5, $6, $7, $8, $9, $10, $11, NOW(), NOW())
        RETURNING *
        "#,
    )
    .bind(batch_id)
    .bind(create_request.product_id)
    .bind(producer_id)
    .bind(&lot_code)
    .bind(create_request.dluo_ddm)
    .bind(create_request.quantity_produced)
    .bind(&create_request.production_site)
    .bind(create_request.operators.unwrap_or_default())
    .bind(&production_params_json)
    .bind(BatchState::Draft.to_string())
    .bind(&create_request.notes)
    .fetch_one(&state.db.pool)
    .await
    {
        Ok(batch) => {
            tracing::info!("Lot créé avec succès: {} ({})", batch.lot_code, batch.id);
            let response = serde_json::json!({
                "success": true,
                "message": "Lot créé avec succès",
                "batch": batch
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::CREATED,
            ))
        }
        Err(e) => {
            tracing::error!("Erreur lors de la création du lot: {:?}", e);
            let response = serde_json::json!({
                "success": false,
                "error": "Erreur lors de la création du lot",
                "details": "Erreur interne"
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

// Handler pour mettre à jour un lot
async fn update_batch_handler(
    id: String,
    update_request: UpdateBatchRequest,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    if !is_allowed(&authenticated.role, Action::ManageBatches) {
        return Ok(forbidden());
    }
    let batch_id = match Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => {
            let response = serde_json::json!({
                "success": false,
                "error": "ID lot invalide",
                "details": "L'ID doit être un UUID valide"
            });
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
    };

    let production_params_json = update_request
        .production_parameters
        .as_ref()
        .and_then(|params| serde_json::to_value(params).ok());

    match sqlx::query_as::<_, Batch>(
        r#"
        UPDATE batches
        SET
            dluo_ddm = COALESCE($1, dluo_ddm),
            quantity_produced = COALESCE($2, quantity_produced),
            production_site = COALESCE($3, production_site),
            operators = COALESCE($4, operators),
            production_parameters = COALESCE($5, production_parameters),
            notes = COALESCE($6, notes),
            updated_at = NOW()
        WHERE id = $7 AND producer_id = $8
        RETURNING *
        "#,
    )
    .bind(update_request.dluo_ddm)
    .bind(update_request.quantity_produced)
    .bind(update_request.production_site)
    .bind(update_request.operators)
    .bind(production_params_json)
    .bind(update_request.notes)
    .bind(batch_id)
    .bind(authenticated.producer_id)
    .fetch_optional(&state.db.pool)
    .await
    {
        Ok(Some(batch)) => {
            let response = serde_json::json!({
                "success": true,
                "message": "Lot mis à jour avec succès",
                "batch": batch
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::OK,
            ))
        }
        Ok(None) => {
            let response = serde_json::json!({
                "success": false,
                "error": "Lot non trouvé"
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::NOT_FOUND,
            ))
        }
        Err(e) => {
            tracing::error!("Erreur lors de la mise à jour du lot: {:?}", e);
            let response = serde_json::json!({
                "success": false,
                "error": "Erreur lors de la mise à jour du lot",
                "details": "Erreur interne"
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

// Handler pour rappeler un lot
async fn recall_batch_handler(
    id: String,
    recall_request: RecallBatchRequest,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    if !is_allowed(&authenticated.role, Action::RecallBatches) {
        return Ok(forbidden());
    }
    let batch_id = match Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => {
            let response = serde_json::json!({
                "success": false,
                "error": "ID lot invalide",
                "details": "L'ID doit être un UUID valide"
            });
            return Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
    };

    match sqlx::query_as::<_, Batch>(
        "UPDATE batches SET state = 'recalled', recall_reason = $1, recall_date = NOW(), updated_at = NOW() WHERE id = $2 AND producer_id = $3 RETURNING *"
    )
    .bind(&recall_request.reason)
    .bind(batch_id)
    .bind(authenticated.producer_id)
    .fetch_optional(&state.db.pool)
    .await
    {
        Ok(Some(batch)) => {
            tracing::warn!("Lot rappelé: {} - Raison: {}", batch.lot_code, recall_request.reason);
            let response = serde_json::json!({
                "success": true,
                "message": "Lot rappelé avec succès",
                "batch": batch
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::OK,
            ))
        }
        Ok(None) => {
            let response = serde_json::json!({
                "success": false,
                "error": "Lot non trouvé"
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::NOT_FOUND,
            ))
        }
        Err(e) => {
            tracing::error!("Erreur lors du rappel du lot: {:?}", e);
            let response = serde_json::json!({
                "success": false,
                "error": "Erreur lors du rappel du lot",
                "details": "Erreur interne"
            });
            Ok(warp::reply::with_status(
                warp::reply::json(&response),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

fn forbidden() -> warp::reply::WithStatus<warp::reply::Json> {
    warp::reply::with_status(
        warp::reply::json(&serde_json::json!({
            "success": false,
            "error": "Action non autorisée"
        })),
        warp::http::StatusCode::FORBIDDEN,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn update_batch_request_serializes_partial_update_payload() {
        let request = UpdateBatchRequest {
            dluo_ddm: Some(Utc.with_ymd_and_hms(2026, 4, 20, 10, 0, 0).unwrap()),
            quantity_produced: Some(42),
            production_site: Some("Atelier B".to_string()),
            operators: Some(vec!["Rico".to_string()]),
            production_parameters: None,
            notes: Some("Ajustement".to_string()),
        };

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["quantity_produced"], 42);
        assert_eq!(json["production_site"], "Atelier B");
    }
}
