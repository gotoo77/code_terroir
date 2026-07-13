#![allow(dead_code)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Modèle pour les boîtes individuelles (optionnel selon granularité)
/// Utilisé pour les séries limitées ou le storytelling poussé (comme une bouteille de vin numérotée)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct IndividualBox {
    pub id: Uuid,
    pub batch_id: Uuid,                 // Référence vers le lot
    pub serial_number: String,          // Ex: "0001/0240" pour la boîte 1 sur 240
    pub qr_code_slug: Option<String>,   // Slug unique pour le QR collé sur cette boîte
    pub numero_sequence: i32,           // Numéro dans la séquence (1, 2, 3...)
    pub total_sequence: i32,            // Total de boîtes dans le lot (240 dans l'exemple)
    pub date_numerotage: DateTime<Utc>, // Quand la boîte a été numérotée
    pub operateur_id: Uuid,             // Qui a effectué la numérotation
    pub statut: String,                 // "active", "vendue", "perdue", "rappel"
    pub commentaire: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Historique des scans pour une boîte individuelle
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct BoxScanLog {
    pub id: Uuid,
    pub box_id: Uuid,
    pub scan_date: DateTime<Utc>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub geolocation: Option<String>, // Pays/ville si détectable
    pub scan_type: String,           // "qr_scan", "manual_lookup", "api_call"
    pub additional_data: Option<serde_json::Value>,
}

/// Structure pour créer une série de boîtes numérotées
#[derive(Debug, Deserialize)]
pub struct CreateBoxSeriesRequest {
    pub batch_id: Uuid,
    pub quantite: i32,               // Nombre de boîtes à créer
    pub format_numerotation: String, // Ex: "{numero:04}/{total:04}" pour "0001/0240"
    pub generer_qr: bool,            // Si true, génère un QR unique pour chaque boîte
}

/// Structure pour mettre à jour le statut d'une boîte
#[derive(Debug, Deserialize)]
pub struct UpdateBoxStatusRequest {
    pub statut: String,
    pub commentaire: Option<String>,
}

/// Réponse pour afficher une boîte avec ses infos complètes
#[derive(Debug, Serialize)]
pub struct BoxWithDetails {
    pub box_info: IndividualBox,
    pub batch_info: serde_json::Value,   // Infos du lot
    pub product_info: serde_json::Value, // Infos du produit
    pub scan_history: Vec<BoxScanLog>,
    pub qr_stats: Option<serde_json::Value>, // Statistiques de scan si applicable
}

/// Utilitaires pour la numérotation
impl IndividualBox {
    /// Génère un numéro de série formaté
    pub fn format_serial_number(numero: i32, total: i32, format: &str) -> String {
        format
            .replace("{numero:04}", &format!("{:04}", numero))
            .replace("{total:04}", &format!("{:04}", total))
            .replace("{numero}", &numero.to_string())
            .replace("{total}", &total.to_string())
    }

    /// Vérifie si une boîte est dans un lot rappelé
    pub fn is_recalled(&self) -> bool {
        self.statut == "rappel"
    }

    /// Génère un slug QR unique pour cette boîte
    pub fn generate_qr_slug(&self) -> String {
        // Format: LOT-SERIAL (ex: RCD-2025-09-18-A-0001)
        format!(
            "{}-{:04}",
            self.batch_id.to_string().split('-').next().unwrap_or("BOX"),
            self.numero_sequence
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serial_number_formatting() {
        assert_eq!(
            IndividualBox::format_serial_number(1, 240, "{numero:04}/{total:04}"),
            "0001/0240"
        );
        assert_eq!(
            IndividualBox::format_serial_number(42, 100, "N°{numero} sur {total}"),
            "N°42 sur 100"
        );
    }
}
