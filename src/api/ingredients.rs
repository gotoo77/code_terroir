use crate::api::AppState;
use crate::models::ingredient::{CreateIngredientRequest, Ingredient, UpdateIngredientRequest};
use std::convert::Infallible;
use uuid::Uuid;
use warp::{Filter, Rejection, Reply};

pub fn routes(state: AppState) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let api_prefix = warp::path("api")
        .and(warp::path("v1"))
        .and(warp::path("ingredients"));

    let list_ingredients = api_prefix
        .clone()
        .and(warp::get())
        .and(warp::path::end())
        .and(with_state(state.clone()))
        .and_then(list_ingredients_handler);

    let get_ingredient = api_prefix
        .clone()
        .and(warp::get())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(with_state(state.clone()))
        .and_then(get_ingredient_handler);

    let create_ingredient = api_prefix
        .clone()
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(create_ingredient_handler);

    let update_ingredient = api_prefix
        .and(warp::put())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(update_ingredient_handler);

    list_ingredients
        .or(get_ingredient)
        .or(create_ingredient)
        .or(update_ingredient)
}

fn with_state(state: AppState) -> impl Filter<Extract = (AppState,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}

async fn list_ingredients_handler(state: AppState) -> Result<impl Reply, Rejection> {
    match sqlx::query_as::<_, Ingredient>("SELECT * FROM ingredients ORDER BY created_at DESC")
        .fetch_all(&state.db.pool)
        .await
    {
        Ok(ingredients) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": true,
                "count": ingredients.len(),
                "ingredients": ingredients
            })),
            warp::http::StatusCode::OK,
        )),
        Err(e) => {
            tracing::error!("Erreur lors de la récupération des ingrédients: {:?}", e);
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "Erreur lors de la récupération des ingrédients",
                    "details": e.to_string()
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn get_ingredient_handler(id: String, state: AppState) -> Result<impl Reply, Rejection> {
    let ingredient_id = match Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "ID ingrédient invalide",
                    "details": "L'ID doit être un UUID valide"
                })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
    };

    match sqlx::query_as::<_, Ingredient>("SELECT * FROM ingredients WHERE id = $1")
        .bind(ingredient_id)
        .fetch_optional(&state.db.pool)
        .await
    {
        Ok(Some(ingredient)) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": true,
                "ingredient": ingredient
            })),
            warp::http::StatusCode::OK,
        )),
        Ok(None) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": false,
                "error": "Ingrédient non trouvé",
                "ingredient_id": id
            })),
            warp::http::StatusCode::NOT_FOUND,
        )),
        Err(e) => {
            tracing::error!(
                "Erreur lors de la récupération de l'ingrédient {}: {:?}",
                id,
                e
            );
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "Erreur lors de la récupération de l'ingrédient",
                    "details": e.to_string()
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn create_ingredient_handler(
    create_request: CreateIngredientRequest,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    if let Err(response) = ensure_producer_exists(create_request.producer_id, &state).await {
        return Ok(response);
    }

    if let Some(supplier_id) = create_request.supplier_id {
        if let Err(response) = ensure_supplier_exists(supplier_id, &state).await {
            return Ok(response);
        }
    }

    let ingredient_id = Uuid::new_v4();
    match sqlx::query_as::<_, Ingredient>(
        r#"
        INSERT INTO ingredients (
            id, producer_id, name, category, allergens, nutritional_info, supplier_id, documents
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING *
        "#,
    )
    .bind(ingredient_id)
    .bind(create_request.producer_id)
    .bind(create_request.name)
    .bind(create_request.category)
    .bind(create_request.allergens.unwrap_or_default())
    .bind(create_request.nutritional_info)
    .bind(create_request.supplier_id)
    .bind(create_request.documents.unwrap_or_default())
    .fetch_one(&state.db.pool)
    .await
    {
        Ok(ingredient) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": true,
                "message": "Ingrédient créé avec succès",
                "ingredient": ingredient
            })),
            warp::http::StatusCode::CREATED,
        )),
        Err(e) => {
            tracing::error!("Erreur lors de la création de l'ingrédient: {:?}", e);
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "Erreur lors de la création de l'ingrédient",
                    "details": e.to_string()
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn update_ingredient_handler(
    id: String,
    update_request: UpdateIngredientRequest,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    let ingredient_id = match Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "ID ingrédient invalide",
                    "details": "L'ID doit être un UUID valide"
                })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
    };

    if let Some(supplier_id) = update_request.supplier_id {
        if let Err(response) = ensure_supplier_exists(supplier_id, &state).await {
            return Ok(response);
        }
    }

    match sqlx::query_as::<_, Ingredient>(
        r#"
        UPDATE ingredients
        SET
            name = COALESCE($2, name),
            category = COALESCE($3, category),
            allergens = COALESCE($4, allergens),
            nutritional_info = COALESCE($5, nutritional_info),
            supplier_id = COALESCE($6, supplier_id),
            documents = COALESCE($7, documents),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(ingredient_id)
    .bind(update_request.name)
    .bind(update_request.category)
    .bind(update_request.allergens)
    .bind(update_request.nutritional_info)
    .bind(update_request.supplier_id)
    .bind(update_request.documents)
    .fetch_optional(&state.db.pool)
    .await
    {
        Ok(Some(ingredient)) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": true,
                "ingredient": ingredient
            })),
            warp::http::StatusCode::OK,
        )),
        Ok(None) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": false,
                "error": "Ingrédient non trouvé",
                "ingredient_id": id
            })),
            warp::http::StatusCode::NOT_FOUND,
        )),
        Err(e) => {
            tracing::error!(
                "Erreur lors de la mise à jour de l'ingrédient {}: {:?}",
                id,
                e
            );
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "Erreur lors de la mise à jour de l'ingrédient",
                    "details": e.to_string()
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn ensure_producer_exists(
    producer_id: Uuid,
    state: &AppState,
) -> Result<(), warp::reply::WithStatus<warp::reply::Json>> {
    match sqlx::query_scalar::<_, Uuid>("SELECT id FROM producers WHERE id = $1")
        .bind(producer_id)
        .fetch_optional(&state.db.pool)
        .await
    {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": false,
                "error": "Producteur introuvable",
                "details": "Le producer_id fourni n'existe pas"
            })),
            warp::http::StatusCode::BAD_REQUEST,
        )),
        Err(e) => Err(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": false,
                "error": "Erreur interne",
                "details": e.to_string()
            })),
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

async fn ensure_supplier_exists(
    supplier_id: Uuid,
    state: &AppState,
) -> Result<(), warp::reply::WithStatus<warp::reply::Json>> {
    match sqlx::query_scalar::<_, Uuid>("SELECT id FROM suppliers WHERE id = $1")
        .bind(supplier_id)
        .fetch_optional(&state.db.pool)
        .await
    {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": false,
                "error": "Fournisseur introuvable",
                "details": "Le supplier_id fourni n'existe pas"
            })),
            warp::http::StatusCode::BAD_REQUEST,
        )),
        Err(e) => Err(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": false,
                "error": "Erreur interne",
                "details": e.to_string()
            })),
            warp::http::StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}
