//! Specification validation functionality

use super::*;
use anyhow::{Result, Context};
use std::path::Path;

/// Validates specifications for completeness and consistency
pub struct SpecValidator {
    // Could add configuration for validation rules
}

impl SpecValidator {
    /// Create a new specification validator
    pub fn new() -> Self {
        Self {}
    }

    /// Validate a specification file
    pub fn validate(&self, spec_path: &Path) -> Result<ValidationResult> {
        let spec_content = std::fs::read_to_string(spec_path)
            .context("Failed to read specification file")?;

        let mut errors = Vec::new();
        let mut warnings = Vec::new();
        let mut suggestions = Vec::new();

        // Validate basic structure
        self.validate_structure(&spec_content, &mut errors, &mut warnings);

        // Validate required sections
        self.validate_required_sections(&spec_content, &mut errors);

        // Validate content quality
        self.validate_content_quality(&spec_content, &mut warnings, &mut suggestions);

        // Validate requirements
        self.validate_requirements(&spec_content, &mut errors, &mut warnings, &mut suggestions);

        // Validate success criteria
        self.validate_success_criteria(&spec_content, &mut errors, &mut warnings);

        // Validate implementation plan
        self.validate_implementation_plan(&spec_content, &mut errors, &mut warnings, &mut suggestions);

        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            suggestions,
        })
    }

    fn validate_structure(&self, content: &str, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
        // Check if content starts with a title
        if !content.lines().next().map_or(false, |line| line.starts_with("# ")) {
            errors.push("Specification must start with a title (# Title)".to_string());
        }

        // Check for markdown formatting consistency
        let lines: Vec<&str> = content.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.starts_with("## ") && i > 0 && lines[i-1].trim() != "" {
                warnings.push(format!("Line {}: Section header should be preceded by empty line", i+1));
            }
        }
    }

    fn validate_required_sections(&self, content: &str, errors: &mut Vec<String>) {
        let required_sections = vec![
            "Executive Summary",
            "Requirements",
            "Success Criteria",
            "Technical Requirements",
            "Implementation Plan",
            "Testing Strategy",
            "Acceptance Criteria"
        ];

        for section in required_sections {
            if !content.contains(&format!("## {}", section)) {
                errors.push(format!("Missing required section: {}", section));
            }
        }
    }

    fn validate_content_quality(&self, content: &str, warnings: &mut Vec<String>, suggestions: &mut Vec<String>) {
        // Check for empty sections
        let sections: Vec<&str> = content.split("## ").collect();
        for (i, section) in sections.iter().enumerate() {
            if i == 0 { continue; } // Skip content before first section

            let lines: Vec<&str> = section.lines().collect();
            let section_name = lines.first().unwrap_or(&"").trim();

            // Skip if this is the beginning of another section
            if section_name.is_empty() { continue; }

            let content_lines: Vec<&str> = lines[1..].iter()
                .take_while(|line| !line.starts_with("## "))
                .copied()
                .collect();

            let content_text = content_lines.join(" ").trim();

            if content_text.is_empty() || content_text.len() < 10 {
                warnings.push(format!("Section '{}' appears to be empty or too short", section_name));
            }
        }

        // Check for specific quality indicators
        if !content.contains("## Security") {
            suggestions.push("Consider adding a Security section for better security coverage".to_string());
        }

        if !content.contains("## Performance") {
            suggestions.push("Consider adding a Performance section for performance requirements".to_string());
        }

        // Check for measurable success criteria
        let success_criteria_section = self.extract_section_content(content, "Success Criteria");
        if let Some(criteria) = success_criteria_section {
            let has_measureable = criteria.contains("ms") ||
                                 criteria.contains("%") ||
                                 criteria.contains("<") ||
                                 criteria.contains(">") ||
                                 criteria.contains("seconds") ||
                                 criteria.contains("requests") ||
                                 criteria.contains("users");

            if !has_measureable {
                suggestions.push("Success criteria should include measurable metrics where possible".to_string());
            }
        }
    }

    fn validate_requirements(&self, content: &str, errors: &mut Vec<String>, warnings: &mut Vec<String>, suggestions: &mut Vec<String>) {
        // Check for requirements section and its subsections
        if !content.contains("### User Requirements") {
            warnings.push("Consider adding User Requirements subsection".to_string());
        }

        if !content.contains("### System Requirements") {
            warnings.push("Consider adding System Requirements subsection".to_string());
        }

        // Check for requirement format consistency
        let requirement_lines: Vec<&str> = content.lines()
            .filter(|line| line.trim().starts_with("- [ ]"))
            .collect();

        if requirement_lines.is_empty() {
            errors.push("No requirements found in checklist format (- [ ] Description)".to_string());
        }

        // Check for requirement priority indicators
        let has_priority = content.contains("(High)") ||
                          content.contains("(Medium)") ||
                          content.contains("(Low)") ||
                          content.contains("(Critical)");

        if !has_priority && !requirement_lines.is_empty() {
            suggestions.push("Consider adding priority indicators to requirements".to_string());
        }
    }

    fn validate_success_criteria(&self, content: &str, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
        let criteria_section = self.extract_section_content(content, "Success Criteria");

        if let Some(criteria) = criteria_section {
            let criteria_count = criteria.lines()
                .filter(|line| line.trim().starts_with("- [ ]"))
                .count();

            if criteria_count < 3 {
                warnings.push("Consider adding more success criteria for better definition".to_string());
            }

            if criteria_count > 10 {
                warnings.push("Consider consolidating success criteria to focus on key outcomes".to_string());
            }

            // Check for testable criteria
            let testable_keywords = vec![
                "must", "should", "will", "able to", "can",
                "pass", "meet", "achieve", "complete", "implement"
            ];

            let has_testable = testable_keywords.iter().any(|keyword|
                criteria.to_lowercase().contains(keyword)
            );

            if !has_testable {
                errors.push("Success criteria should be testable and include action verbs".to_string());
            }
        }
    }

    fn validate_implementation_plan(&self, content: &str, errors: &mut Vec<String>, warnings: &mut Vec<String>, suggestions: &mut Vec<String>) {
        let plan_section = self.extract_section_content(content, "Implementation Plan");

        if let Some(plan) = plan_section {
            // Check for stages
            let has_stages = plan.contains("Stage 1") ||
                            plan.contains("### Stage") ||
                            plan.contains("Stage:");

            if !has_stages {
                errors.push("Implementation plan should include clearly defined stages".to_string());
            }

            // Check for duration estimates
            let has_duration = plan.contains("hours") ||
                             plan.contains("days") ||
                             plan.contains("weeks");

            if !has_duration {
                warnings.push("Consider adding duration estimates to implementation stages".to_string());
            }

            // Check for success criteria in stages
            if !plan.contains("Success Criteria") {
                warnings.push("Implementation stages should have success criteria".to_string());
            }

            // Check for logical progression
            let stages: Vec<&str> = plan.split("### Stage").collect();
            if stages.len() < 3 { // Less than 2 stages (split creates n+1 parts)
                suggestions.push("Consider breaking implementation into multiple stages for better planning".to_string());
            }
        }
    }

    fn extract_section_content(&self, content: &str, section_name: &str) -> Option<String> {
        let lines: Vec<&str> = content.lines().collect();

        let start_idx = lines.iter().position(|line| line.trim() == format!("## {}", section_name))?;
        let end_idx = lines.iter()
            .skip(start_idx + 1)
            .position(|line| line.starts_with("## "))
            .map(|i| start_idx + 1 + i)
            .unwrap_or(lines.len());

        Some(lines[start_idx + 1..end_idx].join("\n"))
    }

    /// Validate multiple specification files
    pub fn validate_multiple(&self, spec_paths: &[&Path]) -> Vec<(String, ValidationResult)> {
        spec_paths.iter()
            .filter_map(|path| {
                match self.validate(path) {
                    Ok(result) => Some((path.to_string_lossy().to_string(), result)),
                    Err(e) => Some((path.to_string_lossy().to_string(), ValidationResult {
                        is_valid: false,
                        errors: vec![format!("Validation failed: {}", e)],
                        warnings: vec![],
                        suggestions: vec![],
                    })),
                }
            })
            .collect()
    }

    /// Generate a validation report
    pub fn generate_report(&self, results: &[(String, ValidationResult)]) -> String {
        let mut report = String::new();
        let total_specs = results.len();
        let valid_specs = results.iter().filter(|(_, r)| r.is_valid).count();
        let error_count = results.iter().map(|(_, r)| r.errors.len()).sum::<usize>();
        let warning_count = results.iter().map(|(_, r)| r.warnings.len()).sum::<usize>();

        report.push_str(&format!(
            "# Specification Validation Report\n\n\
             **Total Specifications**: {}\n\
             **Valid Specifications**: {}\n\
             **Invalid Specifications**: {}\n\
             **Total Errors**: {}\n\
             **Total Warnings**: {}\n\n",
            total_specs,
            valid_specs,
            total_specs - valid_specs,
            error_count,
            warning_count
        ));

        for (file_path, result) in results {
            report.push_str(&format!(
                "## {}\n\n\
                 **Status**: {}\n\
                 **Errors**: {}\n\
                 **Warnings**: {}\n\n",
                file_path,
                if result.is_valid { "✅ Valid" } else { "❌ Invalid" },
                result.errors.len(),
                result.warnings.len()
            ));

            if !result.errors.is_empty() {
                report.push_str("**Errors**:\n");
                for error in &result.errors {
                    report.push_str(&format!("- ❌ {}\n", error));
                }
                report.push('\n');
            }

            if !result.warnings.is_empty() {
                report.push_str("**Warnings**:\n");
                for warning in &result.warnings {
                    report.push_str(&format!("- ⚠️ {}\n", warning));
                }
                report.push('\n');
            }

            if !result.suggestions.is_empty() {
                report.push_str("**Suggestions**:\n");
                for suggestion in &result.suggestions {
                    report.push_str(&format!("- 💡 {}\n", suggestion));
                }
                report.push('\n');
            }
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_validate_valid_spec() {
        let validator = SpecValidator::new();
        let spec_content = r#"
# Test Specification

## Executive Summary
This is a test specification.

## Requirements
### User Requirements
- [ ] User must be able to use the feature

## Success Criteria
- [ ] Feature must work correctly

## Technical Requirements
### Architecture
- [ ] Must follow patterns

## Implementation Plan
### Stage 1: Foundation
**Goal**: Set up foundation
**Success Criteria**: - Foundation is ready

## Testing Strategy
### Unit Tests
- [ ] Core functionality

## Acceptance Criteria
- [ ] All requirements met
"#;

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", spec_content).unwrap();

        let result = validator.validate(temp_file.path()).unwrap();
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validate_missing_sections() {
        let validator = SpecValidator::new();
        let spec_content = "# Test\n\nJust some content.";

        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", spec_content).unwrap();

        let result = validator.validate(temp_file.path()).unwrap();
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
        assert!(result.errors.iter().any(|e| e.contains("Missing required section")));
    }
}