use crate::api::AppState;
use crate::models::product::{
    normalize_allergens, normalize_nutriscore, CreateProductIngredientLink, CreateProductRequest,
    Product, ProductIngredientLink, ProductWithRelations, UpdateProductRequest,
};
use sqlx::{Postgres, Transaction};
use std::convert::Infallible;
use uuid::Uuid;
use warp::{Filter, Rejection, Reply};

pub fn routes(state: AppState) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let api_prefix = warp::path("api")
        .and(warp::path("v1"))
        .and(warp::path("products"));

    let list_products = api_prefix
        .clone()
        .and(warp::get())
        .and(warp::path::end())
        .and(with_state(state.clone()))
        .and_then(list_products_handler);

    let get_product = api_prefix
        .clone()
        .and(warp::get())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(with_state(state.clone()))
        .and_then(get_product_handler);

    let create_product = api_prefix
        .clone()
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(create_product_handler);

    let update_product = api_prefix
        .and(warp::put())
        .and(warp::path::param::<String>())
        .and(warp::path::end())
        .and(warp::body::json())
        .and(with_state(state.clone()))
        .and_then(update_product_handler);

    list_products
        .or(get_product)
        .or(create_product)
        .or(update_product)
}

fn with_state(state: AppState) -> impl Filter<Extract = (AppState,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}

async fn list_products_handler(state: AppState) -> Result<impl Reply, Rejection> {
    match sqlx::query_as::<_, Product>("SELECT * FROM products ORDER BY created_at DESC")
        .fetch_all(&state.db.pool)
        .await
    {
        Ok(products) => {
            let mut enriched_products = Vec::with_capacity(products.len());
            for product in products {
                let product_ingredients = match fetch_product_ingredients(product.id, &state).await
                {
                    Ok(value) => value,
                    Err(response) => return Ok(response),
                };
                enriched_products.push(ProductWithRelations {
                    product,
                    product_ingredients,
                });
            }

            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": true,
                    "count": enriched_products.len(),
                    "products": enriched_products
                })),
                warp::http::StatusCode::OK,
            ))
        }
        Err(e) => Ok(internal_error_response(
            "Erreur lors de la récupération des produits",
            e,
        )),
    }
}

async fn get_product_handler(id: String, state: AppState) -> Result<impl Reply, Rejection> {
    let product_id = match parse_uuid(&id, "ID produit invalide") {
        Ok(uuid) => uuid,
        Err(response) => return Ok(response),
    };

    match sqlx::query_as::<_, Product>("SELECT * FROM products WHERE id = $1")
        .bind(product_id)
        .fetch_optional(&state.db.pool)
        .await
    {
        Ok(Some(product)) => {
            let product_ingredients = match fetch_product_ingredients(product.id, &state).await {
                Ok(value) => value,
                Err(response) => return Ok(response),
            };

            Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": true,
                    "product": ProductWithRelations {
                        product,
                        product_ingredients,
                    }
                })),
                warp::http::StatusCode::OK,
            ))
        }
        Ok(None) => Ok(warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": false,
                "error": "Produit non trouvé",
                "product_id": id
            })),
            warp::http::StatusCode::NOT_FOUND,
        )),
        Err(e) => Ok(internal_error_response(
            "Erreur lors de la récupération du produit",
            e,
        )),
    }
}

