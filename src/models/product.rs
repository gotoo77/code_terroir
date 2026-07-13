use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Product {
    pub id: Uuid,
    pub producer_id: Uuid,
    pub name: String,
    pub category: String,
    pub recipe_id: Option<Uuid>,
    pub description_marketing: Option<String>,
    pub ingredients: Option<serde_json::Value>, // Liste détaillée avec %
    pub nutritional_values: Option<serde_json::Value>, // JSON pour flexibilité
    pub image_url: Option<String>,
    pub nutriscore: Option<String>,
    pub energie_kcal_100g: Option<f32>,
    pub glucides_100g: Option<f32>,
    pub lipides_100g: Option<f32>,
    pub proteines_100g: Option<f32>,
    pub allergenes: Vec<String>,
    pub labels_certifications: Vec<String>,
    pub visuels: Vec<String>,                 // Photos produit, logos
    pub conseils_utilisation: Option<String>, // Suggestions service, accords
    pub infos_legales: Option<String>,        // Mentions obligatoires
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Structure pour les ingrédients détaillés
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ingredient {
    pub nom: String,
    pub pourcentage: Option<f32>, // % si obligatoire sur l'étiquette
    pub allergenes: Option<Vec<String>>,
    pub origine: Option<String>,
    pub bio: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NutritionalValues {
    pub energie_kcal: Option<f32>,
    pub lipides: Option<f32>,
    pub glucides: Option<f32>,
    pub proteines: Option<f32>,
    pub sel: Option<f32>,
    pub fibres: Option<f32>,
    pub sucres: Option<f32>,
    // Valeurs pour 100g par défaut
    pub per_100g: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProductIngredientLink {
    pub id: Uuid,
    pub product_id: Uuid,
    pub ingredient_id: Uuid,
    pub producer_id: Uuid,
    pub quantity: Option<f32>,
    pub unit: Option<String>,
    pub ingredient_category: Option<String>,
    pub notes: Option<String>,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductWithRelations {
    pub product: Product,
    pub product_ingredients: Vec<ProductIngredientLink>,
}

// Informations producteur intégrées
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProducteurInfo {
    pub nom: String,
    pub agrement: Option<String>, // Agrément sanitaire
    pub contact: Option<String>,
    pub adresse: Option<String>,
    pub labels: Option<Vec<String>>, // Bio, AOP, IGP, etc.
}

#[derive(Debug, Deserialize)]
pub struct CreateProductRequest {
    pub name: String,
    pub category: String,
    pub producer_id: Option<Uuid>,
    pub recipe_id: Option<Uuid>,
    pub description_marketing: Option<String>,
    pub ingredients: Option<Vec<Ingredient>>,
    pub nutritional_values: Option<NutritionalValues>,
    pub image_url: Option<String>,
    pub nutriscore: Option<String>,
    pub energie_kcal_100g: Option<f32>,
    pub glucides_100g: Option<f32>,
    pub lipides_100g: Option<f32>,
    pub proteines_100g: Option<f32>,
    pub allergenes: Option<Vec<String>>,
    pub product_ingredients: Option<Vec<CreateProductIngredientLink>>,
    pub labels_certifications: Option<Vec<String>>,
    pub visuels: Option<Vec<String>>,
    pub conseils_utilisation: Option<String>,
    pub infos_legales: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct UpdateProductRequest {
    pub name: Option<String>,
    pub category: Option<String>,
    pub producer_id: Option<Uuid>,
    pub recipe_id: Option<Uuid>,
    pub description_marketing: Option<String>,
    pub ingredients: Option<Vec<Ingredient>>,
    pub nutritional_values: Option<NutritionalValues>,
    pub image_url: Option<String>,
    pub nutriscore: Option<String>,
    pub energie_kcal_100g: Option<f32>,
    pub glucides_100g: Option<f32>,
    pub lipides_100g: Option<f32>,
    pub proteines_100g: Option<f32>,
    pub allergenes: Option<Vec<String>>,
    pub product_ingredients: Option<Vec<CreateProductIngredientLink>>,
    pub labels_certifications: Option<Vec<String>>,
    pub visuels: Option<Vec<String>>,
    pub conseils_utilisation: Option<String>,
    pub infos_legales: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProductIngredientLink {
    pub ingredient_id: Uuid,
    pub producer_id: Uuid,
    pub quantity: Option<f32>,
    pub unit: Option<String>,
    pub ingredient_category: Option<String>,
    pub notes: Option<String>,
    pub sort_order: Option<i32>,
}

#[allow(dead_code)]
// Catégories de produits prédéfinies
pub const PRODUCT_CATEGORIES: &[&str] = &[
    "conserve",
    "confiture",
    "legume_appertise",
    "sauce",
    "condiment",
    "charcuterie",
    "fromage",
    "boisson",
    "boulangerie",
    "patisserie",
    "autre",
];

pub const EU_ALLERGENS: &[&str] = &[
    "gluten",
    "crustaces",
    "oeufs",
    "poissons",
    "arachides",
    "soja",
    "lait",
    "fruits-a-coque",
    "celeri",
    "moutarde",
    "sesame",
    "sulfites",
    "lupin",
    "mollusques",
];

pub fn normalize_nutriscore(input: Option<&str>) -> Option<String> {
    input
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_uppercase())
        .filter(|value| matches!(value.as_str(), "A" | "B" | "C" | "D" | "E"))
}

pub fn normalize_allergens(allergens: Option<Vec<String>>) -> Vec<String> {
    let mut normalized = Vec::new();
    for allergen in allergens.unwrap_or_default() {
        let value = allergen.trim().to_ascii_lowercase();
        if value.is_empty()
            || !EU_ALLERGENS.contains(&value.as_str())
            || normalized.iter().any(|existing| existing == &value)
        {
            continue;
        }
        normalized.push(value);
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::{normalize_allergens, normalize_nutriscore};

    #[test]
    fn normalize_nutriscore_accepts_letters_a_to_e() {
        assert_eq!(normalize_nutriscore(Some(" b ")), Some(String::from("B")));
        assert_eq!(normalize_nutriscore(Some("F")), None);
        assert_eq!(normalize_nutriscore(Some("")), None);
    }

    #[test]
    fn normalize_allergens_deduplicates_and_lowercases() {
        let allergens = normalize_allergens(Some(vec![
            String::from("Gluten"),
            String::from(" gluten "),
            String::from("Lait"),
            String::from(""),
        ]));

        assert_eq!(
            allergens,
            vec![String::from("gluten"), String::from("lait")]
        );
    }
}
