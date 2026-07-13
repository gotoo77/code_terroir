use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum QACheckType {
    Ph,
    Temperature,
    Sealed,
    Visual,
    Microbiological,
    Weight,
    Other(String),
}

impl std::fmt::Display for QACheckType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QACheckType::Ph => write!(f, "pH"),
            QACheckType::Temperature => write!(f, "temperature"),
            QACheckType::Sealed => write!(f, "sealed"),
            QACheckType::Visual => write!(f, "visual"),
            QACheckType::Microbiological => write!(f, "microbiological"),
            QACheckType::Weight => write!(f, "weight"),
            QACheckType::Other(s) => write!(f, "{}", s),
        }
    }
}

impl std::str::FromStr for QACheckType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ph" => Ok(QACheckType::Ph),
            "temperature" => Ok(QACheckType::Temperature),
            "sealed" => Ok(QACheckType::Sealed),
            "visual" => Ok(QACheckType::Visual),
            "microbiological" => Ok(QACheckType::Microbiological),
            "weight" => Ok(QACheckType::Weight),
            other => Ok(QACheckType::Other(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct QACheck {
    pub id: Uuid,
    pub batch_id: Uuid,
    pub check_type: String, // Sera converti en QACheckType
    pub value: Option<f64>,
    pub min_threshold: Option<f64>,
    pub max_threshold: Option<f64>,
    pub is_compliant: bool,
    pub unit: Option<String>, // °C, pH, g, etc.
    pub operator_id: Uuid,
    pub notes: Option<String>,
    pub attachments: Vec<String>, // URLs des pièces jointes
    pub checked_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl QACheck {
    pub fn get_check_type(&self) -> Result<QACheckType, String> {
        self.check_type.parse()
    }

    pub fn set_check_type(&mut self, check_type: QACheckType) {
        self.check_type = check_type.to_string();
    }

    /// Vérifie automatiquement la conformité si des seuils sont définis
    pub fn auto_check_compliance(&mut self) {
        if let Some(value) = self.value {
            if let (Some(min), Some(max)) = (self.min_threshold, self.max_threshold) {
                self.is_compliant = value >= min && value <= max;
            } else if let Some(min) = self.min_threshold {
                self.is_compliant = value >= min;
            } else if let Some(max) = self.max_threshold {
                self.is_compliant = value <= max;
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateQACheckRequest {
    pub check_type: String,
    pub value: Option<f64>,
    pub min_threshold: Option<f64>,
    pub max_threshold: Option<f64>,
    pub operator_id: Uuid,
    pub is_compliant: Option<bool>, // Si non fourni, sera calculé automatiquement
    pub unit: Option<String>,
    pub notes: Option<String>,
    pub attachments: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateQACheckRequest {
    pub value: Option<f64>,
    pub min_threshold: Option<f64>,
    pub max_threshold: Option<f64>,
    pub is_compliant: Option<bool>,
    pub unit: Option<String>,
    pub notes: Option<String>,
    pub attachments: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct QACheckSummary {
    pub batch_id: Uuid,
    pub total_checks: i32,
    pub compliant_checks: i32,
    pub non_compliant_checks: i32,
    pub compliance_rate: f64,
    pub check_types: Vec<String>,
}

// Seuils prédéfinis pour différents types de contrôles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QAThresholds {
    pub ph_min: Option<f64>,
    pub ph_max: Option<f64>,
    pub sterilization_temp_min: Option<f64>,
    pub core_temp_min: Option<f64>,
}

impl Default for QAThresholds {
    fn default() -> Self {
        Self {
            ph_min: Some(3.8),
            ph_max: Some(4.6),
            sterilization_temp_min: Some(121.0),
            core_temp_min: Some(85.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qa_check_type_roundtrips_for_ph() {
        assert_eq!("ph".parse::<QACheckType>(), Ok(QACheckType::Ph));
        assert_eq!(QACheckType::Ph.to_string(), "pH");
    }

    #[test]
    fn auto_check_compliance_uses_thresholds() {
        let mut check = QACheck {
            id: Uuid::new_v4(),
            batch_id: Uuid::new_v4(),
            check_type: "temperature".to_string(),
            value: Some(85.0),
            min_threshold: Some(80.0),
            max_threshold: Some(90.0),
            is_compliant: false,
            unit: Some("C".to_string()),
            operator_id: Uuid::new_v4(),
            notes: None,
            attachments: vec![],
            checked_at: Utc::now(),
            created_at: Utc::now(),
        };

        check.auto_check_compliance();
        assert!(check.is_compliant);
    }
}
