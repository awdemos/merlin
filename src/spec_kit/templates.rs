//! Template management for specifications

use super::*;
use anyhow::Result;
use std::path::Path;

/// Template management system
pub struct TemplateManager {
    templates_dir: PathBuf,
}

impl TemplateManager {
    /// Create a new template manager
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Result<Self> {
        let templates_dir = base_dir.as_ref().join("templates");
        Ok(Self { templates_dir })
    }

    /// Get all available templates
    pub fn list_templates(&self) -> Result<Vec<TemplateInfo>> {
        let mut templates = Vec::new();

        if !self.templates_dir.exists() {
            return Ok(templates);
        }

        for entry in std::fs::read_dir(&self.templates_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if let Some(info) = self.load_template_info(&path)? {
                    templates.push(info);
                }
            }
        }

        Ok(templates)
    }

    /// Load a specific template
    pub fn load_template(&self, name: &str) -> Result<String> {
        let template_path = self.templates_dir.join(format!("{}.md", name));
        std::fs::read_to_string(template_path)
            .map_err(|_| anyhow::anyhow!("Template '{}' not found", name))
    }

    /// Create a new template
    pub fn create_template(&self, name: &str, content: &str) -> Result<()> {
        let template_path = self.templates_dir.join(format!("{}.md", name));
        std::fs::write(template_path, content)?;
        Ok(())
    }

    /// Update an existing template
    pub fn update_template(&self, name: &str, content: &str) -> Result<()> {
        let template_path = self.templates_dir.join(format!("{}.md", name));
        if !template_path.exists() {
            return Err(anyhow::anyhow!("Template '{}' not found", name));
        }

        std::fs::write(template_path, content)?;
        Ok(())
    }

    /// Delete a template
    pub fn delete_template(&self, name: &str) -> Result<()> {
        let template_path = self.templates_dir.join(format!("{}.md", name));
        std::fs::remove_file(template_path)?;
        Ok(())
    }

    /// Load template information
    fn load_template_info(&self, path: &Path) -> Result<Option<TemplateInfo>> {
        let content = std::fs::read_to_string(path)?;
        let file_name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        let description = self.extract_description(&content)?;
        let category = self.extract_category(&content)?;

        Ok(Some(TemplateInfo {
            name: file_name.to_string(),
            description,
            category,
            path: path.to_path_buf(),
        }))
    }

    /// Extract template description
    fn extract_description(&self, content: &str) -> Result<String> {
        if let Some(desc_start) = content.find("## Description") {
            if let Some(desc_end) = content[desc_start..].find('\n', desc_start + 14) {
                return Ok(content[desc_start + 14..desc_start + desc_end].trim().to_string());
            }
        }
        Ok("No description available".to_string())
    }

    /// Extract template category
    fn extract_category(&self, content: &str) -> Result<String> {
        if let Some(cat_start) = content.find("## Category") {
            if let Some(cat_end) = content[cat_start..].find('\n', cat_start + 11) {
                return Ok(content[cat_start + 11..cat_start + cat_end].trim().to_string());
            }
        }
        Ok("general".to_string())
    }
}

/// Template information
#[derive(Debug, Clone)]
pub struct TemplateInfo {
    pub name: String,
    pub description: String,
    pub category: String,
    pub path: PathBuf,
}

/// Template engine for variable substitution
pub struct TemplateEngine {
    variables: HashMap<String, String>,
}

impl TemplateEngine {
    /// Create a new template engine
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    /// Add a variable
    pub fn add_variable(&mut self, key: &str, value: &str) {
        self.variables.insert(key.to_string(), value.to_string());
    }

    /// Add multiple variables
    pub fn add_variables(&mut self, vars: &[(&str, &str)]) {
        for (key, value) in vars {
            self.add_variable(key, value);
        }
    }

    /// Render a template with variables
    pub fn render(&self, template: &str) -> String {
        let mut result = template.to_string();

        for (key, value) in &self.variables {
            let placeholder = format!("${{{}}}", key);
            result = result.replace(&placeholder, value);
        }

        result
    }

    /// Clear all variables
    pub fn clear(&mut self) {
        self.variables.clear();
    }
}

/// Pre-defined template variables
pub struct TemplateVariables;

impl TemplateVariables {
    /// Get common template variables
    pub fn common_variables(spec: &Spec) -> Vec<(&'static str, String)> {
        vec![
            ("TITLE", spec.title.clone()),
            ("ID", spec.id.clone()),
            ("DESCRIPTION", spec.description.clone()),
            ("EXECUTIVE_SUMMARY", spec.executive_summary.clone()),
            ("CREATED_AT", spec.created_at.to_rfc3339()),
            ("UPDATED_AT", spec.updated_at.to_rfc3339()),
            ("STATUS", format!("{:?}", spec.status)),
        ]
    }

