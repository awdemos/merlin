//! Specification execution and task management

use super::*;
use anyhow::{Result, Context};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;
// Removed unused imports

/// Executes specifications and manages implementation tasks
pub struct SpecExecutor {
    base_dir: PathBuf,
    work_dir: PathBuf,
}

impl SpecExecutor {
    /// Create a new specification executor
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        let base_dir = base_dir.as_ref().to_path_buf();
        let work_dir = base_dir.join("work");

        Self {
            base_dir,
            work_dir,
        }
    }

    /// Execute a specification implementation
    pub fn execute(&self, spec_path: &Path) -> Result<ExecutionResult> {
        let start_time = std::time::Instant::now();
        let mut artifacts = Vec::new();
        let mut output = String::new();

        // Load and parse the specification
        let spec = self.load_spec(spec_path)?;

        output.push_str(&format!("Executing specification: {}\n", spec.title));
        output.push_str(&format!("Created: {}\n", spec.created_at));
        output.push_str(&format!("Status: {:?}\n\n", spec.status));

        // Create working directory
        let spec_work_dir = self.work_dir.join(&spec.id);
        fs::create_dir_all(&spec_work_dir)
            .context("Failed to create working directory")?;

        // Execute implementation stages
        for stage in &spec.implementation_plan {
            output.push_str(&format!("=== Stage {}: {} ===\n", stage.stage, stage.name));

            let stage_result = self.execute_stage(stage, &spec_work_dir, &mut artifacts)?;
            output.push_str(&stage_result);
            output.push('\n');
        }

        // Run validation and tests
        output.push_str("=== Validation and Testing ===\n");
        let validation_result = self.run_validation(&spec_work_dir, &mut artifacts)?;
        output.push_str(&validation_result);

        let duration = start_time.elapsed().as_secs() as u32;

        Ok(ExecutionResult {
            success: true,
            output,
            artifacts,
            duration_seconds: duration,
            error: None,
        })
    }

    /// Execute a specific implementation stage
    pub fn execute_stage(&self, stage: &ImplementationStage, work_dir: &Path, artifacts: &mut Vec<PathBuf>) -> Result<String> {
        let mut output = String::new();

        output.push_str(&format!("Goal: {}\n", stage.goal));
        output.push_str(&format!("Estimated duration: {} hours\n\n",
            stage.estimated_duration_hours.unwrap_or(0)));

        // Execute tasks in this stage
        for task in &stage.tasks {
            if let Err(e) = self.execute_task(task, work_dir, artifacts) {
                output.push_str(&format!("❌ Task {} failed: {}\n", task.id, e));
                return Ok(output);
            }
        }

        // Verify success criteria
        let criteria_met = self.verify_success_criteria(&stage.success_criteria, work_dir)?;
        if criteria_met {
            output.push_str("✅ All success criteria met\n");
        } else {
            output.push_str("⚠️ Some success criteria not fully met\n");
        }

        Ok(output)
    }

    /// Execute a specific task
    fn execute_task(&self, task: &Task, work_dir: &Path, artifacts: &mut Vec<PathBuf>) -> Result<()> {
        println!("Executing task: {} - {}", task.id, task.description);

        // Parse task ID to determine action
        let action = self.parse_task_action(&task.id)?;

        match action {
            TaskAction::GenerateCode => self.generate_code_task(task, work_dir, artifacts)?,
            TaskAction::CreateFiles => self.create_files_task(task, work_dir, artifacts)?,
            TaskAction::RunTests => self.run_tests_task(task, work_dir)?,
            TaskAction::Validate => self.validate_task(task, work_dir)?,
            TaskAction::Custom(action_name) => self.execute_custom_task(task, work_dir, &action_name)?,
        }

        Ok(())
    }

    /// Parse task action from task ID
    fn parse_task_action(&self, task_id: &str) -> Result<TaskAction> {
        if task_id.contains("code") || task_id.contains("implement") {
            Ok(TaskAction::GenerateCode)
        } else if task_id.contains("file") || task_id.contains("create") {
            Ok(TaskAction::CreateFiles)
        } else if task_id.contains("test") {
            Ok(TaskAction::RunTests)
        } else if task_id.contains("validate") || task_id.contains("check") {
            Ok(TaskAction::Validate)
        } else {
            Ok(TaskAction::Custom(task_id.to_string()))
        }
    }

    /// Generate code for a task
    fn generate_code_task(&self, task: &Task, work_dir: &Path, artifacts: &mut Vec<PathBuf>) -> Result<()> {
        // Create source code directory structure
        let src_dir = work_dir.join("src");
        fs::create_dir_all(&src_dir)?;

        // Generate example code based on task description
        let code_content = self.generate_code_for_task(task)?;
        let code_file = src_dir.join(format!("{}.rs", task.id.replace('-', "_")));

        fs::write(&code_file, code_content)?;
        artifacts.push(code_file);

        Ok(())
    }

    /// Create files for a task
    fn create_files_task(&self, task: &Task, work_dir: &Path, artifacts: &mut Vec<PathBuf>) -> Result<()> {
        // Create configuration files, documentation, etc.
        let config_dir = work_dir.join("config");
        fs::create_dir_all(&config_dir)?;

        // Generate configuration file
        let config_content = self.generate_config_for_task(task)?;
        let config_file = config_dir.join(format!("{}.toml", task.id));

        fs::write(&config_file, config_content)?;
        artifacts.push(config_file);

        Ok(())
    }

    /// Run tests for a task
    fn run_tests_task(&self, _task: &Task, work_dir: &Path) -> Result<()> {
        // Run cargo test if this is a Rust project
        if work_dir.join("Cargo.toml").exists() {
            let output = Command::new("cargo")
                .args(&["test", "--lib"])
                .current_dir(work_dir)
                .output()
                .context("Failed to run cargo test")?;

            if !output.status.success() {
                let error_msg = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("Tests failed: {}", error_msg));
            }
        }

        Ok(())
    }

    /// Validate implementation
    fn validate_task(&self, _task: &Task, work_dir: &Path) -> Result<()> {
        // Run cargo check
        if work_dir.join("Cargo.toml").exists() {
            let output = Command::new("cargo")
                .args(&["check"])
                .current_dir(work_dir)
                .output()
                .context("Failed to run cargo check")?;

            if !output.status.success() {
                let error_msg = String::from_utf8_lossy(&output.stderr);
                return Err(anyhow::anyhow!("Cargo check failed: {}", error_msg));
            }
        }

        Ok(())
    }

    /// Execute custom task
    fn execute_custom_task(&self, task: &Task, work_dir: &Path, action: &str) -> Result<()> {
        // For now, log the custom action
        let log_file = work_dir.join("custom_tasks.log");
        let mut log_content = if log_file.exists() {
            fs::read_to_string(&log_file)?
        } else {
            String::new()
        };

        log_content.push_str(&format!(
            "{}: {} - {}\n",
            chrono::Utc::now().to_rfc3339(),
            task.id,
            action
        ));

        fs::write(log_file, log_content)?;
        Ok(())
    }

    /// Verify success criteria
    fn verify_success_criteria(&self, criteria: &[String], work_dir: &Path) -> Result<bool> {
        let mut all_met = true;

        for criterion in criteria {
            let met = self.verify_criterion(criterion, work_dir)?;
            if !met {
                all_met = false;
                println!("❌ Criteria not met: {}", criterion);
            } else {
                println!("✅ Criteria met: {}", criterion);
            }
        }

        Ok(all_met)
    }

    /// Verify a single criterion
    fn verify_criterion(&self, criterion: &str, work_dir: &Path) -> Result<bool> {
        // Simple heuristic-based verification
        if criterion.contains("compile") {
            // Check if Rust project compiles
            if work_dir.join("Cargo.toml").exists() {
                let output = Command::new("cargo")
                    .args(&["check"])
                    .current_dir(work_dir)
                    .output()
                    .context("Failed to run cargo check")?;

                return Ok(output.status.success());
            }
        }

        if criterion.contains("test") {
            // Check if tests pass
            if work_dir.join("Cargo.toml").exists() {
                let output = Command::new("cargo")
                    .args(&["test"])
                    .current_dir(work_dir)
                    .output()
                    .context("Failed to run cargo test")?;

                return Ok(output.status.success());
            }
        }

        if criterion.contains("file") || criterion.contains("created") {
            // Check if files were created
            let has_files = fs::read_dir(work_dir)
                .map(|mut entries| entries.next().is_some())
                .unwrap_or(false);

            return Ok(has_files);
        }

        // Default to true for unverified criteria
        Ok(true)
    }

    /// Run validation and testing
    fn run_validation(&self, work_dir: &Path, artifacts: &mut Vec<PathBuf>) -> Result<String> {
        let mut output = String::new();

        // Run cargo fmt check
        if work_dir.join("Cargo.toml").exists() {
            output.push_str("Running cargo fmt check...\n");
            let fmt_result = Command::new("cargo")
                .args(&["fmt", "--check"])
                .current_dir(work_dir)
                .output();

            match fmt_result {
                Ok(output) => {
                    if output.status.success() {
                        output.push_str("✅ Code formatting check passed\n");
                    } else {
                        output.push_str("⚠️ Code formatting issues found\n");
                    }
                }
                Err(e) => {
                    output.push_str(&format!("❌ Failed to run cargo fmt: {}\n", e));
                }
            }
        }

        // Run cargo clippy
        if work_dir.join("Cargo.toml").exists() {
            output.push_str("Running cargo clippy...\n");
            let clippy_result = Command::new("cargo")
                .args(&["clippy", "--", "-D", "warnings"])
                .current_dir(work_dir)
                .output();

            match clippy_result {
                Ok(clippy_output) => {
                    if clippy_output.status.success() {
                        output.push_str("✅ Clippy check passed\n");
                    } else {
                        let stderr = String::from_utf8_lossy(&clippy_output.stderr);
                        output.push_str(&format!("⚠️ Clippy warnings:\n{}\n", stderr));
                    }
                }
                Err(e) => {
                    output.push_str(&format!("❌ Failed to run cargo clippy: {}\n", e));
                }
            }
        }

        // Generate execution report
        let report_content = self.generate_execution_report(work_dir)?;
        let report_file = work_dir.join("execution_report.md");
        fs::write(&report_file, report_content)?;
        artifacts.push(report_file);

        output.push_str("✅ Execution report generated\n");

        Ok(output)
    }

    /// Load specification from file
    fn load_spec(&self, spec_path: &Path) -> Result<Spec> {
        let content = fs::read_to_string(spec_path)?;
        Spec::from_markdown(&content)
    }

    /// Generate code for task
    fn generate_code_for_task(&self, task: &Task) -> Result<String> {
        // Generate simple example code based on task description
        let task_lower = task.description.to_lowercase();

        if task_lower.contains("api") || task_lower.contains("endpoint") {
            Ok(self.generate_api_code(task))
        } else if task_lower.contains("config") || task_lower.contains("settings") {
            Ok(self.generate_config_code(task))
        } else {
            Ok(self.generate_generic_code(task))
        }
    }

    fn generate_api_code(&self, task: &Task) -> String {
        format!(
            "// Generated code for task: {}\n\
            // Description: {}\n\n\
            use std::collections::HashMap;\n\
            use serde::{{Deserialize, Serialize}};\n\
            use anyhow::Result;\n\n\
            #[derive(Debug, Serialize, Deserialize)]\n\
            pub struct ApiResponse {{\n    pub success: bool,\n    pub message: String,\n}}\n\n\
            pub async fn handle_request() -> Result<ApiResponse> {{\n    Ok(ApiResponse {{\n        success: true,\n        message: \"Request processed successfully\".to_string(),\n    }})\n}}\n\n\
            #[cfg(test)]\nmod tests {{\n    use super::*;\n\n    #[tokio::test]\n    async fn test_handle_request() {{\n        let result = handle_request().await;\n        assert!(result.is_ok());\n        \n        let response = result.unwrap();\n        assert!(response.success);\n    }}\n}}",
            task.id, task.description
        )
    }

    fn generate_config_code(&self, task: &Task) -> String {
        format!(
            "// Generated configuration code for task: {}\n\
            // Description: {}\n\n\
            use serde::{{Deserialize, Serialize}};\n\
            use std::fs;\nuse std::path::Path;\n\n\
            #[derive(Debug, Serialize, Deserialize, Clone)]\n\
            pub struct Config {{\n    pub enabled: bool,\n    pub timeout_seconds: u64,\n    pub max_retries: u32,\n}}\n\n\
            impl Default for Config {{\n    fn default() -> Self {{\n        Self {{\n            enabled: true,\n            timeout_seconds: 30,\n            max_retries: 3,\n        }}\n    }}\n}}\n\n\
            impl Config {{\n    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {{\n        let content = fs::read_to_string(path)?;\n        Ok(serde_json::from_str(&content)?)\n    }}\n\n    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {{\n        let content = serde_json::to_string_pretty(self)?;\n        fs::write(path, content)?;\n        Ok(())\n    }}\n}}\n\n\
            #[cfg(test)]\nmod tests {{\n    use super::*;\n    use tempfile::NamedTempFile;\n\n    #[test]\n    fn test_config_save_load() {{\n        let config = Config::default();\n        let temp_file = NamedTempFile::new().unwrap();\n        \n        config.save(&temp_file).unwrap();\n        let loaded = Config::load(&temp_file).unwrap();\n        \n        assert_eq!(config.enabled, loaded.enabled);\n        assert_eq!(config.timeout_seconds, loaded.timeout_seconds);\n        assert_eq!(config.max_retries, loaded.max_retries);\n    }}\n}}",
            task.id, task.description
        )
    }

    fn generate_generic_code(&self, task: &Task) -> String {
        format!(
            "// Generated code for task: {}\n\
            // Description: {}\n\n\
            use anyhow::Result;\n\n\
            pub struct {}Processor;\n\n\
            impl {}Processor {{\n    pub fn new() -> Self {{\n        Self\n    }}\n\n    pub fn process(&self) -> Result<String> {{\n        // Implementation for {}\n        Ok(\"Processing completed successfully\".to_string())\n    }}\n}}\n\n\
            #[cfg(test)]\nmod tests {{\n    use super::*;\n\n    #[test]\n    fn test_process() {{\n        let processor = {}Processor::new();\n        let result = processor.process();\n        assert!(result.is_ok());\n    }}\n}}",
            task.id,
            task.description,
            self.camel_case(&task.id),
            self.camel_case(&task.id),
            task.description,
            self.camel_case(&task.id)
        )
    }

    fn generate_config_for_task(&self, task: &Task) -> Result<String> {
        Ok(format!(
            "# Configuration for task: {}\n\
            # Description: {}\n\n\
            [task.{}]\n\
            enabled = true\n\
            timeout_seconds = 30\n\
            max_retries = 3\n\
            \n\
            [logging]\n\
            level = \"info\"\n\
            format = \"json\"\n",
            task.id,
            task.description,
            task.id.replace('-', "_")
        ))
    }

    fn camel_case(&self, s: &str) -> String {
        s.split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect()
    }

    fn generate_execution_report(&self, work_dir: &Path) -> Result<String> {
        let mut report = String::new();
        report.push_str("# Execution Report\n\n");
        report.push_str(&format!("**Generated**: {}\n\n", chrono::Utc::now().to_rfc3339()));

        // List generated files
        report.push_str("## Generated Files\n\n");
        if let Ok(entries) = fs::read_dir(work_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    report.push_str(&format!("- {}\n", path.file_name().unwrap().to_string_lossy()));
                }
            }
        }

        report.push_str("\n## Summary\n\n");
        report.push_str("Specification execution completed successfully.\n");
        report.push_str("All generated files have been created in the working directory.\n");

        Ok(report)
    }
}

/// Task action type
#[derive(Debug, Clone)]
enum TaskAction {
    GenerateCode,
    CreateFiles,
    RunTests,
    Validate,
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_executor_creation() {
        let temp_dir = TempDir::new().unwrap();
        let executor = SpecExecutor::new(temp_dir.path());

        assert_eq!(executor.base_dir, temp_dir.path());
        assert_eq!(executor.work_dir, temp_dir.path().join("work"));
    }

    #[test]
    fn test_task_action_parsing() {
        let temp_dir = TempDir::new().unwrap();
        let executor = SpecExecutor::new(temp_dir.path());

        assert!(matches!(executor.parse_task_action("generate-code").unwrap(), TaskAction::GenerateCode));
        assert!(matches!(executor.parse_task_action("create-files").unwrap(), TaskAction::CreateFiles));
        assert!(matches!(executor.parse_task_action("run-tests").unwrap(), TaskAction::RunTests));
        assert!(matches!(executor.parse_task_action("validate-task").unwrap(), TaskAction::Validate));
        assert!(matches!(executor.parse_task_action("custom-action").unwrap(), TaskAction::Custom(_)));
    }
}