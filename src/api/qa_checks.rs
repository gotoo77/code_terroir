use crate::api::auth::AuthenticatedUser;
use crate::api::{auth, AppState};
use crate::models::qa_check::{
    CreateQACheckRequest, QACheck, QACheckSummary, QACheckType, QAThresholds, UpdateQACheckRequest,
};
use crate::models::user::UserRole;
use std::convert::Infallible;
use uuid::Uuid;
use warp::{Filter, Rejection, Reply};

pub fn routes(state: AppState) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let qa_prefix = warp::path("api")
        .and(warp::path("v1"))
        .and(warp::path("qa-checks"));
    let batch_prefix = warp::path("api")
        .and(warp::path("v1"))
        .and(warp::path("batches"));

    let list_checks = qa_prefix
        .and(warp::get())
        .and(warp::path::end())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(list_checks_handler);

    let get_check = qa_prefix
        .and(warp::get())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(get_check_handler);

    let update_check = qa_prefix
        .and(warp::put())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::body::json())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(update_check_handler);

    let list_batch_checks = batch_prefix
        .and(warp::path::param::<String>())
        .and(warp::path("qa"))
        .and(warp::get())
        .and(warp::path::end())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(list_batch_checks_handler);

    let create_batch_check = batch_prefix
        .and(warp::path::param::<String>())
        .and(warp::path("qa"))
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::json())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(create_batch_check_handler);

    let get_batch_summary = batch_prefix
        .and(warp::path::param::<String>())
        .and(warp::path("qa"))
        .and(warp::path("summary"))
        .and(warp::get())
        .and(warp::path::end())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state))
        .and_then(get_batch_summary_handler);

    list_checks
        .or(get_check)
        .or(update_check)
        .or(list_batch_checks)
        .or(create_batch_check)
        .or(get_batch_summary)
}

fn with_state(state: AppState) -> impl Filter<Extract = (AppState,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}

async fn list_checks_handler(
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    match sqlx::query_as::<_, QACheck>(
        "SELECT q.* FROM qa_checks q INNER JOIN batches b ON b.id = q.batch_id WHERE b.producer_id = $1 ORDER BY q.checked_at DESC",
    )
        .bind(authenticated.producer_id)
        .fetch_all(&state.db.pool)
        .await
    {
        Ok(checks) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": true,
                "count": checks.len(),
                "qa_checks": checks
            })),
            warp::http::StatusCode::OK,
        )),
        Err(e) => server_error("Erreur lors de la récupération des contrôles qualité", e),
    }
}

async fn get_check_handler(
    id: String,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    let check_id = match parse_uuid_param(&id, "ID contrôle qualité invalide") {
        Ok(uuid) => uuid,
        Err(reply) => return Ok(reply),
    };

    match sqlx::query_as::<_, QACheck>(
        "SELECT q.* FROM qa_checks q INNER JOIN batches b ON b.id = q.batch_id WHERE q.id = $1 AND b.producer_id = $2",
    )
        .bind(check_id)
        .bind(authenticated.producer_id)
        .fetch_optional(&state.db.pool)
        .await
    {
        Ok(Some(check)) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": true,
                "qa_check": check
            })),
            warp::http::StatusCode::OK,
        )),
        Ok(None) => Ok(not_found("Contrôle qualité non trouvé", "qa_check_id", id)),
        Err(e) => server_error("Erreur lors de la récupération du contrôle qualité", e),
    }
}

async fn list_batch_checks_handler(
    id: String,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    let batch_id = match parse_uuid_param(&id, "ID lot invalide") {
        Ok(uuid) => uuid,
        Err(reply) => return Ok(reply),
    };

    match sqlx::query_as::<_, QACheck>(
        "SELECT q.* FROM qa_checks q INNER JOIN batches b ON b.id = q.batch_id WHERE q.batch_id = $1 AND b.producer_id = $2 ORDER BY q.checked_at DESC, q.created_at DESC",
    )
    .bind(batch_id)
    .bind(authenticated.producer_id)
    .fetch_all(&state.db.pool)
    .await
    {
        Ok(checks) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": true,
                "batch_id": batch_id,
                "count": checks.len(),
                "qa_checks": checks
            })),
            warp::http::StatusCode::OK,
        )),
        Err(e) => server_error("Erreur lors de la récupération des contrôles du lot", e),
    }
}

