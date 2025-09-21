use super::*;
use crate::feature_numbering::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_creation() {
        // This test should fail initially since Feature struct doesn't exist
        let feature = Feature {
            id: "001-test-feature".to_string(),
            number: 1,
            name: "test-feature".to_string(),
            description: "Test feature creation".to_string(),
            status: FeatureStatus::Draft,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            metadata: None,
            branch_name: "001-test-feature".to_string(),
        };

        assert_eq!(feature.id, "001-test-feature");
        assert_eq!(feature.number, 1);
        assert_eq!(feature.name, "test-feature");
        assert_eq!(feature.status, FeatureStatus::Draft);
    }

    #[test]
    fn test_feature_number_creation() {
        // This test should fail initially since FeatureNumber struct doesn't exist
        let feature_number = FeatureNumber {
            value: 1,
            prefix: "".to_string(),
            is_reserved: false,
            assigned_at: None,
            feature_id: None,
        };

        assert_eq!(feature_number.value, 1);
        assert!(!feature_number.is_reserved);
        assert!(feature_number.assigned_at.is_none());
        assert!(feature_number.feature_id.is_none());
    }

    #[test]
    fn test_feature_metadata_creation() {
        // This test should fail initially since FeatureMetadata struct doesn't exist
        let metadata = FeatureMetadata {
            priority: FeaturePriority::Medium,
            tags: vec!["test".to_string()],
            estimated_effort: Some("3 days".to_string()),
            assignee: Some("developer".to_string()),
            dependencies: vec![],
            related_features: vec![],
        };

        assert_eq!(metadata.priority, FeaturePriority::Medium);
        assert_eq!(metadata.tags, vec!["test".to_string()]);
        assert_eq!(metadata.estimated_effort, Some("3 days".to_string()));
    }

    #[test]
    fn test_feature_status_transitions() {
        // Test valid transitions
        let mut feature = create_test_feature();

        // Draft -> Planned should be valid
        let result = feature.update_status(FeatureStatus::Planned);
        assert!(result.is_ok());
        assert_eq!(feature.status, FeatureStatus::Planned);
    }

    #[test]
    fn test_feature_status_invalid_transitions() {
        // Test invalid transitions
        let mut feature = create_test_feature();

        // Draft -> Completed should be invalid
        let result = feature.update_status(FeatureStatus::Completed);
        assert!(result.is_err());
        assert_eq!(feature.status, FeatureStatus::Draft);
    }

    fn create_test_feature() -> Feature {
        Feature {
            id: "001-test".to_string(),
            number: 1,
            name: "test".to_string(),
            description: "Test feature".to_string(),
            status: FeatureStatus::Draft,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            metadata: None,
            branch_name: "001-test".to_string(),
        }
    }
}