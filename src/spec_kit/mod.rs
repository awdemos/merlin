//! Specification Kit for Merlin AI Routing
//!
//! This module provides tools for creating, managing, and executing
//! feature specifications in a structured, test-driven manner.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub mod generator;
pub mod validator;
pub mod executor;
pub mod templates;

pub use generator::SpecGenerator;
pub use validator::SpecValidator;
pub use executor::SpecExecutor;

/// Main specification kit that coordinates all components
#[derive(Debug, Clone)]
pub struct SpecKit {
    base_dir: PathBuf,
    templates: SpecTemplates,
}

impl SpecKit {
    /// Create a new spec kit instance
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        let templates = SpecTemplates::new(&base_dir)?;

        Ok(Self {
            base_dir,
            templates,
        })
    }

    /// Generate a new specification from a feature description
    pub fn generate_spec(&self, description: &str) -> Result<Spec> {
        let generator = SpecGenerator::new(self.templates.clone());
        generator.generate(description)
    }

    /// Validate an existing specification
    pub fn validate_spec(&self, spec_path: &Path) -> Result<ValidationResult> {
        let validator = SpecValidator::new();
        validator.validate(spec_path)
    }

    /// Execute a specification implementation
    pub fn execute_spec(&self, spec_path: &Path) -> Result<ExecutionResult> {
        let executor = SpecExecutor::new(&self.base_dir);
        executor.execute(spec_path)
    }

    /// List all specifications
    pub fn list_specs(&self) -> Result<Vec<Spec>> {
        self.load_specs()
    }

    /// Get a specific specification by ID
    pub fn get_spec(&self, spec_id: &str) -> Result<Spec> {
        let spec_path = self.base_dir.join(spec_id).join("spec.md");
        self.load_spec(&spec_path)
    }

    /// Load all specifications from the specs directory
    fn load_specs(&self) -> Result<Vec<Spec>> {
        let mut specs = Vec::new();

        if !self.base_dir.exists() {
            return Ok(specs);
        }

        for entry in fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                if let Some(spec) = self.load_spec_from_dir(&path)? {
                    specs.push(spec);
                }
            }
        }

        Ok(specs)
    }

    /// Load a specification from a directory
    fn load_spec_from_dir(&self, dir_path: &Path) -> Result<Option<Spec>> {
        let spec_file = dir_path.join("spec.md");
        if !spec_file.exists() {
            return Ok(None);
        }

        self.load_spec(&spec_file).map(Some)
    }

    /// Load a specification from a file
    fn load_spec(&self, spec_path: &Path) -> Result<Spec> {
        let content = fs::read_to_string(spec_path)?;
        Spec::from_markdown(&content)
    }
}

/// Complete specification data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub id: String,
    pub title: String,
    pub description: String,
    pub executive_summary: String,
    pub requirements: Vec<Requirement>,
    pub success_criteria: Vec<String>,
    pub technical_requirements: TechnicalRequirements,
    pub dependencies: Dependencies,
    pub implementation_plan: Vec<ImplementationStage>,
    pub testing_strategy: TestingStrategy,
    pub documentation_requirements: Vec<String>,
    pub acceptance_criteria: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: SpecStatus,
    pub metadata: HashMap<String, String>,
}

/// Specification requirement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Requirement {
    pub category: RequirementCategory,
    pub description: String,
    pub is_mandatory: bool,
    pub priority: Priority,
}

/// Requirement category
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RequirementCategory {
    User,
    System,
    Technical,
    Security,
    Performance,
}

/// Priority level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// Technical requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechnicalRequirements {
    pub architecture: Vec<String>,
    pub performance: Vec<String>,
    pub security: Vec<String>,
    pub scalability: Vec<String>,
    pub maintainability: Vec<String>,
}

/// Dependencies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependencies {
    pub external: Vec<String>,
    pub internal: Vec<String>,
    pub version_constraints: HashMap<String, String>,
}

/// Implementation stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationStage {
    pub stage: u32,
    pub name: String,
    pub goal: String,
    pub success_criteria: Vec<String>,
    pub tasks: Vec<Task>,
    pub estimated_duration_hours: Option<u32>,
}

/// Task within an implementation stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub description: String,
    pub assignee: Option<String>,
    pub estimated_hours: Option<u32>,
    pub dependencies: Vec<String>,
    pub status: TaskStatus,
}

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    InProgress,
    Completed,
    Blocked,
}

/// Testing strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestingStrategy {
    pub unit_tests: Vec<String>,
    pub integration_tests: Vec<String>,
    pub e2e_tests: Vec<String>,
    pub performance_tests: Vec<String>,
    pub security_tests: Vec<String>,
}

/// Specification status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpecStatus {
    Draft,
    Review,
    Approved,
    InProgress,
    Completed,
    Rejected,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub suggestions: Vec<String>,
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub output: String,
    pub artifacts: Vec<PathBuf>,
    pub duration_seconds: u32,
    pub error: Option<String>,
}

/// Template management
#[derive(Debug, Clone)]
pub struct SpecTemplates {
    spec_template: String,
    plan_template: String,
    tasks_template: String,
}

impl SpecTemplates {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Result<Self> {
        let templates_dir = base_dir.as_ref().parent()
            .ok_or_else(|| anyhow!("Cannot determine templates directory"))?
            .join("templates");

        Ok(Self {
            spec_template: fs::read_to_string(templates_dir.join("spec.md"))?,
            plan_template: fs::read_to_string(templates_dir.join("plan.md"))?,
            tasks_template: fs::read_to_string(templates_dir.join("tasks.md"))?,
        })
    }

