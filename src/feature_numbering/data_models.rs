use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::feature_numbering::error::FeatureNumberingError;
use crate::feature_numbering::error::Result;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FeatureStatus {
    Draft,
    Planned,
    InProgress,
    Review,
    Completed,
    Cancelled,
    OnHold,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FeaturePriority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReferenceType {
    Dependency,
    Related,
    Duplicates,
    Supersedes,
    BlockedBy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureMetadata {
    pub priority: FeaturePriority,
    pub tags: Vec<String>,
    pub estimated_effort: Option<String>,
    pub assignee: Option<String>,
    pub dependencies: Vec<String>,
    pub related_features: Vec<String>,
}

impl Default for FeatureMetadata {
    fn default() -> Self {
        Self {
            priority: FeaturePriority::Medium,
            tags: Vec::new(),
            estimated_effort: None,
            assignee: None,
            dependencies: Vec::new(),
            related_features: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Feature {
    pub id: String,
    pub number: u32,
    pub name: String,
    pub description: String,
    pub status: FeatureStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: Option<FeatureMetadata>,
    pub branch_name: String,
}

impl Feature {
    pub fn new(number: u32, name: String, description: String) -> Self {
        let now = Utc::now();
        let id = format!("{:03}-{}", number, name);
        Self {
            id: id.clone(),
            number,
            name: name.clone(),
            description,
            status: FeatureStatus::Draft,
            created_at: now,
            updated_at: now,
            metadata: None,
            branch_name: id,
        }
    }

    pub fn update_status(&mut self, new_status: FeatureStatus) -> Result<()> {
        if self.is_valid_transition(&new_status) {
            self.status = new_status;
            self.updated_at = Utc::now();
            Ok(())
        } else {
            Err(FeatureNumberingError::InvalidStatusTransition)
        }
    }

    fn is_valid_transition(&self, new_status: &FeatureStatus) -> bool {
        use FeatureStatus::*;

        match (&self.status, new_status) {
            // Draft can go to Planned or Cancelled
            (Draft, Planned) | (Draft, Cancelled) => true,
            // Planned can go to InProgress or Cancelled
            (Planned, InProgress) | (Planned, Cancelled) => true,
            // InProgress can go to Review, OnHold, or Cancelled
            (InProgress, Review) | (InProgress, OnHold) | (InProgress, Cancelled) => true,
            // Review can go to Completed, InProgress, or Cancelled
            (Review, Completed) | (Review, InProgress) | (Review, Cancelled) => true,
            // OnHold can go back to InProgress or Cancelled
            (OnHold, InProgress) | (OnHold, Cancelled) => true,
            // Completed and Cancelled are terminal states
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureNumber {
    pub value: u32,
    pub prefix: String,
    pub is_reserved: bool,
    pub assigned_at: Option<DateTime<Utc>>,
    pub feature_id: Option<String>,
}

impl FeatureNumber {
    pub fn new(value: u32, prefix: String) -> Self {
        Self {
            value,
            prefix,
            is_reserved: false,
            assigned_at: None,
            feature_id: None,
        }
    }

    pub fn reserve(mut self) -> Self {
        self.is_reserved = true;
        self
    }

    pub fn assign(mut self, feature_id: String) -> Self {
        self.assigned_at = Some(Utc::now());
        self.feature_id = Some(feature_id);
        self
    }

    pub fn is_available(&self) -> bool {
        !self.is_reserved && self.feature_id.is_none()
    }

    pub fn formatted(&self) -> String {
        format!("{}{:03}", self.prefix, self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_creation() {
        let feature = Feature::new(1, "test-feature".to_string(), "Test feature".to_string());

        assert_eq!(feature.id, "001-test-feature");
        assert_eq!(feature.number, 1);
        assert_eq!(feature.name, "test-feature");
        assert_eq!(feature.status, FeatureStatus::Draft);
        assert_eq!(feature.branch_name, "001-test-feature");
    }

    #[test]
    fn test_feature_number_creation() {
        let feature_number = FeatureNumber::new(1, "".to_string());

        assert_eq!(feature_number.value, 1);
        assert!(!feature_number.is_reserved);
        assert!(feature_number.assigned_at.is_none());
        assert!(feature_number.feature_id.is_none());
    }

    #[test]
    fn test_feature_metadata_creation() {
        let metadata = FeatureMetadata::default();

        assert_eq!(metadata.priority, FeaturePriority::Medium);
        assert!(metadata.tags.is_empty());
        assert!(metadata.estimated_effort.is_none());
        assert!(metadata.assignee.is_none());
    }

    #[test]
    fn test_valid_status_transitions() {
        let mut feature = Feature::new(1, "test".to_string(), "Test".to_string());

        // Draft -> Planned should be valid
        assert!(feature.update_status(FeatureStatus::Planned).is_ok());
        assert_eq!(feature.status, FeatureStatus::Planned);
    }

    #[test]
    fn test_invalid_status_transitions() {
        let mut feature = Feature::new(1, "test".to_string(), "Test".to_string());

        // Draft -> Completed should be invalid
        assert!(feature.update_status(FeatureStatus::Completed).is_err());
        assert_eq!(feature.status, FeatureStatus::Draft);
    }

    #[test]
    fn test_feature_number_reserve() {
        let feature_number = FeatureNumber::new(1, "".to_string()).reserve();
        assert!(feature_number.is_reserved);
    }

    #[test]
    fn test_feature_number_assign() {
        let feature_number = FeatureNumber::new(1, "".to_string())
            .assign("test-feature".to_string());

        assert!(feature_number.assigned_at.is_some());
        assert_eq!(feature_number.feature_id, Some("test-feature".to_string()));
    }

    #[test]
    fn test_feature_number_availability() {
        let available = FeatureNumber::new(1, "".to_string());
        assert!(available.is_available());

        let reserved = FeatureNumber::new(2, "".to_string()).reserve();
        assert!(!reserved.is_available());

        let assigned = FeatureNumber::new(3, "".to_string())
            .assign("test".to_string());
        assert!(!assigned.is_available());
    }

    #[test]
    fn test_feature_number_formatted() {
        let feature_number = FeatureNumber::new(1, "FEATURE-".to_string());
        assert_eq!(feature_number.formatted(), "FEATURE-001");
    }
}