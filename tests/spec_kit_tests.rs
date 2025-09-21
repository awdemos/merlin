//! Integration tests for spec-kit functionality

use merlin::spec_kit::{SpecKit, Spec, SpecStatus, RequirementCategory, Priority, TaskStatus};
use tempfile::TempDir;
use std::path::Path;

#[test]
fn test_spec_kit_creation() {
    let temp_dir = TempDir::new().unwrap();
    let spec_kit = SpecKit::new(temp_dir.path()).unwrap();

    assert_eq!(spec_kit.base_dir, temp_dir.path());
}

#[test]
fn test_spec_generation() {
    let temp_dir = TempDir::new().unwrap();
    let spec_kit = SpecKit::new(temp_dir.path()).unwrap();

    let description = "Implement a new API endpoint for user management";
    let spec = spec_kit.generate_spec(description).unwrap();

    assert!(!spec.id.is_empty());
    assert!(spec.title.contains("API"));
    assert!(!spec.executive_summary.is_empty());
    assert_eq!(spec.status, SpecStatus::Draft);
    assert!(!spec.requirements.is_empty());
    assert!(!spec.success_criteria.is_empty());
}

#[test]
fn test_spec_validation() {
    let temp_dir = TempDir::new().unwrap();
    let spec_kit = SpecKit::new(temp_dir.path()).unwrap();

    // Create a valid spec
    let spec_content = r#"
# Test Specification

## Executive Summary
This is a test specification.

## Requirements
### User Requirements
- [ ] User must be able to access the feature

### System Requirements
- [ ] System must integrate with existing components

## Success Criteria
- [ ] All requirements are met
- [ ] Tests pass successfully

## Technical Requirements
### Architecture
- [ ] Must follow existing patterns

### Performance
- [ ] Response time < 100ms

### Security
- [ ] No hardcoded secrets

## Implementation Plan
### Stage 1: Foundation
**Goal**: Set up foundation
**Success Criteria**: Foundation is ready

### Stage 2: Implementation
**Goal**: Implement core features
**Success Criteria**: Features are working

## Testing Strategy
### Unit Tests
- [ ] Core functionality

### Integration Tests
- [ ] API integration

## Acceptance Criteria
- [ ] Feature is complete and tested
"#;

    let spec_file = temp_dir.path().join("spec.md");
    std::fs::write(&spec_file, spec_content).unwrap();

    let result = spec_kit.validate_spec(&spec_file).unwrap();
    assert!(result.is_valid);
    assert!(result.errors.is_empty());
}

#[test]
fn test_spec_validation_invalid() {
    let temp_dir = TempDir::new().unwrap();
    let spec_kit = SpecKit::new(temp_dir.path()).unwrap();

    // Create an invalid spec (missing required sections)
    let spec_content = "# Invalid Spec\n\nJust some content.";

    let spec_file = temp_dir.path().join("spec.md");
    std::fs::write(&spec_file, spec_content).unwrap();

    let result = spec_kit.validate_spec(&spec_file).unwrap();
    assert!(!result.is_valid);
    assert!(!result.errors.is_empty());
    assert!(result.errors.iter().any(|e| e.contains("Missing required section")));
}

#[test]
fn test_spec_execution() {
    let temp_dir = TempDir::new().unwrap();
    let spec_kit = SpecKit::new(temp_dir.path()).unwrap();

    // Create a simple spec for execution
    let spec_content = r#"
# Simple Test Spec

## Executive Summary
Simple test for execution.

## Requirements
### System Requirements
- [ ] System must work

## Success Criteria
- [ ] Tests pass

## Technical Requirements
### Architecture
- [ ] Simple architecture

## Implementation Plan
### Stage 1: Simple Implementation
**Goal**: Create simple implementation
**Success Criteria**: Implementation works

## Testing Strategy
### Unit Tests
- [ ] Simple tests

## Acceptance Criteria
- [ ] Everything works
"#;

    let spec_file = temp_dir.path().join("spec.md");
    std::fs::write(&spec_file, spec_content).unwrap();

    let result = spec_kit.execute_spec(&spec_file).unwrap();
    assert!(result.success);
    assert!(!result.output.is_empty());
    assert!(!result.artifacts.is_empty());
}

