use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Producer {
    pub id: Uuid,
    pub raison_sociale: String,
    pub agrement_sanitaire: Option<String>,
    pub siret: Option<String>,
    pub adresse: String,
    pub code_postal: String,
    pub ville: String,
    pub pays: String,
    pub email: String,
    pub telephone: Option<String>,
    pub site_web: Option<String>,
    pub logo_url: Option<String>,
    pub photo_url: Option<String>,
    pub categorie_principale: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProducerRequest {
    pub raison_sociale: Option<String>,
    pub agrement_sanitaire: Option<String>,
    pub siret: Option<String>,
    pub adresse: Option<String>,
    pub code_postal: Option<String>,
    pub ville: Option<String>,
    pub pays: Option<String>,
    pub email: Option<String>,
    pub telephone: Option<String>,
    pub site_web: Option<String>,
    pub logo_url: Option<String>,
    pub photo_url: Option<String>,
    pub categorie_principale: Option<String>,
}
