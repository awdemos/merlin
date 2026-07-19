use std::process::Command;
use std::env;

/// Builds a CLI command running in an isolated working directory, so
/// concurrently-running tests don't race over the same `features.json`.
fn merlin_command(work_dir: &std::path::Path) -> Command {
    let cargo_bin_path = env::var("CARGO_BIN_EXE_merlin")
        .unwrap_or_else(|_| concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/merlin").to_string());
    let mut cmd = Command::new(cargo_bin_path);
    cmd.current_dir(work_dir);
    cmd
}

/// Creates a fresh, empty temp working directory unique to this test process.
fn fresh_work_dir(test_name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("merlin-cli-test-{}-{}", test_name, std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[tokio::test]
async fn test_cli_feature_workflow() {
    // Isolated working directory: the CLI stores features.json in the CWD
    let work_dir = fresh_work_dir("workflow");

    // Test 1: Create a feature via CLI
    let output = merlin_command(&work_dir)
        .args(&[
            "feature", "create",
            "CLI Integration Test Feature",
            "A feature created during CLI integration testing",
        ])
        .output()
        .expect("Failed to execute feature create command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Created feature:"));
    assert!(output_str.contains("CLI Integration Test Feature"));
    assert!(output_str.contains("Number:"));
    assert!(output_str.contains("Status: Draft"));

    // Test 2: List features and verify our feature exists
    let output = merlin_command(&work_dir)
        .args(&["feature", "list"])
        .output()
        .expect("Failed to execute feature list command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("CLI Integration Test Feature"));

    // Test 3: Get feature details
    let output = merlin_command(&work_dir)
        .args(&["feature", "get", "001-CLI Integration Test Feature"])
        .output()
        .expect("Failed to execute feature get command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Feature ID: 001-CLI Integration Test Feature"));
    assert!(output_str.contains("Name: CLI Integration Test Feature"));
    assert!(output_str.contains("Description: A feature created during CLI integration testing"));
    assert!(output_str.contains("Status: Draft"));

    // Test 4: Update feature status
    let output = merlin_command(&work_dir)
        .args(&["feature", "update", "001-CLI Integration Test Feature", "--status", "Planned"])
        .output()
        .expect("Failed to execute feature update command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Updated feature:"));

    // Test 5: Verify status was updated
    let output = merlin_command(&work_dir)
        .args(&["feature", "get", "001-CLI Integration Test Feature"])
        .output()
        .expect("Failed to execute feature get command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Status: Planned"));

    // Test 6: Get next available number
    let output = merlin_command(&work_dir)
        .args(&["feature", "next-number"])
        .output()
        .expect("Failed to execute next-number command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Next available feature number: 2"));

    // Test 7: Reserve a number
    let output = merlin_command(&work_dir)
        .args(&["feature", "reserve", "100", "Reserved for special milestone"])
        .output()
        .expect("Failed to execute reserve command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Reserved number: 100"));

    // Test 8: Verify next number skips reserved numbers
    let output = merlin_command(&work_dir)
        .args(&["feature", "next-number"])
        .output()
        .expect("Failed to execute next-number command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Next available feature number: 2"));

    // Test 9: Create another feature to verify number assignment
    let output = merlin_command(&work_dir)
        .args(&["feature", "create", "Second CLI Test Feature", "Another test feature"])
        .output()
        .expect("Failed to execute second feature create command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Number: 2"));

    // Test 10: List all features and verify both exist
    let output = merlin_command(&work_dir)
        .args(&["feature", "list"])
        .output()
        .expect("Failed to execute feature list command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("CLI Integration Test Feature"));
    assert!(output_str.contains("Second CLI Test Feature"));
    assert!(output_str.contains("Found 2 features"));

    // Test 11: Search features
    let output = merlin_command(&work_dir)
        .args(&["feature", "list", "--search", "integration"])
        .output()
        .expect("Failed to execute feature search command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("CLI Integration Test Feature"));
    assert!(!output_str.contains("Second CLI Test Feature"));

    // Test 12: Filter by status
    let output = merlin_command(&work_dir)
        .args(&["feature", "list", "--status", "Draft"])
        .output()
        .expect("Failed to execute feature status filter command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(!output_str.contains("CLI Integration Test Feature")); // Should be Planned
    assert!(output_str.contains("Second CLI Test Feature")); // Should be Draft

    // Test 13: Delete a draft feature
    let output = merlin_command(&work_dir)
        .args(&["feature", "delete", "002-Second CLI Test Feature"])
        .output()
        .expect("Failed to execute feature delete command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Deleted feature"));

    // Test 14: Verify deletion
    let output = merlin_command(&work_dir)
        .args(&["feature", "list"])
        .output()
        .expect("Failed to execute feature list command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("CLI Integration Test Feature"));
    assert!(!output_str.contains("Second CLI Test Feature"));
    assert!(output_str.contains("Found 1 features"));

    // Test 15: Update feature with metadata
    let output = merlin_command(&work_dir)
        .args(&[
            "feature", "update", "001-CLI Integration Test Feature",
            "--metadata", r#"{"priority": "High", "tags": ["test", "integration"], "dependencies": [], "related_features": []}"#
        ])
        .output()
        .expect("Failed to execute feature update command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    // CLI might have different output format, just check it succeeded

    // Test 16: Verify metadata was updated
    let output = merlin_command(&work_dir)
        .args(&["feature", "get", "001-CLI Integration Test Feature"])
        .output()
        .expect("Failed to execute feature get command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("High"));
    assert!(output_str.contains("test"));
    assert!(output_str.contains("integration"));

    // Clean up
    let _ = std::fs::remove_dir_all(&work_dir);
}

#[tokio::test]
async fn test_cli_error_handling() {
    // Isolated working directory: the CLI stores features.json in the CWD
    let work_dir = fresh_work_dir("error-handling");

    // Test 1: Get non-existent feature
    let output = merlin_command(&work_dir)
        .args(&["feature", "get", "non-existent-feature"])
        .output()
        .expect("Failed to execute feature get command");

    // CLI outputs error to stderr but exits successfully
    let output_str = String::from_utf8_lossy(&output.stderr);
    assert!(output_str.contains("Feature not found"));

    // Test 2: Update non-existent feature
    let output = merlin_command(&work_dir)
        .args(&["feature", "update", "non-existent-feature", "--status", "Planned"])
        .output()
        .expect("Failed to execute feature update command");

    // CLI outputs error to stderr but exits successfully
    let output_str = String::from_utf8_lossy(&output.stderr);
    assert!(output_str.contains("Feature not found"));

    // Test 3: Delete non-existent feature
    let output = merlin_command(&work_dir)
        .args(&["feature", "delete", "non-existent-feature"])
        .output()
        .expect("Failed to execute feature delete command");

    // CLI outputs error to stderr but exits successfully
    let output_str = String::from_utf8_lossy(&output.stderr);
    assert!(output_str.contains("Failed to delete feature"));

    // Test 4: Reserve already assigned number
    // First create a feature
    let output = merlin_command(&work_dir)
        .args(&["feature", "create", "Test Feature", "Test description"])
        .output()
        .expect("Failed to execute feature create command");

    assert!(output.status.success());

    // Then try to reserve the same number
    let output = merlin_command(&work_dir)
        .args(&["feature", "reserve", "1", "Should fail"])
        .output()
        .expect("Failed to execute feature reserve command");

    // CLI outputs error to stderr but exits successfully
    let output_str = String::from_utf8_lossy(&output.stderr);
    assert!(output_str.contains("already assigned"));

    // Clean up
    let _ = std::fs::remove_dir_all(&work_dir);
}