async fn create_product_handler(
    create_request: CreateProductRequest,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    let product_id = Uuid::new_v4();
    let producer_id = match resolve_producer_id(create_request.producer_id, &state).await {
        Ok(producer_id) => producer_id,
        Err(response) => return Ok(response),
    };

    let mut tx = match state.db.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return Ok(internal_error_response(
                "Erreur lors de l'ouverture de la transaction",
                e,
            ))
        }
    };

    let ingredients_json = create_request
        .ingredients
        .as_ref()
        .and_then(|ing| serde_json::to_value(ing).ok());
    let nutritional_json = create_request
        .nutritional_values
        .as_ref()
        .and_then(|nutr| serde_json::to_value(nutr).ok());
    let allergenes = normalize_allergens(create_request.allergenes.clone());
    let nutriscore = normalize_nutriscore(create_request.nutriscore.as_deref());

    let product = match sqlx::query_as::<_, Product>(
        r#"
        INSERT INTO products (
            id, producer_id, name, category, recipe_id,
            description_marketing, ingredients, nutritional_values,
            image_url, nutriscore, energie_kcal_100g, glucides_100g, lipides_100g, proteines_100g, allergenes,
            labels_certifications, visuels, conseils_utilisation, infos_legales
        )
        VALUES (
            $1, $2, $3, $4, $5,
            $6, $7, $8,
            $9, $10, $11, $12, $13, $14, $15,
            $16, $17, $18, $19
        )
        RETURNING *
        "#,
    )
    .bind(product_id)
    .bind(producer_id)
    .bind(&create_request.name)
    .bind(&create_request.category)
    .bind(create_request.recipe_id)
    .bind(&create_request.description_marketing)
    .bind(ingredients_json)
    .bind(nutritional_json)
    .bind(&create_request.image_url)
    .bind(nutriscore)
    .bind(create_request.energie_kcal_100g)
    .bind(create_request.glucides_100g)
    .bind(create_request.lipides_100g)
    .bind(create_request.proteines_100g)
    .bind(&allergenes)
    .bind(create_request.labels_certifications.unwrap_or_default())
    .bind(create_request.visuels.unwrap_or_default())
    .bind(&create_request.conseils_utilisation)
    .bind(&create_request.infos_legales)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(product) => product,
        Err(e) => {
            return Ok(internal_error_response(
                "Erreur lors de la création du produit",
                e,
            ))
        }
    };

    let product_ingredients = match sync_product_ingredients(
        product.id,
        create_request.product_ingredients.as_deref(),
        &state,
        &mut tx,
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return Ok(response),
    };

    if let Err(e) = tx.commit().await {
        return Ok(internal_error_response(
            "Erreur lors de la validation de la transaction produit",
            e,
        ));
    }

    Ok(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({
            "success": true,
            "message": "Produit créé avec succès",
            "product": ProductWithRelations {
                product,
                product_ingredients,
            }
        })),
        warp::http::StatusCode::CREATED,
    ))
}

