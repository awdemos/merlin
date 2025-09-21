use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::collections::HashMap;
use chrono::Utc;

use super::{Feature, FeatureNumber};
use crate::feature_numbering::error::{FeatureNumberingError, Result};

#[derive(Debug, Serialize, Deserialize)]
pub struct FeatureStorage {
    features: HashMap<String, Feature>,
    feature_numbers: HashMap<u32, FeatureNumber>,
    last_assigned_number: u32,
    reserved_numbers: Vec<u32>,
}

impl FeatureStorage {
    pub fn new() -> Self {
        Self {
            features: HashMap::new(),
            feature_numbers: HashMap::new(),
            last_assigned_number: 0,
            reserved_numbers: vec![100, 200, 300], // Default reserved numbers
        }
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        if !path.as_ref().exists() {
            return Ok(Self::new());
        }

        let content = fs::read_to_string(path)?;
        let storage: FeatureStorage = serde_json::from_str(&content)?;
        Ok(storage)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn get_next_available_number(&mut self) -> u32 {
        let mut next = self.last_assigned_number + 1;

        // Skip reserved numbers
        while self.reserved_numbers.contains(&next) {
            next += 1;
        }

        next
    }

    pub fn reserve_number(&mut self, number: u32, _reason: String) -> Result<()> {
        if self.feature_numbers.contains_key(&number) {
            return Err(FeatureNumberingError::NumberAlreadyAssigned(number));
        }

        if !self.reserved_numbers.contains(&number) {
            self.reserved_numbers.push(number);
        }

        Ok(())
    }

    pub fn create_feature(&mut self, name: String, description: String) -> Result<Feature> {
        let number = self.get_next_available_number();
        self.last_assigned_number = number;

        let feature = Feature::new(number, name.clone(), description.clone());
        let feature_id = feature.id.clone();

        // Store feature number
        let feature_number = FeatureNumber::new(number, "".to_string())
            .assign(feature_id.clone());
        self.feature_numbers.insert(number, feature_number);

        // Store feature
        self.features.insert(feature_id.clone(), feature.clone());

        Ok(feature)
    }

    pub fn get_feature(&self, feature_id: &str) -> Option<&Feature> {
        self.features.get(feature_id)
    }

    pub fn update_feature(&mut self, feature_id: &str, mut updated_feature: Feature) -> Result<()> {
        if !self.features.contains_key(feature_id) {
            return Err(FeatureNumberingError::FeatureNotFound(feature_id.to_string()));
        }

        updated_feature.updated_at = Utc::now();
        self.features.insert(feature_id.to_string(), updated_feature);
        Ok(())
    }

    pub fn delete_feature(&mut self, feature_id: &str) -> Result<()> {
        let feature = self.features.get(feature_id)
            .ok_or_else(|| FeatureNumberingError::FeatureNotFound(feature_id.to_string()))?
            .clone();

        // Only allow deletion of Draft features
        if feature.status != crate::feature_numbering::data_models::FeatureStatus::Draft {
            return Err(FeatureNumberingError::ValidationError(
                "Only Draft features can be deleted".to_string()
            ));
        }

        // Remove feature
        self.features.remove(feature_id);

        // Remove feature number assignment
        if let Some(feature_number) = self.feature_numbers.get_mut(&feature.number) {
            feature_number.assigned_at = None;
            feature_number.feature_id = None;
        }

        Ok(())
    }

    pub fn list_features(&self) -> Vec<&Feature> {
        self.features.values().collect()
    }

    pub fn get_reserved_numbers(&self) -> Vec<&FeatureNumber> {
        self.reserved_numbers.iter()
            .filter_map(|num| self.feature_numbers.get(num))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_creation() {
        let storage = FeatureStorage::new();
        assert!(storage.features.is_empty());
        assert!(storage.feature_numbers.is_empty());
        assert_eq!(storage.last_assigned_number, 0);
    }

    #[test]
    fn test_get_next_available_number() {
        let mut storage = FeatureStorage::new();

        // First available number should be 1
        assert_eq!(storage.get_next_available_number(), 1);

        // After assignment, next should be 2
        storage.last_assigned_number = 1;
        assert_eq!(storage.get_next_available_number(), 2);

        // Skip reserved number 100
        storage.last_assigned_number = 99;
        assert_eq!(storage.get_next_available_number(), 101);
    }

    #[test]
    fn test_create_feature() {
        let mut storage = FeatureStorage::new();

        let feature = storage.create_feature(
            "test-feature".to_string(),
            "Test description".to_string()
        ).unwrap();

        assert_eq!(feature.id, "001-test-feature");
        assert_eq!(feature.number, 1);
        assert_eq!(storage.last_assigned_number, 1);
        assert!(storage.features.contains_key("001-test-feature"));
        assert!(storage.feature_numbers.contains_key(&1));
    }

    #[test]
    fn test_reserve_number() {
        let mut storage = FeatureStorage::new();

        // Reserve number 5
        assert!(storage.reserve_number(5, "Special feature".to_string()).is_ok());
        assert!(storage.reserved_numbers.contains(&5));

        // Should skip 5 when getting next available number
        storage.last_assigned_number = 4;
        assert_eq!(storage.get_next_available_number(), 6);
    }

    #[test]
    fn test_reserve_existing_number() {
        let mut storage = FeatureStorage::new();

        // Create a feature first
        storage.create_feature("test".to_string(), "test".to_string()).unwrap();

        // Try to reserve the same number
        let result = storage.reserve_number(1, "Should fail".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_get_feature() {
        let mut storage = FeatureStorage::new();

        // Create a feature
        let created = storage.create_feature("test".to_string(), "test".to_string()).unwrap();

        // Retrieve it
        let retrieved = storage.get_feature("001-test").unwrap();
        assert_eq!(retrieved.id, created.id);

        // Try to get non-existent feature
        assert!(storage.get_feature("non-existent").is_none());
    }

    #[test]
    fn test_delete_feature() {
        let mut storage = FeatureStorage::new();

        // Create a feature
        storage.create_feature("test".to_string(), "test".to_string()).unwrap();

        // Delete it (should succeed since it's in Draft status)
        assert!(storage.delete_feature("001-test").is_ok());
        assert!(!storage.features.contains_key("001-test"));

        // Verify feature number is no longer assigned
        let feature_number = storage.feature_numbers.get(&1).unwrap();
        assert!(feature_number.feature_id.is_none());
    }

    #[test]
    fn test_delete_non_draft_feature() {
        let mut storage = FeatureStorage::new();

        // Create and update feature to non-Draft status
        let mut feature = storage.create_feature("test".to_string(), "test".to_string()).unwrap();
        feature.update_status(crate::feature_numbering::data_models::FeatureStatus::Planned).unwrap();
        storage.update_feature("001-test", feature).unwrap();

        // Try to delete - should fail
        let result = storage.delete_feature("001-test");
        assert!(result.is_err());
    }
}