async fn create_batch_check_handler(
    id: String,
    create_request: CreateQACheckRequest,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    if !can_create_check(&authenticated.role) {
        return Ok(forbidden());
    }
    let batch_id = match parse_uuid_param(&id, "ID lot invalide") {
        Ok(uuid) => uuid,
        Err(reply) => return Ok(reply),
    };

    if let Err(reply) = ensure_batch_exists(batch_id, authenticated.producer_id, &state).await {
        return Ok(reply);
    }
    if let Err(reply) = ensure_user_exists(
        create_request.operator_id,
        authenticated.producer_id,
        &state,
    )
    .await
    {
        return Ok(reply);
    }

    let mut qa_check = QACheck {
        id: Uuid::new_v4(),
        batch_id,
        check_type: create_request.check_type.clone(),
        value: create_request.value,
        min_threshold: create_request.min_threshold,
        max_threshold: create_request.max_threshold,
        is_compliant: create_request.is_compliant.unwrap_or(false),
        unit: create_request.unit.clone(),
        operator_id: create_request.operator_id,
        notes: create_request.notes.clone(),
        attachments: create_request.attachments.clone().unwrap_or_default(),
        checked_at: chrono::Utc::now(),
        created_at: chrono::Utc::now(),
    };
    let check_type = match create_request.check_type.parse::<QACheckType>() {
        Ok(value) => value,
        Err(error) => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "Type de contrôle invalide",
                    "details": error
                })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
    };
    qa_check.set_check_type(check_type.clone());
    apply_default_thresholds(&mut qa_check, &check_type);
    if create_request.is_compliant.is_none() {
        qa_check.auto_check_compliance();
    }

    match sqlx::query_as::<_, QACheck>(
        r#"
        INSERT INTO qa_checks (
            id, batch_id, check_type, value, min_threshold, max_threshold,
            is_compliant, unit, operator_id, notes, attachments, checked_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())
        RETURNING *
        "#,
    )
    .bind(qa_check.id)
    .bind(qa_check.batch_id)
    .bind(qa_check.check_type)
    .bind(qa_check.value)
    .bind(qa_check.min_threshold)
    .bind(qa_check.max_threshold)
    .bind(qa_check.is_compliant)
    .bind(qa_check.unit)
    .bind(qa_check.operator_id)
    .bind(qa_check.notes)
    .bind(qa_check.attachments)
    .fetch_one(&state.db.pool)
    .await
    {
        Ok(check) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": true,
                "message": "Contrôle qualité créé avec succès",
                "qa_check": check
            })),
            warp::http::StatusCode::CREATED,
        )),
        Err(e) => server_error("Erreur lors de la création du contrôle qualité", e),
    }
}

async fn update_check_handler(
    id: String,
    update_request: UpdateQACheckRequest,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    if !can_update_check(&authenticated.role) {
        return Ok(forbidden());
    }
    let check_id = match parse_uuid_param(&id, "ID contrôle qualité invalide") {
        Ok(uuid) => uuid,
        Err(reply) => return Ok(reply),
    };

    let existing = match sqlx::query_as::<_, QACheck>(
        "SELECT q.* FROM qa_checks q INNER JOIN batches b ON b.id = q.batch_id WHERE q.id = $1 AND b.producer_id = $2",
    )
    .bind(check_id)
    .bind(authenticated.producer_id)
    .fetch_optional(&state.db.pool)
    .await
    {
        Ok(Some(check)) => check,
        Ok(None) => return Ok(not_found("Contrôle qualité non trouvé", "qa_check_id", id)),
        Err(e) => return server_error("Erreur lors de la récupération du contrôle qualité", e),
    };

    let mut updated = QACheck {
        id: existing.id,
        batch_id: existing.batch_id,
        check_type: existing.check_type,
        value: update_request.value.or(existing.value),
        min_threshold: update_request.min_threshold.or(existing.min_threshold),
        max_threshold: update_request.max_threshold.or(existing.max_threshold),
        is_compliant: update_request.is_compliant.unwrap_or(existing.is_compliant),
        unit: update_request.unit.or(existing.unit),
        operator_id: existing.operator_id,
        notes: update_request.notes.or(existing.notes),
        attachments: update_request.attachments.unwrap_or(existing.attachments),
        checked_at: existing.checked_at,
        created_at: existing.created_at,
    };
    if let Ok(check_type) = updated.get_check_type() {
        apply_default_thresholds(&mut updated, &check_type);
    }
    if update_request.is_compliant.is_none() {
        updated.auto_check_compliance();
    }

    match sqlx::query_as::<_, QACheck>(
        r#"
        UPDATE qa_checks
        SET
            value = $2,
            min_threshold = $3,
            max_threshold = $4,
            is_compliant = $5,
            unit = $6,
            notes = $7,
            attachments = $8
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(check_id)
    .bind(updated.value)
    .bind(updated.min_threshold)
    .bind(updated.max_threshold)
    .bind(updated.is_compliant)
    .bind(updated.unit)
    .bind(updated.notes)
    .bind(updated.attachments)
    .fetch_one(&state.db.pool)
    .await
    {
        Ok(check) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": true,
                "message": "Contrôle qualité mis à jour avec succès",
                "qa_check": check
            })),
            warp::http::StatusCode::OK,
        )),
        Err(e) => server_error("Erreur lors de la mise à jour du contrôle qualité", e),
    }
}

