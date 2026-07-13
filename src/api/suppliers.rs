use crate::api::auth::AuthenticatedUser;
use crate::api::{auth, AppState};
use crate::models::supplier::{CreateSupplierRequest, Supplier, UpdateSupplierRequest};
use crate::models::user::UserRole;
use std::convert::Infallible;
use uuid::Uuid;
use warp::{Filter, Rejection, Reply};

pub fn routes(state: AppState) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let api_prefix = warp::path("api")
        .and(warp::path("v1"))
        .and(warp::path("suppliers"));

    let list_suppliers = api_prefix
        .and(warp::get())
        .and(warp::path::end())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(list_suppliers_handler);

    let get_supplier = api_prefix
        .and(warp::get())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(get_supplier_handler);

    let create_supplier = api_prefix
        .and(warp::post())
        .and(warp::path::end())
        .and(crate::api::json_body(state.clone()))
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(create_supplier_handler);

    let update_supplier = api_prefix
        .and(warp::put())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(crate::api::json_body(state.clone()))
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(update_supplier_handler);

    list_suppliers
        .or(get_supplier)
        .or(create_supplier)
        .or(update_supplier)
}

fn with_state(state: AppState) -> impl Filter<Extract = (AppState,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}

async fn list_suppliers_handler(
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    match sqlx::query_as::<_, Supplier>(
        "SELECT * FROM suppliers WHERE producer_id = $1 ORDER BY created_at DESC",
    )
    .bind(authenticated.producer_id)
    .fetch_all(&state.db.pool)
    .await
    {
        Ok(suppliers) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": true,
                "count": suppliers.len(),
                "suppliers": suppliers
            })),
            warp::http::StatusCode::OK,
        )),
        Err(e) => {
            tracing::error!("Erreur lors de la récupération des fournisseurs: {:?}", e);
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "Erreur lors de la récupération des fournisseurs",
                    "details": "Erreur interne"
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn get_supplier_handler(
    id: String,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    let supplier_id = match Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "ID supplier invalide",
                    "details": "L'ID doit être un UUID valide"
                })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
    };

    match sqlx::query_as::<_, Supplier>(
        "SELECT * FROM suppliers WHERE id = $1 AND producer_id = $2",
    )
    .bind(supplier_id)
    .bind(authenticated.producer_id)
    .fetch_optional(&state.db.pool)
    .await
    {
        Ok(Some(supplier)) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": true,
                "supplier": supplier
            })),
            warp::http::StatusCode::OK,
        )),
        Ok(None) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": false,
                "error": "Fournisseur non trouvé",
                "supplier_id": id
            })),
            warp::http::StatusCode::NOT_FOUND,
        )),
        Err(e) => {
            tracing::error!(
                "Erreur lors de la récupération du fournisseur {}: {:?}",
                id,
                e
            );
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "Erreur lors de la récupération du fournisseur",
                    "details": "Erreur interne"
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn create_supplier_handler(
    create_request: CreateSupplierRequest,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    if !can_write(&authenticated.role) || create_request.producer_id != authenticated.producer_id {
        return Ok(forbidden());
    }
    let producer_id = authenticated.producer_id;

    let supplier_id = Uuid::new_v4();
    match sqlx::query_as::<_, Supplier>(
        r#"
        INSERT INTO suppliers (
            id, producer_id, name, contact_email,
            contact_phone, address, certifications, documents
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(supplier_id)
    .bind(producer_id)
    .bind(&create_request.name)
    .bind(&create_request.contact_email)
    .bind(&create_request.contact_phone)
    .bind(&create_request.address)
    .bind(&create_request.certifications)
    .bind(create_request.documents.unwrap_or_default())
    .fetch_one(&state.db.pool)
    .await
    {
        Ok(supplier) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": true,
                "message": "Fournisseur créé avec succès",
                "supplier": supplier
            })),
            warp::http::StatusCode::CREATED,
        )),
        Err(e) => {
            tracing::error!("Erreur lors de la création du fournisseur: {:?}", e);
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "Erreur lors de la création du fournisseur",
                    "details": "Erreur interne"
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn update_supplier_handler(
    id: String,
    update_request: UpdateSupplierRequest,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    if !can_write(&authenticated.role) {
        return Ok(forbidden());
    }
    let supplier_id = match Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "ID supplier invalide",
                    "details": "L'ID doit être un UUID valide"
                })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
    };

    match sqlx::query_as::<_, Supplier>(
        r#"
        UPDATE suppliers SET
            name = COALESCE($2, name),
            contact_email = COALESCE($3, contact_email),
            contact_phone = COALESCE($4, contact_phone),
            address = COALESCE($5, address),
            certifications = COALESCE($6, certifications),
            documents = COALESCE($7, documents),
            updated_at = NOW()
        WHERE id = $1 AND producer_id = $8
        RETURNING *
        "#,
    )
    .bind(supplier_id)
    .bind(&update_request.name)
    .bind(&update_request.contact_email)
    .bind(&update_request.contact_phone)
    .bind(&update_request.address)
    .bind(&update_request.certifications)
    .bind(&update_request.documents)
    .bind(authenticated.producer_id)
    .fetch_optional(&state.db.pool)
    .await
    {
        Ok(Some(supplier)) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": true,
                "supplier": supplier
            })),
            warp::http::StatusCode::OK,
        )),
        Ok(None) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": false,
                "error": "Fournisseur non trouvé",
                "supplier_id": id
            })),
            warp::http::StatusCode::NOT_FOUND,
        )),
        Err(e) => {
            tracing::error!(
                "Erreur lors de la mise à jour du fournisseur {}: {:?}",
                id,
                e
            );
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "Erreur lors de la mise à jour du fournisseur",
                    "details": "Erreur interne"
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

fn can_write(role: &UserRole) -> bool {
    matches!(role, UserRole::Admin | UserRole::Atelier)
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
