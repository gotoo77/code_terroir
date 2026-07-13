use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Ingredient {
    pub id: Uuid,
    pub producer_id: Uuid,
    pub name: String,
    pub category: Option<String>,
    pub allergens: Vec<String>,
    pub nutritional_info: Option<serde_json::Value>,
    pub supplier_id: Option<Uuid>,
    pub documents: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateIngredientRequest {
    pub producer_id: Uuid,
    pub name: String,
    pub category: Option<String>,
    pub allergens: Option<Vec<String>>,
    pub nutritional_info: Option<serde_json::Value>,
    pub supplier_id: Option<Uuid>,
    pub documents: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateIngredientRequest {
    pub name: Option<String>,
    pub category: Option<String>,
    pub allergens: Option<Vec<String>>,
    pub nutritional_info: Option<serde_json::Value>,
    pub supplier_id: Option<Uuid>,
    pub documents: Option<Vec<String>>,
}
