use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct QRTag {
    pub id: Uuid,
    pub batch_id: Uuid,
    pub slug: String,                 // Identifiant court unique (ex: x7k3)
    pub short_url: String,            // URL courte complète (ex: https://ct.domain/t/x7k3)
    pub qr_code_svg: Option<String>,  // Code QR au format SVG
    pub qr_code_png: Option<Vec<u8>>, // Code QR au format PNG
    pub print_count: i32,             // Nombre d'impressions
    pub scan_count: i32,              // Nombre de scans
    pub last_scanned_at: Option<DateTime<Utc>>,
    pub is_active: bool, // Permet de désactiver un QR
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateQRTagRequest {
    pub batch_id: Uuid,
    pub format: Option<QRFormat>, // SVG, PNG, ou les deux
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QRFormat {
    SVG,
    PNG,
    Both,
}

impl Default for QRFormat {
    fn default() -> Self {
        QRFormat::Both
    }
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct QRTagResponse {
    pub id: Uuid,
    pub slug: String,
    pub short_url: String,
    pub qr_code_svg: Option<String>,
    pub qr_code_png_base64: Option<String>, // PNG encodé en base64 pour l'API
    pub scan_count: i32,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct QRScan {
    pub id: Uuid,
    pub qr_tag_id: Uuid,
    pub batch_id: Uuid,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
    pub country: Option<String>, // Géolocalisation basique
    pub city: Option<String>,
    pub scanned_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RecordScanRequest {
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub referer: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct QRAnalytics {
    pub qr_tag_id: Uuid,
    pub batch_id: Uuid,
    pub total_scans: i32,
    pub unique_scans: i32, // Basé sur IP
    pub scans_today: i32,
    pub scans_this_week: i32,
    pub scans_this_month: i32,
    pub top_countries: Vec<CountryStats>,
    pub scan_timeline: Vec<DailyScanStats>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct CountryStats {
    pub country: String,
    pub count: i32,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct DailyScanStats {
    pub date: String, // Format YYYY-MM-DD
    pub count: i32,
}

// Utilitaires pour générer des slugs courts
use rand::Rng;

const SLUG_CHARS: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTUVWXYZabcdefghjkmnpqrstuvwxyz";

pub fn generate_slug(length: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..SLUG_CHARS.len());
            SLUG_CHARS[idx] as char
        })
        .collect()
}

pub fn generate_unique_slug() -> String {
    // Génère un slug de 6 caractères par défaut
    // TODO: Vérifier l'unicité en base
    generate_slug(6)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_slug_uses_expected_length_and_charset() {
        let slug = generate_slug(12);
        assert_eq!(slug.len(), 12);
        assert!(slug.chars().all(|ch| SLUG_CHARS.contains(&(ch as u8))));
    }

    #[test]
    fn generate_unique_slug_uses_default_length() {
        assert_eq!(generate_unique_slug().len(), 6);
    }
}
