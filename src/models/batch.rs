use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BatchState {
    Draft,     // Brouillon
    QC,        // Contrôles qualité en cours
    Published, // Publié (QR accessible)
    Recalled,  // Rappelé
}

impl std::fmt::Display for BatchState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatchState::Draft => write!(f, "draft"),
            BatchState::QC => write!(f, "qc"),
            BatchState::Published => write!(f, "published"),
            BatchState::Recalled => write!(f, "recalled"),
        }
    }
}

impl std::str::FromStr for BatchState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "draft" => Ok(BatchState::Draft),
            "qc" => Ok(BatchState::QC),
            "published" => Ok(BatchState::Published),
            "recalled" => Ok(BatchState::Recalled),
            _ => Err(format!("Invalid batch state: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Batch {
    pub id: Uuid,
    pub product_id: Uuid,
    pub producer_id: Uuid,
    pub lot_code: String,
    pub production_date: DateTime<Utc>,
    pub dluo_ddm: DateTime<Utc>, // Date Limite d'Utilisation Optimale ou Date de Durabilité Minimale
    pub quantity_produced: i32,
    pub production_site: String,
    pub operators: Vec<String>, // IDs des opérateurs
    pub production_parameters: Option<serde_json::Value>, // JSON pour flexibilité
    pub resultats_controle: Option<serde_json::Value>, // Liste des QualityControl
    pub docs_associes: Option<serde_json::Value>, // Liste des AssociatedDocument
    pub state: String,          // Sera converti en BatchState
    pub notes: Option<String>,
    pub recall_reason: Option<String>,
    pub recall_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Batch {
    #[allow(dead_code)]
    pub fn get_state(&self) -> Result<BatchState, String> {
        self.state.parse()
    }

    #[allow(dead_code)]
    pub fn set_state(&mut self, state: BatchState) {
        self.state = state.to_string();
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductionParameters {
    pub sterilisation_temp_c: Option<i32>,
    pub sterilisation_duree_min: Option<i32>,
    pub cuisson_temp_c: Option<i32>,
    pub cuisson_duree_min: Option<i32>,
    pub refroidissement_duree_min: Option<i32>,
    pub ph_cible: Option<f32>,
    pub pression_bar: Option<f32>,
    pub ligne_production: Option<String>,
    pub machine_id: Option<String>,
    pub additional_params: Option<serde_json::Value>,
}

// Structure pour les résultats de contrôle qualité
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityControl {
    pub type_controle: String, // pH, température, pression, aspect visuel, etc.
    pub valeur: Option<f32>,
    pub valeur_text: Option<String>, // Pour les contrôles qualitatifs
    pub seuil_min: Option<f32>,
    pub seuil_max: Option<f32>,
    pub unite: Option<String>,
    pub conforme: bool,
    pub operateur_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub commentaire: Option<String>,
}

// Structure pour les documents associés
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssociatedDocument {
    pub nom: String,
    pub type_doc: String, // "fiche_haccp", "photo_cuve", "bon_reception", "analyse_labo"
    pub url: String,      // Chemin vers le fichier
    pub taille_ko: Option<i32>,
    pub upload_date: DateTime<Utc>,
    pub operateur_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CreateBatchRequest {
    pub product_id: Uuid,
    pub lot_code: Option<String>, // Auto-généré si non fourni
    pub dluo_ddm: DateTime<Utc>,
    pub quantity_produced: i32,
    pub production_site: String,
    pub operators: Option<Vec<String>>,
    pub production_parameters: Option<ProductionParameters>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateBatchRequest {
    pub dluo_ddm: Option<DateTime<Utc>>,
    pub quantity_produced: Option<i32>,
    pub production_site: Option<String>,
    pub operators: Option<Vec<String>>,
    pub production_parameters: Option<ProductionParameters>,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RecallBatchRequest {
    pub reason: String,
}

// Utilitaire pour générer un code de lot
pub fn generate_lot_code(product_code: &str, production_date: &DateTime<Utc>) -> String {
    let date_str = production_date.format("%Y-%m-%d").to_string();
    // TODO: Ajouter une lettre incrémentale si plusieurs lots le même jour
    format!("{}-{}-A", product_code, date_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn batch_state_roundtrips() {
        assert_eq!("draft".parse::<BatchState>(), Ok(BatchState::Draft));
        assert_eq!(BatchState::Published.to_string(), "published");
    }

    #[test]
    fn generate_lot_code_uses_product_code_and_date() {
        let date = Utc.with_ymd_and_hms(2026, 4, 19, 8, 30, 0).unwrap();
        assert_eq!(generate_lot_code("CASS", &date), "CASS-2026-04-19-A");
    }
}