    pub fn get_spec_template(&self) -> &str {
        &self.spec_template
    }

    pub fn get_plan_template(&self) -> &str {
        &self.plan_template
    }

    pub fn get_tasks_template(&self) -> &str {
        &self.tasks_template
    }
}

impl Spec {
    /// Create a new specification from markdown content
    pub fn from_markdown(content: &str) -> Result<Self> {
        // Parse markdown content and create spec structure
        // This is a simplified implementation - in production you'd use a proper markdown parser
        let id = Uuid::new_v4().to_string();
        let title = Self::extract_title(content).unwrap_or_else(|| "Untitled Spec".to_string());
        let executive_summary = Self::extract_section(content, "Executive Summary")
            .unwrap_or_else(|| "".to_string());

        Ok(Self {
            id,
            title,
            description: executive_summary.clone(),
            executive_summary,
            requirements: Self::parse_requirements(content)?,
            success_criteria: Self::extract_list_items(content, "Success Criteria"),
            technical_requirements: Self::parse_technical_requirements(content)?,
            dependencies: Self::parse_dependencies(content)?,
            implementation_plan: Self::parse_implementation_plan(content)?,
            testing_strategy: Self::parse_testing_strategy(content)?,
            documentation_requirements: Self::extract_list_items(content, "Documentation Requirements"),
            acceptance_criteria: Self::extract_list_items(content, "Acceptance Criteria"),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            status: SpecStatus::Draft,
            metadata: HashMap::new(),
        })
    }

    /// Convert specification to markdown format
    pub fn to_markdown(&self) -> String {
        // Convert spec structure back to markdown
        // This is a simplified implementation
        format!(
            "# {}\n\n## Executive Summary\n{}\n\n## Requirements\n### User Requirements\n{}\
             \n\n## Success Criteria\n{}\
             \n\n## Implementation Plan\n{}\
             \n\n## Testing Strategy\n{}\
             \n\n## Acceptance Criteria\n{}",
            self.title,
            self.executive_summary,
            self.requirements.iter()
                .filter(|r| r.category == RequirementCategory::User)
                .map(|r| format!("- [ ] {}", r.description))
                .collect::<Vec<_>>()
                .join("\n"),
            self.success_criteria.iter()
                .map(|c| format!("- [ ] {}", c))
                .collect::<Vec<_>>()
                .join("\n"),
            self.implementation_plan.iter()
                .map(|s| format!("### Stage {}: {}\n**Goal**: {}\n**Success Criteria**: {}",
                    s.stage, s.name, s.goal,
                    s.success_criteria.iter()
                        .map(|c| format!("- {}", c))
                        .collect::<Vec<_>>()
                        .join(" ")))
                .collect::<Vec<_>>()
                .join("\n\n"),
            self.testing_strategy.unit_tests.iter()
                .map(|t| format!("- [ ] {}", t))
                .collect::<Vec<_>>()
                .join("\n"),
            self.acceptance_criteria.iter()
                .map(|c| format!("- [ ] {}", c))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    fn extract_title(content: &str) -> Option<String> {
        content.lines()
            .find(|line| line.starts_with("# "))
            .map(|line| line[2..].trim().to_string())
    }

    fn extract_section(content: &str, section_name: &str) -> Option<String> {
        content.lines()
            .skip_while(|line| !line.starts_with(&format!("## {}", section_name)))
            .skip(1)
            .take_while(|line| !line.starts_with("## ") && !line.starts_with("# "))
            .map(|line| line.trim())
            .collect::<Vec<_>>()
            .join("\n")
            .into()
    }

    fn extract_list_items(content: &str, section_name: &str) -> Vec<String> {
        content.lines()
            .skip_while(|line| !line.starts_with(&format!("## {}", section_name)))
            .skip(1)
            .take_while(|line| !line.starts_with("## ") && !line.starts_with("# "))
            .filter(|line| line.starts_with("- [ ]"))
            .map(|line| line[5..].trim().to_string())
            .collect()
    }

    fn parse_requirements(content: &str) -> Result<Vec<Requirement>> {
        // Simplified requirement parsing
        Ok(vec![
            Requirement {
                category: RequirementCategory::User,
                description: "Feature must be accessible via API".to_string(),
                is_mandatory: true,
                priority: Priority::High,
            }
        ])
    }

    fn parse_technical_requirements(content: &str) -> Result<TechnicalRequirements> {
        Ok(TechnicalRequirements {
            architecture: vec!["Must follow existing patterns".to_string()],
            performance: vec!["Response time < 100ms".to_string()],
            security: vec!["No hardcoded secrets".to_string()],
            scalability: vec!["Handle 1000 RPS".to_string()],
            maintainability: vec!["80% test coverage".to_string()],
        })
    }

    fn parse_dependencies(content: &str) -> Result<Dependencies> {
        Ok(Dependencies {
            external: vec![],
            internal: vec![],
            version_constraints: HashMap::new(),
        })
    }

    fn parse_implementation_plan(content: &str) -> Result<Vec<ImplementationStage>> {
        Ok(vec![
            ImplementationStage {
                stage: 1,
                name: "Foundation".to_string(),
                goal: "Set up basic structure".to_string(),
                success_criteria: vec!["All files created".to_string()],
                tasks: vec![],
                estimated_duration_hours: Some(4),
            }
        ])
    }

    fn parse_testing_strategy(content: &str) -> Result<TestingStrategy> {
        Ok(TestingStrategy {
            unit_tests: vec!["Core functionality".to_string()],
            integration_tests: vec!["API endpoints".to_string()],
            e2e_tests: vec![],
            performance_tests: vec![],
            security_tests: vec!["Input validation".to_string()],
        })
    }
}