#[test]
fn test_spec_file_operations() {
    let temp_dir = TempDir::new().unwrap();
    let spec_kit = SpecKit::new(temp_dir.path()).unwrap();

    // Generate a spec
    let description = "Test feature for file operations";
    let spec = spec_kit.generate_spec(description).unwrap();

    // Create spec files
    let spec_dir = temp_dir.path().join("test-spec");
    let generator = merlin::spec_kit::SpecGenerator::new(
        merlin::spec_kit::SpecTemplates::new(temp_dir.path()).unwrap()
    );

    generator.create_spec_files(&spec, &spec_dir).unwrap();

    // Verify files were created
    assert!(spec_dir.join("spec.md").exists());
    assert!(spec_dir.join("plan.md").exists());
    assert!(spec_dir.join("tasks.md").exists());

    // Load spec from file
    let loaded_spec = spec_kit.get_spec("test-spec").unwrap();
    assert_eq!(loaded_spec.title, spec.title);
}

#[test]
fn test_list_specs() {
    let temp_dir = TempDir::new().unwrap();
    let spec_kit = SpecKit::new(temp_dir.path()).unwrap();

    // Initially empty
    let specs = spec_kit.list_specs().unwrap();
    assert!(specs.is_empty());

    // Create a spec directory and file
    let spec_dir = temp_dir.path().join("001-test-spec");
    std::fs::create_dir_all(&spec_dir).unwrap();

    let spec_content = r#"
# Test Spec

## Executive Summary
Test specification.

## Requirements
### User Requirements
- [ ] User requirement

## Success Criteria
- [ ] Success criteria

## Technical Requirements
### Architecture
- [ ] Architecture requirement

## Implementation Plan
### Stage 1: Test Stage
**Goal**: Test goal
**Success Criteria**: Test success

## Testing Strategy
### Unit Tests
- [ ] Unit test

## Acceptance Criteria
- [ ] Acceptance criterion
"#;

    std::fs::write(spec_dir.join("spec.md"), spec_content).unwrap();

    // Now should have one spec
    let specs = spec_kit.list_specs().unwrap();
    assert_eq!(specs.len(), 1);
    assert!(specs[0].title.contains("Test Spec"));
}

#[test]
fn test_spec_markdown_parsing() {
    let markdown_content = r#"
# Test Feature

## Executive Summary
This is a test feature for testing markdown parsing.

## Requirements
### User Requirements
- [ ] Users can access the feature
- [ ] Feature is user-friendly

### System Requirements
- [ ] System integrates properly

## Success Criteria
- [ ] All tests pass
- [ ] Performance benchmarks met

## Technical Requirements
### Architecture
- [ ] Follows existing patterns

## Implementation Plan
### Stage 1: Implementation
**Goal**: Implement the feature
**Success Criteria**: Feature works

## Testing Strategy
### Unit Tests
- [ ] Core functionality

## Acceptance Criteria
- [ ] Feature is complete
"#;

    let spec = Spec::from_markdown(markdown_content).unwrap();

    assert_eq!(spec.title, "Test Feature");
    assert!(spec.executive_summary.contains("test feature"));
    assert_eq!(spec.status, SpecStatus::Draft);
    assert!(!spec.requirements.is_empty());
    assert_eq!(spec.success_criteria.len(), 2);
}

#[test]
fn test_template_management() {
    let temp_dir = TempDir::new().unwrap();
    let templates_dir = temp_dir.path().join("templates");
    std::fs::create_dir_all(&templates_dir).unwrap();

    let manager = merlin::spec_kit::TemplateManager::new(temp_dir.path()).unwrap();

    // Initially no templates
    let templates = manager.list_templates().unwrap();
    assert!(templates.is_empty());

    // Create a template
    let template_content = r#"## Description
Test template

## Category
test

# ${TITLE}

${DESCRIPTION}
"#;
    manager.create_template("test", template_content).unwrap();

    // Should now have one template
    let templates = manager.list_templates().unwrap();
    assert_eq!(templates.len(), 1);
    assert_eq!(templates[0].name, "test");
    assert_eq!(templates[0].category, "test");

    // Load and render template
    let loaded_template = manager.load_template("test").unwrap();
    assert!(loaded_template.contains("Test template"));

    // Test template rendering
    let mut engine = merlin::spec_kit::TemplateEngine::new();
    engine.add_variable("TITLE", "Test Feature");
    engine.add_variable("DESCRIPTION", "Test Description");

    let rendered = engine.render(loaded_template);
    assert!(rendered.contains("Test Feature"));
    assert!(rendered.contains("Test Description"));
}

#[test]
fn test_spec_kit_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let spec_kit = SpecKit::new(temp_dir.path()).unwrap();

    // Test with non-existent spec
    let result = spec_kit.get_spec("non-existent");
    assert!(result.is_err());

    // Test validation of non-existent file
    let non_existent_path = temp_dir.path().join("nonexistent.md");
    let result = spec_kit.validate_spec(&non_existent_path);
    assert!(result.is_err());

    // Test execution of non-existent file
    let result = spec_kit.execute_spec(&non_existent_path);
    assert!(result.is_err());
}