async fn get_batch_summary_handler(
    id: String,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    let batch_id = match parse_uuid_param(&id, "ID lot invalide") {
        Ok(uuid) => uuid,
        Err(reply) => return Ok(reply),
    };

    if let Err(reply) = ensure_batch_exists(batch_id, authenticated.producer_id, &state).await {
        return Ok(reply);
    }

    let total_checks =
        match sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM qa_checks WHERE batch_id = $1")
            .bind(batch_id)
            .fetch_one(&state.db.pool)
            .await
        {
            Ok(value) => value,
            Err(e) => return server_error("Erreur lors du calcul du résumé qualité", e),
        };

    let compliant_checks = match sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM qa_checks WHERE batch_id = $1 AND is_compliant = true",
    )
    .bind(batch_id)
    .fetch_one(&state.db.pool)
    .await
    {
        Ok(value) => value,
        Err(e) => return server_error("Erreur lors du calcul du résumé qualité", e),
    };

    let check_types = match sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT check_type FROM qa_checks WHERE batch_id = $1 ORDER BY check_type",
    )
    .bind(batch_id)
    .fetch_all(&state.db.pool)
    .await
    {
        Ok(values) => values,
        Err(e) => return server_error("Erreur lors du calcul du résumé qualité", e),
    };

    let non_compliant_checks = total_checks - compliant_checks;
    let summary = QACheckSummary {
        batch_id,
        total_checks: total_checks as i32,
        compliant_checks: compliant_checks as i32,
        non_compliant_checks: non_compliant_checks as i32,
        compliance_rate: if total_checks == 0 {
            0.0
        } else {
            (compliant_checks as f64 / total_checks as f64) * 100.0
        },
        check_types,
    };

    Ok(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({
            "success": true,
            "summary": summary
        })),
        warp::http::StatusCode::OK,
    ))
}

fn parse_uuid_param(
    value: &str,
    message: &str,
) -> Result<Uuid, warp::reply::WithStatus<warp::reply::Json>> {
    Uuid::parse_str(value).map_err(|_| {
        warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": false,
                "error": message,
                "details": "L'ID doit être un UUID valide"
            })),
            warp::http::StatusCode::BAD_REQUEST,
        )
    })
}

async fn ensure_batch_exists(
    batch_id: Uuid,
    producer_id: Uuid,
    state: &AppState,
) -> Result<(), warp::reply::WithStatus<warp::reply::Json>> {
    match sqlx::query_scalar::<_, Uuid>("SELECT id FROM batches WHERE id = $1 AND producer_id = $2")
        .bind(batch_id)
        .bind(producer_id)
        .fetch_optional(&state.db.pool)
        .await
    {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": false,
                "error": "Lot non trouvé",
                "batch_id": batch_id
            })),
            warp::http::StatusCode::BAD_REQUEST,
        )),
        Err(e) => server_error_reply("Erreur lors de la vérification du lot", e),
    }
}

async fn ensure_user_exists(
    user_id: Uuid,
    producer_id: Uuid,
    state: &AppState,
) -> Result<(), warp::reply::WithStatus<warp::reply::Json>> {
    match sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM users WHERE id = $1 AND producer_id = $2 AND is_active = true",
    )
    .bind(user_id)
    .bind(producer_id)
    .fetch_optional(&state.db.pool)
    .await
    {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": false,
                "error": "Opérateur introuvable",
                "operator_id": user_id
            })),
            warp::http::StatusCode::BAD_REQUEST,
        )),
        Err(e) => server_error_reply("Erreur lors de la vérification de l'opérateur", e),
    }
}

fn not_found(error: &str, key: &str, value: String) -> warp::reply::WithStatus<warp::reply::Json> {
    warp::reply::with_status(
        warp::reply::json(&serde_json::json!({
            "success": false,
            "error": error,
            key: value
        })),
        warp::http::StatusCode::NOT_FOUND,
    )
}

fn server_error(
    error: &str,
    source: sqlx::Error,
) -> Result<warp::reply::WithStatus<warp::reply::Json>, Rejection> {
    tracing::error!("{}: {:?}", error, source);
    Ok(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({
            "success": false,
            "error": error,
            "details": "Erreur interne"
        })),
        warp::http::StatusCode::INTERNAL_SERVER_ERROR,
    ))
}

fn server_error_reply(
    error: &str,
    source: sqlx::Error,
) -> Result<(), warp::reply::WithStatus<warp::reply::Json>> {
    tracing::error!("{}: {:?}", error, source);
    Err(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({
            "success": false,
            "error": error,
            "details": "Erreur interne"
        })),
        warp::http::StatusCode::INTERNAL_SERVER_ERROR,
    ))
}

fn apply_default_thresholds(check: &mut QACheck, check_type: &QACheckType) {
    let defaults = QAThresholds::default();
    match check_type {
        QACheckType::Ph => {
            if check.min_threshold.is_none() {
                check.min_threshold = defaults.ph_min;
            }
            if check.max_threshold.is_none() {
                check.max_threshold = defaults.ph_max;
            }
        }
        QACheckType::Temperature => {
            if check.min_threshold.is_none() {
                check.min_threshold = defaults.core_temp_min;
            }
        }
        _ => {}
    }
}

fn can_create_check(role: &UserRole) -> bool {
    matches!(
        role,
        UserRole::Admin | UserRole::Quality | UserRole::Atelier
    )
}

fn can_update_check(role: &UserRole) -> bool {
    matches!(role, UserRole::Admin | UserRole::Quality)
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