async fn update_product_handler(
    id: String,
    update_request: UpdateProductRequest,
    state: AppState,
) -> Result<impl Reply, Rejection> {
    let product_id = match parse_uuid(&id, "ID produit invalide") {
        Ok(uuid) => uuid,
        Err(response) => return Ok(response),
    };

    let producer_id = match update_request.producer_id {
        Some(requested_producer_id) => {
            match resolve_producer_id(Some(requested_producer_id), &state).await {
                Ok(value) => Some(value),
                Err(response) => return Ok(response),
            }
        }
        None => None,
    };

    let mut tx = match state.db.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return Ok(internal_error_response(
                "Erreur lors de l'ouverture de la transaction",
                e,
            ))
        }
    };

    let ingredients_json = update_request
        .ingredients
        .as_ref()
        .and_then(|ingredients| serde_json::to_value(ingredients).ok());
    let nutritional_json = update_request
        .nutritional_values
        .as_ref()
        .and_then(|values| serde_json::to_value(values).ok());
    let allergenes = update_request
        .allergenes
        .clone()
        .map(Some)
        .map(normalize_allergens);
    let nutriscore = update_request
        .nutriscore
        .as_deref()
        .map(|value| normalize_nutriscore(Some(value)))
        .flatten();

    let product = match sqlx::query_as::<_, Product>(
        r#"
        UPDATE products
        SET
            name = COALESCE($2, name),
            category = COALESCE($3, category),
            producer_id = COALESCE($4, producer_id),
            recipe_id = COALESCE($5, recipe_id),
            description_marketing = COALESCE($6, description_marketing),
            ingredients = COALESCE($7, ingredients),
            nutritional_values = COALESCE($8, nutritional_values),
            image_url = COALESCE($9, image_url),
            nutriscore = COALESCE($10, nutriscore),
            energie_kcal_100g = COALESCE($11, energie_kcal_100g),
            glucides_100g = COALESCE($12, glucides_100g),
            lipides_100g = COALESCE($13, lipides_100g),
            proteines_100g = COALESCE($14, proteines_100g),
            allergenes = COALESCE($15, allergenes),
            labels_certifications = COALESCE($16, labels_certifications),
            visuels = COALESCE($17, visuels),
            conseils_utilisation = COALESCE($18, conseils_utilisation),
            infos_legales = COALESCE($19, infos_legales),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(product_id)
    .bind(update_request.name)
    .bind(update_request.category)
    .bind(producer_id)
    .bind(update_request.recipe_id)
    .bind(update_request.description_marketing)
    .bind(ingredients_json)
    .bind(nutritional_json)
    .bind(update_request.image_url)
    .bind(nutriscore)
    .bind(update_request.energie_kcal_100g)
    .bind(update_request.glucides_100g)
    .bind(update_request.lipides_100g)
    .bind(update_request.proteines_100g)
    .bind(allergenes)
    .bind(update_request.labels_certifications)
    .bind(update_request.visuels)
    .bind(update_request.conseils_utilisation)
    .bind(update_request.infos_legales)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(Some(product)) => product,
        Ok(None) => {
            return Ok(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "Produit non trouvé",
                    "product_id": id
                })),
                warp::http::StatusCode::NOT_FOUND,
            ))
        }
        Err(e) => {
            return Ok(internal_error_response(
                "Erreur lors de la mise à jour du produit",
                e,
            ))
        }
    };

    let product_ingredients = match update_request.product_ingredients.as_deref() {
        Some(links) => {
            match sync_product_ingredients(product.id, Some(links), &state, &mut tx).await {
                Ok(value) => value,
                Err(response) => return Ok(response),
            }
        }
        None => match fetch_product_ingredients_tx(product.id, &mut tx).await {
            Ok(value) => value,
            Err(e) => {
                return Ok(internal_error_response(
                    "Erreur lors de la lecture des liaisons produit",
                    e,
                ))
            }
        },
    };

    if let Err(e) = tx.commit().await {
        return Ok(internal_error_response(
            "Erreur lors de la validation de la transaction produit",
            e,
        ));
    }

    Ok(warp::reply::with_status(
        warp::reply::json(&serde_json::json!({
            "success": true,
            "message": "Produit mis à jour avec succès",
            "product": ProductWithRelations {
                product,
                product_ingredients,
            }
        })),
        warp::http::StatusCode::OK,
    ))
}

async fn fetch_product_ingredients(
    product_id: Uuid,
    state: &AppState,
) -> Result<Vec<ProductIngredientLink>, warp::reply::WithStatus<warp::reply::Json>> {
    sqlx::query_as::<_, ProductIngredientLink>(
        "SELECT * FROM product_ingredients WHERE product_id = $1 ORDER BY sort_order, created_at",
    )
    .bind(product_id)
    .fetch_all(&state.db.pool)
    .await
    .map_err(|e| internal_error_response("Erreur lors de la récupération des liaisons produit", e))
}

