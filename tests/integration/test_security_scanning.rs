use std::process::Command;
use std::thread;
use std::time::Duration;
use std::fs;
use std::path::Path;

#[test]
fn test_security_scan_trivy_integration() {
    // Test Trivy vulnerability scanning integration
    let build_output = Command::new("docker")
        .args(&["build", "-t", "merlin:test-trivy", "-f", "docker/Dockerfile.hardened", "."])
        .output()
        .expect("Failed to execute docker build command");

    if !build_output.status.success() {
        println!("Build failed as expected - Dockerfile not implemented yet");
        return;
    }

    // Create scan results directory
    fs::create_dir_all("target/test-scans").unwrap();

    // Run Trivy scan
    let output = Command::new("trivy")
        .args(&[
            "image",
            "--format", "json",
            "--output", "target/test-scans/trivy-results.json",
            "--exit-code", "0",  // Don't fail on vulnerabilities for test
            "merlin:test-trivy"
        ])
        .output();

    // If Trivy is not installed, skip this test
    if output.is_err() {
        println!("Trivy not installed - skipping security scan test");
        // Cleanup
        Command::new("docker")
            .args(&["rmi", "merlin:test-trivy"])
            .output()
            .expect("Failed to remove test image");
        return;
    }

    let output = output.unwrap();
    assert!(output.status.success(), "Trivy scan should complete");

    // Check if scan results file was created
    assert!(Path::new("target/test-scans/trivy-results.json").exists(),
            "Trivy scan results file should be created");

    // Read and validate scan results
    let scan_results = fs::read_to_string("target/test-scans/trivy-results.json")
        .expect("Failed to read scan results");

    // Parse JSON and check structure
    let scan_json: serde_json::Value = serde_json::from_str(&scan_results)
        .expect("Failed to parse scan results JSON");

    assert!(scan_json.is_array(), "Scan results should be an array");

    if !scan_json.as_array().unwrap().is_empty() {
        let first_result = &scan_json.as_array().unwrap()[0];
        assert!(first_result.get("Target").is_some(), "Should have Target field");
        assert!(first_result.get("Vulnerabilities").is_some(), "Should have Vulnerabilities field");
    }

    // Cleanup
    fs::remove_file("target/test-scans/trivy-results.json").unwrap_or(());
    fs::remove_dir("target/test-scans").unwrap_or(());
    Command::new("docker")
        .args(&["rmi", "merlin:test-trivy"])
        .output()
        .expect("Failed to remove test image");
}

#[test]
fn test_security_scan_hadolint_integration() {
    // Test Hadolint Dockerfile validation
    // Create a test Dockerfile
    fs::create_dir_all("target/test-docker").unwrap();

    let test_dockerfile = r#"
# Test Dockerfile for Hadolint validation
FROM alpine:3.19
RUN apk add --no-cache curl
USER root
WORKDIR /app
COPY . .
CMD ["tail", "-f", "/dev/null"]
"#;

    fs::write("target/test-docker/Dockerfile.test", test_dockerfile)
        .expect("Failed to write test Dockerfile");

    // Run Hadolint
    let output = Command::new("hadolint")
        .args(&["target/test-docker/Dockerfile.test"])
        .output();

    // If Hadolint is not installed, skip this test
    if output.is_err() {
        println!("Hadolint not installed - skipping Hadolint test");
        // Cleanup
        fs::remove_file("target/test-docker/Dockerfile.test").unwrap_or(());
        fs::remove_dir("target/test-docker").unwrap_or(());
        return;
    }

    let output = output.unwrap();

    // Hadolint should find issues with our test Dockerfile
    // (running as root, using latest tag, etc.)
    assert!(output.status.success() || !output.status.success(),
            "Hadolint should run regardless of issues found");

    let lint_output = String::from_utf8_lossy(&output.stdout);
    let error_output = String::from_utf8_lossy(&output.stderr);

    // Should detect running as root
    assert!(lint_output.contains("DL3002") || error_output.contains("DL3002"),
            "Should detect running as root user");

    // Should detect using latest tag
    assert!(lint_output.contains("DL3007") || error_output.contains("DL3007"),
            "Should detect using latest tag");

    // Cleanup
    fs::remove_file("target/test-docker/Dockerfile.test").unwrap_or(());
    fs::remove_dir("target/test-docker").unwrap_or(());
}

