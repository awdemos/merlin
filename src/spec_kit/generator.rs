//! Specification generator for creating new feature specifications

use super::*;
use anyhow::{Result, Context};

/// Generates new specifications from feature descriptions
pub struct SpecGenerator {
    templates: SpecTemplates,
}

impl SpecGenerator {
    /// Create a new specification generator
    pub fn new(templates: SpecTemplates) -> Self {
        Self { templates }
    }

    /// Generate a specification from a feature description
    pub fn generate(&self, description: &str) -> Result<Spec> {
        let title = self.extract_title(description);
        let executive_summary = self.generate_executive_summary(description);

        let requirements = self.generate_requirements(description);
        let success_criteria = self.generate_success_criteria(description);
        let technical_requirements = self.generate_technical_requirements(description);
        let dependencies = self.generate_dependencies(description);
        let implementation_plan = self.generate_implementation_plan(description);
        let testing_strategy = self.generate_testing_strategy(description);
        let documentation_requirements = self.generate_documentation_requirements(description);
        let acceptance_criteria = self.generate_acceptance_criteria(description);

        Ok(Spec {
            id: Uuid::new_v4().to_string(),
            title,
            description: executive_summary.clone(),
            executive_summary,
            requirements,
            success_criteria,
            technical_requirements,
            dependencies,
            implementation_plan,
            testing_strategy,
            documentation_requirements,
            acceptance_criteria,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            status: SpecStatus::Draft,
            metadata: self.generate_metadata(description),
        })
    }

    /// Create specification files in the filesystem
    pub fn create_spec_files(&self, spec: &Spec, output_dir: &Path) -> Result<()> {
        fs::create_dir_all(output_dir).context("Failed to create spec directory")?;

        // Create spec.md
        let spec_content = self.generate_spec_markdown(spec);
        fs::write(output_dir.join("spec.md"), spec_content)
            .context("Failed to write spec.md")?;

        // Create plan.md
        let plan_content = self.generate_plan_markdown(spec);
        fs::write(output_dir.join("plan.md"), plan_content)
            .context("Failed to write plan.md")?;

        // Create tasks.md
        let tasks_content = self.generate_tasks_markdown(spec);
        fs::write(output_dir.join("tasks.md"), tasks_content)
            .context("Failed to write tasks.md")?;

        Ok(())
    }

    fn extract_title(&self, description: &str) -> String {
        // Simple title extraction - take first sentence or phrase
        description
            .split('.')
            .next()
            .unwrap_or(description)
            .trim()
            .to_string()
    }

    fn generate_executive_summary(&self, description: &str) -> String {
        format!(
            "This feature will implement: {}",
            description.trim()
        )
    }

    fn generate_requirements(&self, _description: &str) -> Vec<Requirement> {
        vec![
            Requirement {
                category: RequirementCategory::User,
                description: "Feature must be easily accessible to users".to_string(),
                is_mandatory: true,
                priority: Priority::High,
            },
            Requirement {
                category: RequirementCategory::System,
                description: "Feature must integrate with existing system architecture".to_string(),
                is_mandatory: true,
                priority: Priority::High,
            },
            Requirement {
                category: RequirementCategory::Technical,
                description: "Feature must follow established coding standards".to_string(),
                is_mandatory: true,
                priority: Priority::Medium,
            },
        ]
    }

    fn generate_success_criteria(&self, _description: &str) -> Vec<String> {
        vec![
            "All acceptance criteria are met".to_string(),
            "Feature passes all tests".to_string(),
            "Documentation is complete and accurate".to_string(),
            "Performance benchmarks are achieved".to_string(),
        ]
    }

