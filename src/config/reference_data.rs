use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::Path;

const DEFAULT_REFERENCE_DATA: &str = include_str!("../../config/reference-data.json");
const EU_PROFILE: &str = "eu-1169-2011";
const REQUIRED_EU_ALLERGENS: &[&str] = &[
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceData {
    pub regulatory_profile: String,
    pub allergens: Vec<AllergenDefinition>,
    pub product_categories: Vec<String>,
    pub ingredient_categories: Vec<String>,
    pub units: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllergenDefinition {
    pub id: String,
    pub label: String,
}

impl ReferenceData {
    pub fn load(path: Option<&Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = match path {
            Some(path) => fs::read_to_string(path)?,
            None => DEFAULT_REFERENCE_DATA.to_owned(),
        };
        let data: Self = serde_json::from_str(&contents)?;
        data.validate()?;
        Ok(data)
    }

    pub fn contains_allergen(&self, id: &str) -> bool {
        self.allergens.iter().any(|allergen| allergen.id == id)
    }

    pub fn contains_product_category(&self, category: &str) -> bool {
        self.product_categories
            .iter()
            .any(|value| value == category)
    }

    pub fn contains_ingredient_category(&self, category: &str) -> bool {
        self.ingredient_categories
            .iter()
            .any(|value| value == category)
    }

    fn validate(&self) -> Result<(), Error> {
        if self.regulatory_profile.trim().is_empty() {
            return Err(invalid_data("regulatory_profile ne peut pas être vide"));
        }

        let mut allergen_ids = HashSet::new();
        for allergen in &self.allergens {
            if allergen.id.trim().is_empty() || allergen.label.trim().is_empty() {
                return Err(invalid_data(
                    "chaque allergène doit avoir un identifiant et un libellé",
                ));
            }
            if !allergen_ids.insert(allergen.id.as_str()) {
                return Err(invalid_data(format!(
                    "identifiant d'allergène dupliqué: {}",
                    allergen.id
                )));
            }
        }

        if self.regulatory_profile == EU_PROFILE {
            for required in REQUIRED_EU_ALLERGENS {
                if !allergen_ids.contains(required) {
                    return Err(invalid_data(format!(
                        "le profil {EU_PROFILE} exige l'allergène {required}"
                    )));
                }
            }
        }

        validate_unique_values("product_categories", &self.product_categories)?;
        validate_unique_values("ingredient_categories", &self.ingredient_categories)?;
        validate_unique_values("units", &self.units)?;
        Ok(())
    }
}

fn validate_unique_values(name: &str, values: &[String]) -> Result<(), Error> {
    if values.is_empty() {
        return Err(invalid_data(format!("{name} ne peut pas être vide")));
    }

    let mut unique = HashSet::new();
    for value in values {
        if value.trim().is_empty() || !unique.insert(value.as_str()) {
            return Err(invalid_data(format!(
                "{name} contient une valeur vide ou dupliquée"
            )));
        }
    }
    Ok(())
}

fn invalid_data(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidData, message.into())
}

#[cfg(test)]
mod tests {
    use super::{ReferenceData, REQUIRED_EU_ALLERGENS};

    #[test]
    fn embedded_reference_data_is_valid_and_complete() {
        let data = ReferenceData::load(None).expect("valid embedded reference data");
        for allergen in REQUIRED_EU_ALLERGENS {
            assert!(data.contains_allergen(allergen));
        }
    }

    #[test]
    fn european_profile_rejects_a_missing_required_allergen() {
        let mut data = ReferenceData::load(None).expect("valid embedded reference data");
        data.allergens.retain(|allergen| allergen.id != "lait");
        assert!(data.validate().is_err());
    }
}