async fn fetch_product_ingredients_tx(
    product_id: Uuid,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<Vec<ProductIngredientLink>, sqlx::Error> {
    sqlx::query_as::<_, ProductIngredientLink>(
        "SELECT * FROM product_ingredients WHERE product_id = $1 ORDER BY sort_order, created_at",
    )
    .bind(product_id)
    .fetch_all(&mut **tx)
    .await
}

async fn sync_product_ingredients(
    product_id: Uuid,
    links: Option<&[CreateProductIngredientLink]>,
    state: &AppState,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<Vec<ProductIngredientLink>, warp::reply::WithStatus<warp::reply::Json>> {
    let Some(links) = links else {
        return fetch_product_ingredients_tx(product_id, tx)
            .await
            .map_err(|e| {
                internal_error_response("Erreur lors de la récupération des liaisons produit", e)
            });
    };

    validate_product_ingredient_links(links, state, tx).await?;

    if let Err(e) = sqlx::query("DELETE FROM product_ingredients WHERE product_id = $1")
        .bind(product_id)
        .execute(&mut **tx)
        .await
    {
        return Err(internal_error_response(
            "Erreur lors de la réinitialisation des ingrédients du produit",
            e,
        ));
    }

    for (index, link) in links.iter().enumerate() {
        if let Err(e) = sqlx::query(
            r#"
            INSERT INTO product_ingredients (
                id, product_id, ingredient_id, producer_id, quantity, unit,
                ingredient_category, notes, sort_order
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(product_id)
        .bind(link.ingredient_id)
        .bind(link.producer_id)
        .bind(link.quantity)
        .bind(&link.unit)
        .bind(&link.ingredient_category)
        .bind(&link.notes)
        .bind(link.sort_order.unwrap_or(index as i32))
        .execute(&mut **tx)
        .await
        {
            return Err(internal_error_response(
                "Erreur lors de l'enregistrement des ingrédients du produit",
                e,
            ));
        }
    }

    fetch_product_ingredients_tx(product_id, tx)
        .await
        .map_err(|e| {
            internal_error_response("Erreur lors de la récupération des liaisons produit", e)
        })
}

async fn validate_product_ingredient_links(
    links: &[CreateProductIngredientLink],
    state: &AppState,
    tx: &mut Transaction<'_, Postgres>,
) -> Result<(), warp::reply::WithStatus<warp::reply::Json>> {
    for link in links {
        if let Err(response) = ensure_producer_exists(link.producer_id, state).await {
            return Err(response);
        }

        match sqlx::query_scalar::<_, Uuid>("SELECT id FROM ingredients WHERE id = $1")
            .bind(link.ingredient_id)
            .fetch_optional(&mut **tx)
            .await
        {
            Ok(Some(_)) => {}
            Ok(None) => {
                return Err(warp::reply::with_status(
                    warp::reply::json(&serde_json::json!({
                        "success": false,
                        "error": "Ingrédient introuvable",
                        "details": format!("ingredient_id {} n'existe pas", link.ingredient_id)
                    })),
                    warp::http::StatusCode::BAD_REQUEST,
                ))
            }
            Err(e) => {
                return Err(internal_error_response(
                    "Erreur lors de la validation des ingrédients",
                    e,
                ))
            }
        }
    }

    Ok(())
}

fn parse_uuid(
    value: &str,
    error_message: &str,
) -> Result<Uuid, warp::reply::WithStatus<warp::reply::Json>> {
    Uuid::parse_str(value).map_err(|_| {
        warp::reply::with_status(
            warp::reply::json(&serde_json::json!({
                "success": false,
                "error": error_message,
                "details": "L'ID doit être un UUID valide"
            })),
            warp::http::StatusCode::BAD_REQUEST,
        )
    })
}

async fn resolve_producer_id(
    requested_producer_id: Option<Uuid>,
    state: &AppState,
) -> Result<Uuid, warp::reply::WithStatus<warp::reply::Json>> {
    if let Some(requested_producer_id) = requested_producer_id {
        ensure_producer_exists(requested_producer_id, state).await?;
        Ok(requested_producer_id)
    } else {
        match sqlx::query_scalar::<_, Uuid>("SELECT id FROM producers LIMIT 1")
            .fetch_optional(&state.db.pool)
            .await
        {
            Ok(Some(id)) => Ok(id),
            Ok(None) => Err(warp::reply::with_status(
                warp::reply::json(&serde_json::json!({
                    "success": false,
                    "error": "Aucun producteur trouvé",
                    "details": "Vous devez d'abord créer un producteur"
                })),
                warp::http::StatusCode::BAD_REQUEST,
            )),
            Err(e) => Err(internal_error_response("Erreur interne", e)),
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
                "details": "L'ID du producteur fourni n'existe pas"
            })),
            warp::http::StatusCode::BAD_REQUEST,
        )),
        Err(e) => Err(internal_error_response("Erreur interne", e)),
    }
}

fn internal_error_response<E: std::fmt::Display>(
    error: &str,
    details: E,
) -> warp::reply::WithStatus<warp::reply::Json> {
    tracing::error!("{}: {}", error, details);
    warp::reply::with_status(
        warp::reply::json(&serde_json::json!({
            "success": false,
            "error": error,
            "details": details.to_string()
        })),
        warp::http::StatusCode::INTERNAL_SERVER_ERROR,
    )
}