    fn generate_technical_requirements(&self, _description: &str) -> TechnicalRequirements {
        TechnicalRequirements {
            architecture: vec![
                "Must follow existing module structure".to_string(),
                "Use existing design patterns".to_string(),
            ],
            performance: vec![
                "Response time < 100ms for normal operations".to_string(),
                "Memory usage must be efficient".to_string(),
            ],
            security: vec![
                "Input validation on all external inputs".to_string(),
                "No hardcoded secrets or credentials".to_string(),
                "Follow security best practices".to_string(),
            ],
            scalability: vec![
                "Must handle expected load growth".to_string(),
                "Database queries must be optimized".to_string(),
            ],
            maintainability: vec![
                "Code must be well documented".to_string(),
                "Test coverage > 80%".to_string(),
                "Follow existing code style".to_string(),
            ],
        }
    }

    fn generate_dependencies(&self, _description: &str) -> Dependencies {
        Dependencies {
            external: vec![],
            internal: vec!["Core routing system".to_string()],
            version_constraints: HashMap::new(),
        }
    }

    fn generate_implementation_plan(&self, _description: &str) -> Vec<ImplementationStage> {
        vec![
            ImplementationStage {
                stage: 1,
                name: "Research and Planning".to_string(),
                goal: "Understand requirements and design solution".to_string(),
                success_criteria: vec![
                    "Requirements are clearly defined".to_string(),
                    "Technical approach is validated".to_string(),
                ],
                tasks: vec![
                    Task {
                        id: "research-requirements".to_string(),
                        description: "Research and document requirements".to_string(),
                        assignee: None,
                        estimated_hours: Some(4),
                        dependencies: vec![],
                        status: TaskStatus::Pending,
                    },
                ],
                estimated_duration_hours: Some(8),
            },
            ImplementationStage {
                stage: 2,
                name: "Core Implementation".to_string(),
                goal: "Implement main functionality".to_string(),
                success_criteria: vec![
                    "Core functionality is working".to_string(),
                    "All unit tests pass".to_string(),
                ],
                tasks: vec![
                    Task {
                        id: "implement-core".to_string(),
                        description: "Implement core feature logic".to_string(),
                        assignee: None,
                        estimated_hours: Some(16),
                        dependencies: vec!["research-requirements".to_string()],
                        status: TaskStatus::Pending,
                    },
                ],
                estimated_duration_hours: Some(24),
            },
            ImplementationStage {
                stage: 3,
                name: "Integration and Testing".to_string(),
                goal: "Integrate with existing system and test thoroughly".to_string(),
                success_criteria: vec![
                    "Feature integrates with existing system".to_string(),
                    "All tests pass including integration tests".to_string(),
                    "Performance requirements are met".to_string(),
                ],
                tasks: vec![
                    Task {
                        id: "integration-testing".to_string(),
                        description: "Perform integration testing".to_string(),
                        assignee: None,
                        estimated_hours: Some(8),
                        dependencies: vec!["implement-core".to_string()],
                        status: TaskStatus::Pending,
                    },
                ],
                estimated_duration_hours: Some(16),
            },
        ]
    }

    fn generate_testing_strategy(&self, _description: &str) -> TestingStrategy {
        TestingStrategy {
            unit_tests: vec![
                "Core functionality tests".to_string(),
                "Edge case handling".to_string(),
                "Error handling scenarios".to_string(),
            ],
            integration_tests: vec![
                "API endpoint integration".to_string(),
                "Database integration".to_string(),
                "External service integration".to_string(),
            ],
            e2e_tests: vec![
                "Complete user workflow".to_string(),
                "Error scenarios".to_string(),
            ],
            performance_tests: vec![
                "Load testing".to_string(),
                "Stress testing".to_string(),
            ],
            security_tests: vec![
                "Input validation".to_string(),
                "Authorization testing".to_string(),
                "Injection attack testing".to_string(),
            ],
        }
    }

    fn generate_documentation_requirements(&self, _description: &str) -> Vec<String> {
        vec![
            "User documentation".to_string(),
            "API documentation".to_string(),
            "Technical design document".to_string(),
            "Deployment guide".to_string(),
        ]
    }

