use crate::api::auth::AuthenticatedUser;
use crate::api::{auth, AppState};
use serde::Serialize;
use sqlx::FromRow;
use std::collections::BTreeMap;
use std::convert::Infallible;
use warp::{Filter, Rejection, Reply};

#[derive(Debug, Clone, Serialize, FromRow)]
struct CatalogIngredientRow {
    ingredient_id: uuid::Uuid,
    ingredient_name: String,
    ingredient_category: Option<String>,
    ingredient_allergens: Vec<String>,
    producer_id: uuid::Uuid,
    producer_name: String,
    producer_category: Option<String>,
    producer_city: String,
    producer_email: String,
    producer_phone: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct CatalogIngredientSuggestion {
    ingredient_id: uuid::Uuid,
    ingredient_name: String,
    ingredient_category: Option<String>,
    ingredient_allergens: Vec<String>,
    producer: CatalogProducerSuggestion,
}

#[derive(Debug, Clone, Serialize)]
struct CatalogProducerSuggestion {
    producer_id: uuid::Uuid,
    producer_name: String,
    producer_category: Option<String>,
    producer_city: String,
    producer_email: String,
    producer_phone: Option<String>,
}

pub fn routes(state: AppState) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let api_prefix = warp::path("api")
        .and(warp::path("v1"))
        .and(warp::path("catalog"))
        .and(warp::path("categories"));

    api_prefix
        .and(warp::path::param::<String>())
        .and(warp::path("ingredients"))
        .and(warp::path::end())
        .and(warp::get())
        .and(auth::authenticated(state.clone()))
        .and(with_state(state))
        .and_then(get_category_catalog_handler)
}

fn with_state(state: AppState) -> impl Filter<Extract = (AppState,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}

async fn get_category_catalog_handler(
    category: String,
    authenticated: AuthenticatedUser,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    let normalized_category = category.trim().to_ascii_lowercase();
    if normalized_category.is_empty() {
        return Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": false,
                "error": "Categorie invalide",
                "details": "La categorie ne peut pas etre vide"
            })),
            warp::http::StatusCode::BAD_REQUEST,
        ));
    }

    match sqlx::query_as::<_, CatalogIngredientRow>(
        r#"
        SELECT
            i.id AS ingredient_id,
            i.name AS ingredient_name,
            i.category AS ingredient_category,
            i.allergens AS ingredient_allergens,
            p.id AS producer_id,
            p.raison_sociale AS producer_name,
            p.categorie_principale AS producer_category,
            p.ville AS producer_city,
            p.email AS producer_email,
            p.telephone AS producer_phone
        FROM ingredients i
        INNER JOIN producers p ON p.id = i.producer_id
        WHERE i.producer_id = $2
          AND (
            LOWER(COALESCE(i.category, '')) = $1
            OR LOWER(COALESCE(p.categorie_principale, '')) = $1
          )
        ORDER BY i.name, p.raison_sociale
        "#,
    )
    .bind(&normalized_category)
    .bind(authenticated.producer_id)
    .fetch_all(&state.db.pool)
    .await
    {
        Ok(rows) => {
            let (ingredients, producers) = build_catalog_payload(rows);
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": true,
                    "category": normalized_category,
                    "count": ingredients.len(),
                    "ingredients": ingredients,
                    "producers": producers
                })),
                warp::http::StatusCode::OK,
            ))
        }
        Err(e) => {
            tracing::error!(
                "Erreur catalogue catégorie {}: {:?}",
                normalized_category,
                e
            );
            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "Erreur lors du chargement du catalogue",
                    "details": "Erreur interne"
                })),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

fn build_catalog_payload(
    rows: Vec<CatalogIngredientRow>,
) -> (
    Vec<CatalogIngredientSuggestion>,
    Vec<CatalogProducerSuggestion>,
) {
    let mut producers = BTreeMap::new();
    let mut ingredients = Vec::with_capacity(rows.len());

    for row in rows {
        let producer = CatalogProducerSuggestion {
            producer_id: row.producer_id,
            producer_name: row.producer_name.clone(),
            producer_category: row.producer_category.clone(),
            producer_city: row.producer_city.clone(),
            producer_email: row.producer_email.clone(),
            producer_phone: row.producer_phone.clone(),
        };

        producers.insert(row.producer_id, producer.clone());
        ingredients.push(CatalogIngredientSuggestion {
            ingredient_id: row.ingredient_id,
            ingredient_name: row.ingredient_name,
            ingredient_category: row.ingredient_category,
            ingredient_allergens: row.ingredient_allergens,
            producer,
        });
    }

    (ingredients, producers.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::{build_catalog_payload, CatalogIngredientRow};
    use uuid::Uuid;

    #[test]
    fn build_catalog_payload_deduplicates_producers() {
        let producer_id = Uuid::new_v4();
        let rows = vec![
            CatalogIngredientRow {
                ingredient_id: Uuid::new_v4(),
                ingredient_name: String::from("Carotte"),
                ingredient_category: Some(String::from("legumes")),
                ingredient_allergens: vec![],
                producer_id,
                producer_name: String::from("Apothicaire Culinaire"),
                producer_category: Some(String::from("legumes")),
                producer_city: String::from("Agen"),
                producer_email: String::from("contact@example.com"),
                producer_phone: None,
            },
            CatalogIngredientRow {
                ingredient_id: Uuid::new_v4(),
                ingredient_name: String::from("Poivre"),
                ingredient_category: Some(String::from("assaisonnement")),
                ingredient_allergens: vec![String::from("moutarde")],
                producer_id,
                producer_name: String::from("Apothicaire Culinaire"),
                producer_category: Some(String::from("assaisonnement")),
                producer_city: String::from("Agen"),
                producer_email: String::from("contact@example.com"),
                producer_phone: Some(String::from("+33 5 12 34 56 78")),
            },
        ];

        let (ingredients, producers) = build_catalog_payload(rows);

        assert_eq!(ingredients.len(), 2);
        assert_eq!(producers.len(), 1);
        assert_eq!(producers[0].producer_name, "Apothicaire Culinaire");
    }
}