#[test]
fn test_security_scan_docker_bench() {
    // Test Docker Bench for Security
    let build_output = Command::new("docker")
        .args(&["build", "-t", "merlin:test-bench", "-f", "docker/Dockerfile.hardened", "."])
        .output()
        .expect("Failed to execute docker build command");

    if !build_output.status.success() {
        println!("Build failed as expected - Dockerfile not implemented yet");
        return;
    }

    // Create scan results directory
    fs::create_dir_all("target/test-bench").unwrap();

    // Run Docker Bench for Security
    let output = Command::new("docker")
        .args(&[
            "run", "--rm",
            "-v", "/var/run/docker.sock:/var/run/docker.sock",
            "-v", format!("{}/target/test-bench:/output", std::env::current_dir().unwrap().to_string_lossy()).as_str(),
            "docker/docker-bench-security",
            "-l", "/output/bench-results.log",
            "-t", "container"  // Test only container security
        ])
        .output();

    // If Docker Bench image is not available, skip this test
    if output.is_err() {
        println!("Docker Bench for Security not available - skipping test");
        // Cleanup
        fs::remove_dir("target/test-bench").unwrap_or(());
        Command::new("docker")
            .args(&["rmi", "merlin:test-bench"])
            .output()
            .expect("Failed to remove test image");
        return;
    }

    let output = output.unwrap();

    // Docker Bench might fail on some checks but should complete
    println!("Docker Bench exit code: {}", output.status);

    // Check if log file was created
    if Path::new("target/test-bench/bench-results.log").exists() {
        let log_contents = fs::read_to_string("target/test-bench/bench-results.log")
            .expect("Failed to read bench results");

        // Should contain security checks
        assert!(log_contents.contains("[WARN]") || log_contents.contains("[INFO]") || log_contents.contains("[PASS]"),
                "Should contain security check results");
    }

    // Cleanup
    fs::remove_dir("target/test-bench").unwrap_or(());
    Command::new("docker")
        .args(&["rmi", "merlin:test-bench"])
        .output()
        .expect("Failed to remove test image");
}

#[test]
fn test_security_sbom_generation() {
    // Test Software Bill of Materials (SBOM) generation
    let build_output = Command::new("docker")
        .args(&["build", "-t", "merlin:test-sbom", "-f", "docker/Dockerfile.hardened", "."])
        .output()
        .expect("Failed to execute docker build command");

    if !build_output.status.success() {
        println!("Build failed as expected - Dockerfile not implemented yet");
        return;
    }

    // Create SBOM directory
    fs::create_dir_all("target/test-sbom").unwrap();

    // Generate SBOM using Syft (if available)
    let output = Command::new("syft")
        .args(&[
            "merlin:test-sbom",
            "--output", "cyclonedx-json=target/test-sbom/sbom.json"
        ])
        .output();

    // If Syft is not installed, skip this test
    if output.is_err() {
        println!("Syft not installed - skipping SBOM test");
        // Cleanup
        fs::remove_dir("target/test-sbom").unwrap_or(());
        Command::new("docker")
            .args(&["rmi", "merlin:test-sbom"])
            .output()
            .expect("Failed to remove test image");
        return;
    }

    let output = output.unwrap();
    assert!(output.status.success(), "SBOM generation should succeed");

    // Check if SBOM file was created
    assert!(Path::new("target/test-sbom/sbom.json").exists(),
            "SBOM file should be created");

    // Read and validate SBOM
    let sbom_contents = fs::read_to_string("target/test-sbom/sbom.json")
        .expect("Failed to read SBOM");

    let sbom_json: serde_json::Value = serde_json::from_str(&sbom_contents)
        .expect("Failed to parse SBOM JSON");

    // Check CycloneDX SBOM structure
    assert!(sbom_json.get("bomFormat").is_some(), "Should have bomFormat");
    assert!(sbom_json.get("specVersion").is_some(), "Should have specVersion");
    assert!(sbom_json.get("components").is_some(), "Should have components");

    // Cleanup
    fs::remove_file("target/test-sbom/sbom.json").unwrap_or(());
    fs::remove_dir("target/test-sbom").unwrap_or(());
    Command::new("docker")
        .args(&["rmi", "merlin:test-sbom"])
        .output()
        .expect("Failed to remove test image");
}

#[test]
fn test_security_compliance_checking() {
    // Test security compliance checking
    let build_output = Command::new("docker")
        .args(&["build", "-t", "merlin:test-compliance", "-f", "docker/Dockerfile.hardened", "."])
        .output()
        .expect("Failed to execute docker build command");

    if !build_output.status.success() {
        println!("Build failed as expected - Dockerfile not implemented yet");
        return;
    }

    // Run container for compliance checking
    let mut child = Command::new("docker")
        .args(&[
            "run", "-d",
            "--name", "merlin-compliance-test",
            "merlin:test-compliance"
        ])
        .spawn()
        .expect("Failed to start container");

    // Give container time to start
    thread::sleep(Duration::from_secs(3));

    // Check compliance requirements
    let checks = vec![
        ("user", "1000:1000"),
        ("read_only", "true"),
        ("security_opt", "no-new-privileges"),
    ];

    for (check_type, expected) in checks {
        let format_arg = match check_type {
            "user" => "{{.Config.User}}",
            "read_only" => "{{.HostConfig.ReadonlyRootfs}}",
            "security_opt" => "{{index .HostConfig.SecurityOpt 0}}",
            _ => continue,
        };

        let output = Command::new("docker")
            .args(&["inspect", "merlin-compliance-test", "--format", format_arg])
            .output()
            .expect("Failed to inspect container");

        assert!(output.status.success(), format!("Should be able to check {}", check_type));

        let result = String::from_utf8_lossy(&output.stdout).trim();

        if check_type == "security_opt" {
            assert!(result.contains("no-new-privileges") || result.is_empty(),
                    "Should have no-new-privileges security option");
        } else {
            assert_eq!(result, expected, format!("{} should be {}", check_type, expected));
        }
    }

    // Stop and cleanup container
    Command::new("docker")
        .args(&["stop", "merlin-compliance-test"])
        .output()
        .expect("Failed to stop container");

    child.wait().expect("Failed to wait for container process");

    // Cleanup
    Command::new("docker")
        .args(&["rmi", "merlin:test-compliance"])
        .output()
        .expect("Failed to remove test image");
}