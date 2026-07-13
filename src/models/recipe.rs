use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Recipe {
    pub id: Uuid,
    pub producer_id: Uuid,
    pub name: String,
    pub version: i32,
    pub ingredients: serde_json::Value,
    pub steps: serde_json::Value,
    pub notes: Option<String>,
    pub is_active: bool,
    pub ingredients_detail: Option<serde_json::Value>,
    pub etapes_detail: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeIngredient {
    pub ingredient_id: Uuid,
    pub nom: String,
    pub quantite: f32,
    pub unite: String,
    pub fournisseur_id: Option<Uuid>,
    pub fournisseur_nom: Option<String>,
    pub ordre: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeStep {
    pub etape: i32,
    pub nom: String,
    pub description: String,
    pub duree_min: Option<i32>,
    pub temperature_c: Option<i32>,
    pub equipement: Option<String>,
    pub controles: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRecipeRequest {
    pub producer_id: Uuid,
    pub name: String,
    pub ingredients: Vec<RecipeIngredient>,
    pub steps: Vec<RecipeStep>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRecipeRequest {
    pub name: Option<String>,
    pub ingredients: Option<Vec<RecipeIngredient>>,
    pub steps: Option<Vec<RecipeStep>>,
    pub notes: Option<String>,
    pub create_new_version: bool,
}

#[derive(Debug, Serialize)]
pub struct RecipeWithDetails {
    pub recipe: Recipe,
    pub ingredients_detail: Vec<RecipeIngredient>,
    pub steps_detail: Vec<RecipeStep>,
}

impl RecipeWithDetails {
    pub fn from_recipe(recipe: Recipe) -> Self {
        let ingredients_detail = recipe
            .ingredients_detail
            .clone()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
        let steps_detail = recipe
            .etapes_detail
            .clone()
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();

        Self {
            recipe,
            ingredients_detail,
            steps_detail,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recipe_with_details_reads_embedded_json() {
        let recipe = Recipe {
            id: Uuid::new_v4(),
            producer_id: Uuid::new_v4(),
            name: "Cassoulet".to_string(),
            version: 2,
            ingredients: serde_json::json!([]),
            steps: serde_json::json!([]),
            notes: Some("Test".to_string()),
            is_active: true,
            ingredients_detail: Some(serde_json::json!([{
                "ingredient_id": Uuid::nil(),
                "nom": "Haricots",
                "quantite": 1.5,
                "unite": "kg",
                "fournisseur_id": null,
                "fournisseur_nom": null,
                "ordre": 1
            }])),
            etapes_detail: Some(serde_json::json!([{
                "etape": 1,
                "nom": "Cuisson",
                "description": "Cuire doucement",
                "duree_min": 120,
                "temperature_c": 95,
                "equipement": null,
                "controles": ["temperature"]
            }])),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let details = RecipeWithDetails::from_recipe(recipe);
        assert_eq!(details.ingredients_detail.len(), 1);
        assert_eq!(details.steps_detail.len(), 1);
        assert_eq!(details.steps_detail[0].nom, "Cuisson");
    }
}
