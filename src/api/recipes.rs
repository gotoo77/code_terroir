use crate::api::auth::AuthenticatedUser;
use crate::api::{auth, AppState};
use crate::models::recipe::{CreateRecipeRequest, Recipe, RecipeWithDetails, UpdateRecipeRequest};
use crate::models::user::UserRole;
use std::convert::Infallible;
use uuid::Uuid;
use warp::{Filter, Rejection, Reply};

pub fn routes(state: AppState) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let api_prefix = warp::path("api")
        .and(warp::path("v1"))
        .and(warp::path("recipes"));

    let list_recipes = api_prefix
        .and(warp::get())
        .and(warp::path::end())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(list_recipes_handler);

    let get_recipe = api_prefix
        .and(warp::get())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(get_recipe_handler);

    let create_recipe = api_prefix
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::json())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(create_recipe_handler);

    let update_recipe = api_prefix
        .and(warp::put())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::body::json())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state.clone()))
        .and_then(update_recipe_handler);

    list_recipes
        .or(get_recipe)
        .or(create_recipe)
        .or(update_recipe)
}

fn with_state(state: AppState) -> impl Filter<Extract = (AppState,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}

async fn list_recipes_handler(
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    match sqlx::query_as::<_, Recipe>(
        "SELECT * FROM recipes WHERE producer_id = $1 ORDER BY name, version DESC, created_at DESC",
    )
    .bind(authenticated.producer_id)
    .fetch_all(&state.db.pool)
    .await
    {
        Ok(recipes) => {
            let recipes = recipes
                .into_iter()
                .map(RecipeWithDetails::from_recipe)
                .collect::<Vec<_>>();

            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": true,
                    "count": recipes.len(),
                    "recipes": recipes
                })),
                warp::http::StatusCode::OK,
            ))
        }
        Err(e) => {
            tracing::error!("Erreur lors de la récupération des recettes: {:?}", e);
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "Erreur lors de la récupération des recettes",
                    "details": "Erreur interne"
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn get_recipe_handler(
    id: String,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    let recipe_id = match Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "ID recette invalide",
                    "details": "L'ID doit être un UUID valide"
                })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
    };

    match sqlx::query_as::<_, Recipe>("SELECT * FROM recipes WHERE id = $1 AND producer_id = $2")
        .bind(recipe_id)
        .bind(authenticated.producer_id)
        .fetch_optional(&state.db.pool)
        .await
    {
        Ok(Some(recipe)) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": true,
                "recipe": RecipeWithDetails::from_recipe(recipe)
            })),
            warp::http::StatusCode::OK,
        )),
        Ok(None) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": false,
                "error": "Recette non trouvée",
                "recipe_id": id
            })),
            warp::http::StatusCode::NOT_FOUND,
        )),
        Err(e) => {
            tracing::error!(
                "Erreur lors de la récupération de la recette {}: {:?}",
                id,
                e
            );
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "Erreur lors de la récupération de la recette",
                    "details": "Erreur interne"
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn create_recipe_handler(
    create_request: CreateRecipeRequest,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    if !can_write(&authenticated.role) || create_request.producer_id != authenticated.producer_id {
        return Ok(forbidden());
    }

    let recipe_id = Uuid::new_v4();
    let version = next_recipe_version(authenticated.producer_id, &create_request.name, &state)
        .await
        .unwrap_or(1);
    let ingredients =
        serde_json::to_value(&create_request.ingredients).unwrap_or(serde_json::json!([]));
    let steps = serde_json::to_value(&create_request.steps).unwrap_or(serde_json::json!([]));

    match sqlx::query_as::<_, Recipe>(
        r#"
        INSERT INTO recipes (
            id, producer_id, name, version, ingredients, steps, notes, is_active,
            ingredients_detail, etapes_detail
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, true, $8, $9)
        RETURNING *
        "#,
    )
    .bind(recipe_id)
    .bind(authenticated.producer_id)
    .bind(create_request.name)
    .bind(version)
    .bind(&ingredients)
    .bind(&steps)
    .bind(create_request.notes)
    .bind(&ingredients)
    .bind(&steps)
    .fetch_one(&state.db.pool)
    .await
    {
        Ok(recipe) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": true,
                "message": "Recette créée avec succès",
                "recipe": RecipeWithDetails::from_recipe(recipe)
            })),
            warp::http::StatusCode::CREATED,
        )),
        Err(e) => {
            tracing::error!("Erreur lors de la création de la recette: {:?}", e);
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "Erreur lors de la création de la recette",
                    "details": "Erreur interne"
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn update_recipe_handler(
    id: String,
    update_request: UpdateRecipeRequest,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    if !can_write(&authenticated.role) {
        return Ok(forbidden());
    }
    let recipe_id = match Uuid::parse_str(&id) {
        Ok(uuid) => uuid,
        Err(_) => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "ID recette invalide",
                    "details": "L'ID doit être un UUID valide"
                })),
                warp::http::StatusCode::BAD_REQUEST,
            ));
        }
    };

    let existing_recipe = match sqlx::query_as::<_, Recipe>(
        "SELECT * FROM recipes WHERE id = $1 AND producer_id = $2",
    )
    .bind(recipe_id)
    .bind(authenticated.producer_id)
    .fetch_optional(&state.db.pool)
    .await
    {
        Ok(Some(recipe)) => recipe,
        Ok(None) => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "Recette non trouvée",
                    "recipe_id": id
                })),
                warp::http::StatusCode::NOT_FOUND,
            ));
        }
        Err(e) => {
            tracing::error!("Erreur lors de la récupération de la recette: {:?}", e);
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "Erreur lors de la récupération de la recette",
                    "details": "Erreur interne"
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ));
        }
    };

    let next_name = update_request
        .name
        .clone()
        .unwrap_or_else(|| existing_recipe.name.clone());
    let next_ingredients = update_request
        .ingredients
        .clone()
        .map(|value| serde_json::to_value(value).unwrap_or(serde_json::json!([])))
        .unwrap_or_else(|| existing_recipe.ingredients.clone());
    let next_steps = update_request
        .steps
        .clone()
        .map(|value| serde_json::to_value(value).unwrap_or(serde_json::json!([])))
        .unwrap_or_else(|| existing_recipe.steps.clone());
    let next_notes = update_request
        .notes
        .clone()
        .or(existing_recipe.notes.clone());

    if update_request.create_new_version {
        let next_version = next_recipe_version(existing_recipe.producer_id, &next_name, &state)
            .await
            .unwrap_or(existing_recipe.version + 1);

        let _ = sqlx::query("UPDATE recipes SET is_active = false, updated_at = NOW() WHERE producer_id = $1 AND name = $2")
            .bind(existing_recipe.producer_id)
            .bind(&existing_recipe.name)
            .execute(&state.db.pool)
            .await;

        match sqlx::query_as::<_, Recipe>(
            r#"
            INSERT INTO recipes (
                id, producer_id, name, version, ingredients, steps, notes, is_active,
                ingredients_detail, etapes_detail
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, true, $8, $9)
            RETURNING *
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(existing_recipe.producer_id)
        .bind(next_name)
        .bind(next_version)
        .bind(&next_ingredients)
        .bind(&next_steps)
        .bind(next_notes)
        .bind(&next_ingredients)
        .bind(&next_steps)
        .fetch_one(&state.db.pool)
        .await
        {
            Ok(recipe) => Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": true,
                    "message": "Nouvelle version de recette créée",
                    "recipe": RecipeWithDetails::from_recipe(recipe)
                })),
                warp::http::StatusCode::OK,
            )),
            Err(e) => {
                tracing::error!(
                    "Erreur lors de la création d'une version de recette: {:?}",
                    e
                );
                Ok(warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({
                        "success": false,
                        "error": "Erreur lors de la création de la nouvelle version",
                        "details": "Erreur interne"
                    })),
                    warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
        }
    } else {
        match sqlx::query_as::<_, Recipe>(
            r#"
            UPDATE recipes
            SET
                name = $2,
                ingredients = $3,
                steps = $4,
                notes = $5,
                ingredients_detail = $6,
                etapes_detail = $7,
                updated_at = NOW()
            WHERE id = $1 AND producer_id = $8
            RETURNING *
            "#,
        )
        .bind(recipe_id)
        .bind(next_name)
        .bind(&next_ingredients)
        .bind(&next_steps)
        .bind(authenticated.producer_id)
        .bind(next_notes)
        .bind(&next_ingredients)
        .bind(&next_steps)
        .fetch_one(&state.db.pool)
        .await
        {
            Ok(recipe) => Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": true,
                    "message": "Recette mise à jour avec succès",
                    "recipe": RecipeWithDetails::from_recipe(recipe)
                })),
                warp::http::StatusCode::OK,
            )),
            Err(e) => {
                tracing::error!("Erreur lors de la mise à jour de la recette: {:?}", e);
                Ok(warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({
                        "success": false,
                        "error": "Erreur lors de la mise à jour de la recette",
                        "details": "Erreur interne"
                    })),
                    warp::http::StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
        }
    }
}

async fn next_recipe_version(
    producer_id: Uuid,
    name: &str,
    state: &AppState,
) -> Result<i32, sqlx::Error> {
    let max_version = sqlx::query_scalar::<_, Option<i32>>(
        "SELECT MAX(version) FROM recipes WHERE producer_id = $1 AND name = $2",
    )
    .bind(producer_id)
    .bind(name)
    .fetch_one(&state.db.pool)
    .await?;

    Ok(max_version.unwrap_or(0) + 1)
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