#[test]
fn test_spec_requirements_parsing() {
    let content = r#"
# Feature with Requirements

## Executive Summary
Feature with detailed requirements.

## Requirements
### User Requirements
- [ ] Users can create items (High)
- [ ] Users can edit items (Medium)
- [ ] Users can delete items (Low)

### System Requirements
- [ ] System handles 1000 RPS (Critical)
- [ ] Data is persisted (High)

## Success Criteria
- [ ] All functionality works
- [ ] Performance targets met

## Technical Requirements
### Architecture
- [ ] Microservice architecture

## Implementation Plan
### Stage 1: Setup
**Goal**: Initial setup
**Success Criteria**: Setup complete

## Testing Strategy
### Unit Tests
- [ ] Core functionality

## Acceptance Criteria
- [ ] Requirements met
"#;

    let spec = Spec::from_markdown(content).unwrap();

    // Check that requirements were parsed
    assert!(!spec.requirements.is_empty());

    // Should have user and system requirements
    let user_reqs: Vec<_> = spec.requirements.iter()
        .filter(|r| r.category == RequirementCategory::User)
        .collect();
    let system_reqs: Vec<_> = spec.requirements.iter()
        .filter(|r| r.category == RequirementCategory::System)
        .collect();

    assert!(!user_reqs.is_empty());
    assert!(!system_reqs.is_empty());
}

#[test]
fn test_spec_implementation_plan() {
    let content = r#"
# Multi-stage Feature

## Executive Summary
Feature with multiple implementation stages.

## Requirements
### User Requirements
- [ ] User requirement

## Success Criteria
- [ ] Success criterion

## Technical Requirements
### Architecture
- [ ] Architecture requirement

## Implementation Plan
### Stage 1: Research
**Goal**: Research requirements
**Success Criteria**: Requirements documented

### Stage 2: Development
**Goal**: Develop feature
**Success Criteria**: Feature developed

### Stage 3: Testing
**Goal**: Test feature
**Success Criteria**: All tests pass

## Testing Strategy
### Unit Tests
- [ ] Unit test

## Acceptance Criteria
- [ ] Acceptance criterion
"#;

    let spec = Spec::from_markdown(content).unwrap();

    // Should have 3 implementation stages
    assert_eq!(spec.implementation_plan.len(), 3);

    // Check stages are properly ordered
    assert_eq!(spec.implementation_plan[0].stage, 1);
    assert_eq!(spec.implementation_plan[1].stage, 2);
    assert_eq!(spec.implementation_plan[2].stage, 3);

    // Check stage names
    assert!(spec.implementation_plan[0].name.contains("Research"));
    assert!(spec.implementation_plan[1].name.contains("Development"));
    assert!(spec.implementation_plan[2].name.contains("Testing"));
}

#[test]
fn test_spec_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let spec_kit = SpecKit::new(temp_dir.path()).unwrap();

    let description = "Feature with metadata requirements";
    let spec = spec_kit.generate_spec(description).unwrap();

    // Check that metadata includes generator info
    assert!(spec.metadata.contains_key("description"));
    assert!(spec.metadata.contains_key("generator"));
    assert!(spec.metadata.contains_key("version"));

    assert_eq!(spec.metadata["description"], description);
    assert_eq!(spec.metadata["generator"], "spec_kit");
    assert_eq!(spec.metadata["version"], "1.0");
}

#[test]
fn test_spec_markdown_roundtrip() {
    let original_content = r#"
# Original Feature

## Executive Summary
Original feature description.

## Requirements
### User Requirements
- [ ] User can use feature

## Success Criteria
- [ ] Feature works

## Technical Requirements
### Architecture
- [ ] Good architecture

## Implementation Plan
### Stage 1: Implementation
**Goal**: Implement feature
**Success Criteria**: Feature implemented

## Testing Strategy
### Unit Tests
- [ ] Tests pass

## Acceptance Criteria
- [ ] All criteria met
"#;

    let spec = Spec::from_markdown(original_content).unwrap();
    let markdown = spec.to_markdown();

    // Should contain key elements
    assert!(markdown.contains("Original Feature"));
    assert!(markdown.contains("Executive Summary"));
    assert!(markdown.contains("Requirements"));
    assert!(markdown.contains("Success Criteria"));
    assert!(markdown.contains("Implementation Plan"));
    assert!(markdown.contains("Testing Strategy"));
    assert!(markdown.contains("Acceptance Criteria"));
}