    fn generate_acceptance_criteria(&self, _description: &str) -> Vec<String> {
        vec![
            "Feature works as expected in all scenarios".to_string(),
            "All edge cases are handled properly".to_string(),
            "Feature meets performance requirements".to_string(),
            "Documentation is complete and accurate".to_string(),
            "Feature is accessible and user-friendly".to_string(),
        ]
    }

    fn generate_metadata(&self, description: &str) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert("description".to_string(), description.to_string());
        metadata.insert("generator".to_string(), "spec_kit".to_string());
        metadata.insert("version".to_string(), "1.0".to_string());
        metadata
    }

    fn generate_spec_markdown(&self, spec: &Spec) -> String {
        format!(
            "# {}\n\n## Executive Summary\n{}\n\n## Requirements\n\
            ### User Requirements\n{}\
            \n\n### System Requirements\n{}\
            \n\n## Success Criteria\n{}\
            \n\n## Technical Requirements\n\
            ### Architecture\n{}\
            \n### Performance\n{}\
            \n### Security\n{}\
            \n\n## Dependencies\n\
            ### External Dependencies\n{}\
            \n### Internal Dependencies\n{}\
            \n\n## Implementation Plan\n{}\
            \n\n## Testing Strategy\n\
            ### Unit Tests\n{}\
            \n### Integration Tests\n{}\
            \n\n## Documentation Requirements\n{}\
            \n\n## Acceptance Criteria\n{}",
            spec.title,
            spec.executive_summary,
            spec.requirements
                .iter()
                .filter(|r| r.category == RequirementCategory::User)
                .map(|r| format!("- [ ] {}", r.description))
                .collect::<Vec<_>>()
                .join("\n"),
            spec.requirements
                .iter()
                .filter(|r| r.category == RequirementCategory::System)
                .map(|r| format!("- [ ] {}", r.description))
                .collect::<Vec<_>>()
                .join("\n"),
            spec.success_criteria
                .iter()
                .map(|c| format!("- [ ] {}", c))
                .collect::<Vec<_>>()
                .join("\n"),
            spec.technical_requirements.architecture
                .iter()
                .map(|a| format!("- [ ] {}", a))
                .collect::<Vec<_>>()
                .join("\n"),
            spec.technical_requirements.performance
                .iter()
                .map(|p| format!("- [ ] {}", p))
                .collect::<Vec<_>>()
                .join("\n"),
            spec.technical_requirements.security
                .iter()
                .map(|s| format!("- [ ] {}", s))
                .collect::<Vec<_>>()
                .join("\n"),
            spec.dependencies.external
                .iter()
                .map(|d| format!("- [ ] {}", d))
                .collect::<Vec<_>>()
                .join("\n"),
            spec.dependencies.internal
                .iter()
                .map(|d| format!("- [ ] {}", d))
                .collect::<Vec<_>>()
                .join("\n"),
            spec.implementation_plan
                .iter()
                .map(|s| format!(
                    "### Stage {}: {}\n**Goal**: {}\n**Success Criteria**: {}",
                    s.stage,
                    s.name,
                    s.goal,
                    s.success_criteria
                        .iter()
                        .map(|c| format!("- {}", c))
                        .collect::<Vec<_>>()
                        .join(" ")
                ))
                .collect::<Vec<_>>()
                .join("\n\n"),
            spec.testing_strategy.unit_tests
                .iter()
                .map(|t| format!("- [ ] {}", t))
                .collect::<Vec<_>>()
                .join("\n"),
            spec.testing_strategy.integration_tests
                .iter()
                .map(|t| format!("- [ ] {}", t))
                .collect::<Vec<_>>()
                .join("\n"),
            spec.documentation_requirements
                .iter()
                .map(|d| format!("- [ ] {}", d))
                .collect::<Vec<_>>()
                .join("\n"),
            spec.acceptance_criteria
                .iter()
                .map(|c| format!("- [ ] {}", c))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    fn generate_plan_markdown(&self, spec: &Spec) -> String {
        format!(
            "# Implementation Plan: {}\n\n## Overview\n{}\n\n## Stages\n\n{}\
            \n\n## Total Estimated Duration\n{} hours\n\n## Risks and Mitigations\n{}",
            spec.title,
            spec.executive_summary,
            spec.implementation_plan
                .iter()
                .map(|s| format!(
                    "### Stage {} - {}\n**Duration**: {} hours\n\n**Goal**: {}\n\n**Success Criteria**: {}\
                    \n\n**Tasks**:\n{}",
                    s.stage,
                    s.name,
                    s.estimated_duration_hours.unwrap_or(0),
                    s.goal,
                    s.success_criteria
                        .iter()
                        .map(|c| format!("- {}", c))
                        .collect::<Vec<_>>()
                        .join("\n"),
                    s.tasks
                        .iter()
                        .map(|t| format!(
                            "- **{}**{} ({} hours){}",
                            t.id,
                            t.description,
                            t.estimated_hours.unwrap_or(0),
                            if t.dependencies.is_empty() {
                                String::new()
                            } else {
                                format!(" (depends on: {})", t.dependencies.join(", "))
                            }
                        ))
                        .collect::<Vec<_>>()
                        .join("\n")
                ))
                .collect::<Vec<_>>()
                .join("\n\n"),
            spec.implementation_plan
                .iter()
                .map(|s| s.estimated_duration_hours.unwrap_or(0))
                .sum::<u32>(),
            "### Technical Risks\n- Integration complexity with existing system\n- Performance bottlenecks under load\n\n### Mitigation Strategies\n- Early integration testing\n- Performance benchmarking and optimization\n- Incremental implementation with frequent testing"
        )
    }

    fn generate_tasks_markdown(&self, spec: &Spec) -> String {
        format!(
            "# Task Breakdown: {}\n\n## All Tasks\n\n{}\
            \n\n## Task Status\n\n{}\
            \n\n## Dependencies\n\n{}\
            \n\n## Timeline\n{}",
            spec.title,
            spec.implementation_plan
                .iter()
                .flat_map(|s| s.tasks.iter())
                .map(|t| format!(
                    "- **[{}] {}** - {} hours (Status: {})",
                    t.id,
                    t.description,
                    t.estimated_hours.unwrap_or(0),
                    match t.status {
                        TaskStatus::Pending => "Pending",
                        TaskStatus::InProgress => "In Progress",
                        TaskStatus::Completed => "Completed",
                        TaskStatus::Blocked => "Blocked",
                    }
                ))
                .collect::<Vec<_>>()
                .join("\n"),
            spec.implementation_plan
                .iter()
                .flat_map(|s| s.tasks.iter())
                .map(|t| format!(
                    "- **[{}]**: {}",
                    t.id,
                    match t.status {
                        TaskStatus::Pending => "⏳ Pending",
                        TaskStatus::InProgress => "🔄 In Progress",
                        TaskStatus::Completed => "✅ Completed",
                        TaskStatus::Blocked => "🚫 Blocked",
                    }
                ))
                .collect::<Vec<_>>()
                .join("\n"),
            spec.implementation_plan
                .iter()
                .flat_map(|s| s.tasks.iter())
                .filter(|t| !t.dependencies.is_empty())
                .map(|t| format!(
                    "- **[{}] depends on**: {}",
                    t.id,
                    t.dependencies.join(", ")
                ))
                .collect::<Vec<_>>()
                .join("\n"),
            spec.implementation_plan
                .iter()
                .map(|s| format!(
                    "### Stage {}: {}\n- Duration: {} hours\n- Tasks: {}",
                    s.stage,
                    s.name,
                    s.estimated_duration_hours.unwrap_or(0),
                    s.tasks.len()
                ))
                .collect::<Vec<_>>()
                .join("\n\n")
        )
    }
}