    /// Get implementation plan variables
    pub fn implementation_variables(stages: &[ImplementationStage]) -> Vec<(&'static str, String)> {
        let stages_content = stages
            .iter()
            .map(|stage| format!(
                "### Stage {}: {}\n**Goal**: {}\n**Success Criteria**: {}\n",
                stage.stage,
                stage.name,
                stage.goal,
                stage.success_criteria.join(", ")
            ))
            .collect::<Vec<_>>()
            .join("\n\n");

        let total_duration = stages
            .iter()
            .map(|s| s.estimated_duration_hours.unwrap_or(0))
            .sum::<u32>();

        vec![
            ("IMPLEMENTATION_STAGES", stages_content),
            ("TOTAL_DURATION", total_duration.to_string()),
            ("STAGE_COUNT", stages.len().to_string()),
        ]
    }

    /// Get requirements variables
    pub fn requirements_variables(requirements: &[Requirement]) -> Vec<(&'static str, String)> {
        let user_reqs = requirements
            .iter()
            .filter(|r| r.category == RequirementCategory::User)
            .map(|r| format!("- [ ] {}", r.description))
            .collect::<Vec<_>>()
            .join("\n");

        let system_reqs = requirements
            .iter()
            .filter(|r| r.category == RequirementCategory::System)
            .map(|r| format!("- [ ] {}", r.description))
            .collect::<Vec<_>>()
            .join("\n");

        vec![
            ("USER_REQUIREMENTS", user_reqs),
            ("SYSTEM_REQUIREMENTS", system_reqs),
            ("REQUIREMENTS_COUNT", requirements.len().to_string()),
        ]
    }

    /// Get testing variables
    pub fn testing_variables(testing: &TestingStrategy) -> Vec<(&'static str, String)> {
        let unit_tests = testing.unit_tests
            .iter()
            .map(|t| format!("- [ ] {}", t))
            .collect::<Vec<_>>()
            .join("\n");

        let integration_tests = testing.integration_tests
            .iter()
            .map(|t| format!("- [ ] {}", t))
            .collect::<Vec<_>>()
            .join("\n");

        vec![
            ("UNIT_TESTS", unit_tests),
            ("INTEGRATION_TESTS", integration_tests),
            ("TOTAL_TESTS", (testing.unit_tests.len() + testing.integration_tests.len()).to_string()),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_template_engine() {
        let mut engine = TemplateEngine::new();
        engine.add_variable("TITLE", "Test Feature");
        engine.add_variable("DESCRIPTION", "A test feature");

        let template = "# ${TITLE}\n\n${DESCRIPTION}";
        let result = engine.render(template);

        assert_eq!(result, "# Test Feature\n\nA test feature");
    }

    #[test]
    fn test_template_manager() {
        let temp_dir = TempDir::new().unwrap();
        let templates_dir = temp_dir.path().join("templates");
        std::fs::create_dir_all(&templates_dir).unwrap();

        // Create a test template
        let template_content = r#"## Description
Test template

## Category
test

# ${TITLE}

${DESCRIPTION}
"#;
        std::fs::write(templates_dir.join("test.md"), template_content).unwrap();

        let manager = TemplateManager::new(temp_dir.path()).unwrap();
        let templates = manager.list_templates().unwrap();

        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].name, "test");
        assert_eq!(templates[0].description, "Test template");
        assert_eq!(templates[0].category, "test");

        // Test loading template
        let loaded = manager.load_template("test").unwrap();
        assert!(loaded.contains("Test template"));
    }

    #[test]
    fn test_template_variables() {
        let spec = Spec {
            id: "test-id".to_string(),
            title: "Test Feature".to_string(),
            description: "Test description".to_string(),
            executive_summary: "Test summary".to_string(),
            requirements: vec![],
            success_criteria: vec![],
            technical_requirements: TechnicalRequirements {
                architecture: vec![],
                performance: vec![],
                security: vec![],
                scalability: vec![],
                maintainability: vec![],
            },
            dependencies: Dependencies {
                external: vec![],
                internal: vec![],
                version_constraints: HashMap::new(),
            },
            implementation_plan: vec![],
            testing_strategy: TestingStrategy {
                unit_tests: vec!["Test 1".to_string()],
                integration_tests: vec!["Test 2".to_string()],
                e2e_tests: vec![],
                performance_tests: vec![],
                security_tests: vec![],
            },
            documentation_requirements: vec![],
            acceptance_criteria: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            status: SpecStatus::Draft,
            metadata: HashMap::new(),
        };

        let vars = TemplateVariables::common_variables(&spec);
        assert_eq!(vars.len(), 6);
        assert_eq!(vars[0].0, "TITLE");
        assert_eq!(vars[0].1, "Test Feature");

        let test_vars = TemplateVariables::testing_variables(&spec.testing_strategy);
        assert_eq!(test_vars.len(), 3);
        assert!(test_vars[0].1.contains("Test 1"));
        assert!(test_vars[1].1.contains("Test 2"));
    }
}