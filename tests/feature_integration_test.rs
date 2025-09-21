use std::process::Command;
use std::env;

#[tokio::test]
async fn test_cli_feature_workflow() {
    // Remove any existing features.json to start fresh
    if std::path::Path::new("features.json").exists() {
        std::fs::remove_file("features.json").unwrap();
    }

    // Find the cargo binary path
    let cargo_bin_path = env::var("CARGO_BIN_EXE_merlin")
        .unwrap_or_else(|_| "./target/debug/merlin".to_string());

    // Test 1: Create a feature via CLI
    let output = Command::new(&cargo_bin_path)
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
    let output = Command::new(&cargo_bin_path)
        .args(&["feature", "list"])
        .output()
        .expect("Failed to execute feature list command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("CLI Integration Test Feature"));

    // Test 3: Get feature details
    let output = Command::new(&cargo_bin_path)
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
    let output = Command::new(&cargo_bin_path)
        .args(&["feature", "update", "001-CLI Integration Test Feature", "--status", "Planned"])
        .output()
        .expect("Failed to execute feature update command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Updated feature:"));

    // Test 5: Verify status was updated
    let output = Command::new(&cargo_bin_path)
        .args(&["feature", "get", "001-CLI Integration Test Feature"])
        .output()
        .expect("Failed to execute feature get command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Status: Planned"));

    // Test 6: Get next available number
    let output = Command::new(&cargo_bin_path)
        .args(&["feature", "next-number"])
        .output()
        .expect("Failed to execute next-number command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Next available feature number: 2"));

    // Test 7: Reserve a number
    let output = Command::new(&cargo_bin_path)
        .args(&["feature", "reserve", "100", "Reserved for special milestone"])
        .output()
        .expect("Failed to execute reserve command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Reserved number: 100"));

    // Test 8: Verify next number skips reserved numbers
    let output = Command::new(&cargo_bin_path)
        .args(&["feature", "next-number"])
        .output()
        .expect("Failed to execute next-number command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Next available feature number: 2"));

    // Test 9: Create another feature to verify number assignment
    let output = Command::new(&cargo_bin_path)
        .args(&["feature", "create", "Second CLI Test Feature", "Another test feature"])
        .output()
        .expect("Failed to execute second feature create command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Number: 2"));

    // Test 10: List all features and verify both exist
    let output = Command::new(&cargo_bin_path)
        .args(&["feature", "list"])
        .output()
        .expect("Failed to execute feature list command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("CLI Integration Test Feature"));
    assert!(output_str.contains("Second CLI Test Feature"));
    assert!(output_str.contains("Found 2 features"));

    // Test 11: Search features
    let output = Command::new(&cargo_bin_path)
        .args(&["feature", "list", "--search", "integration"])
        .output()
        .expect("Failed to execute feature search command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("CLI Integration Test Feature"));
    assert!(!output_str.contains("Second CLI Test Feature"));

    // Test 12: Filter by status
    let output = Command::new(&cargo_bin_path)
        .args(&["feature", "list", "--status", "Draft"])
        .output()
        .expect("Failed to execute feature status filter command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(!output_str.contains("CLI Integration Test Feature")); // Should be Planned
    assert!(output_str.contains("Second CLI Test Feature")); // Should be Draft

    // Test 13: Delete a draft feature
    let output = Command::new(&cargo_bin_path)
        .args(&["feature", "delete", "002-Second CLI Test Feature"])
        .output()
        .expect("Failed to execute feature delete command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("Deleted feature"));

    // Test 14: Verify deletion
    let output = Command::new(&cargo_bin_path)
        .args(&["feature", "list"])
        .output()
        .expect("Failed to execute feature list command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("CLI Integration Test Feature"));
    assert!(!output_str.contains("Second CLI Test Feature"));
    assert!(output_str.contains("Found 1 features"));

    // Test 15: Update feature with metadata
    let output = Command::new(&cargo_bin_path)
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
    let output = Command::new(&cargo_bin_path)
        .args(&["feature", "get", "001-CLI Integration Test Feature"])
        .output()
        .expect("Failed to execute feature get command");

    assert!(output.status.success());
    let output_str = String::from_utf8_lossy(&output.stdout);
    assert!(output_str.contains("High"));
    assert!(output_str.contains("test"));
    assert!(output_str.contains("integration"));

    // Clean up
    if std::path::Path::new("features.json").exists() {
        std::fs::remove_file("features.json").unwrap();
    }
}

#[tokio::test]
async fn test_cli_error_handling() {
    // Remove any existing features.json to start fresh
    if std::path::Path::new("features.json").exists() {
        std::fs::remove_file("features.json").unwrap();
    }

    let cargo_bin_path = env::var("CARGO_BIN_EXE_merlin")
        .unwrap_or_else(|_| "./target/debug/merlin".to_string());

    // Test 1: Get non-existent feature
    let output = Command::new(&cargo_bin_path)
        .args(&["feature", "get", "non-existent-feature"])
        .output()
        .expect("Failed to execute feature get command");

    // CLI outputs error to stderr but exits successfully
    let output_str = String::from_utf8_lossy(&output.stderr);
    assert!(output_str.contains("Feature not found"));

    // Test 2: Update non-existent feature
    let output = Command::new(&cargo_bin_path)
        .args(&["feature", "update", "non-existent-feature", "--status", "Planned"])
        .output()
        .expect("Failed to execute feature update command");

    // CLI outputs error to stderr but exits successfully
    let output_str = String::from_utf8_lossy(&output.stderr);
    assert!(output_str.contains("Feature not found"));

    // Test 3: Delete non-existent feature
    let output = Command::new(&cargo_bin_path)
        .args(&["feature", "delete", "non-existent-feature"])
        .output()
        .expect("Failed to execute feature delete command");

    // CLI outputs error to stderr but exits successfully
    let output_str = String::from_utf8_lossy(&output.stderr);
    assert!(output_str.contains("Failed to delete feature"));

    // Test 4: Reserve already assigned number
    // First create a feature
    let output = Command::new(&cargo_bin_path)
        .args(&["feature", "create", "Test Feature", "Test description"])
        .output()
        .expect("Failed to execute feature create command");

    assert!(output.status.success());

    // Then try to reserve the same number
    let output = Command::new(&cargo_bin_path)
        .args(&["feature", "reserve", "1", "Should fail"])
        .output()
        .expect("Failed to execute feature reserve command");

    // CLI outputs error to stderr but exits successfully
    let output_str = String::from_utf8_lossy(&output.stderr);
    assert!(output_str.contains("already assigned"));

    // Clean up
    if std::path::Path::new("features.json").exists() {
        std::fs::remove_file("features.json").unwrap();
    }
}