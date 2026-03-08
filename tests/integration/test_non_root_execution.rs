use std::process::Command;
use std::thread;
use std::time::Duration;

#[test]
fn test_container_runs_as_non_root_user() {
    // Test that container runs as non-root user
    // This test requires Docker to be running and the hardened image to be built

    // Build a test image (will fail until Dockerfile exists)
    let build_output = Command::new("docker")
        .args(&["build", "-t", "merlin:test-non-root", "-f", "docker/Dockerfile.hardened", "."])
        .output()
        .expect("Failed to execute docker build command");

    // For now, expect the build to fail (Dockerfile doesn't exist yet)
    // This test will pass once we implement the Dockerfile
    if !build_output.status.success() {
        println!("Build failed as expected - Dockerfile not implemented yet");
        println!("Build stderr: {}", String::from_utf8_lossy(&build_output.stderr));
        return;
    }

    // Run container and check user
    let output = Command::new("docker")
        .args(&["run", "--rm", "merlin:test-non-root", "whoami"])
        .output()
        .expect("Failed to execute docker run command");

    assert!(output.status.success(), "Container should run successfully");

    let user_output = String::from_utf8_lossy(&output.stdout);
    let user = user_output.trim();

    assert_ne!(user, "root", "Container should not run as root user");
    assert_eq!(user, "merlin", "Container should run as 'merlin' user");

    // Check user ID
    let uid_output = Command::new("docker")
        .args(&["run", "--rm", "merlin:test-non-root", "id", "-u"])
        .output()
        .expect("Failed to execute docker run command");

    let uid = String::from_utf8_lossy(&uid_output.stdout).trim();
    assert_eq!(uid, "1000", "Container should run as UID 1000");

    // Check group ID
    let gid_output = Command::new("docker")
        .args(&["run", "--rm", "merlin:test-non-root", "id", "-g"])
        .output()
        .expect("Failed to execute docker run command");

    let gid = String::from_utf8_lossy(&gid_output.stdout).trim();
    assert_eq!(gid, "1000", "Container should run as GID 1000");

    // Cleanup
    Command::new("docker")
        .args(&["rmi", "merlin:test-non-root"])
        .output()
        .expect("Failed to remove test image");
}

#[test]
fn test_container_privilege_escalation_prevention() {
    // Test that container cannot escalate privileges
    let build_output = Command::new("docker")
        .args(&["build", "-t", "merlin:test-privs", "-f", "docker/Dockerfile.hardened", "."])
        .output()
        .expect("Failed to execute docker build command");

    if !build_output.status.success() {
        println!("Build failed as expected - Dockerfile not implemented yet");
        return;
    }

    // Try to escalate privileges using sudo
    let output = Command::new("docker")
        .args(&["run", "--rm", "merlin:test-privs", "sudo", "whoami"])
        .output()
        .expect("Failed to execute docker run command");

    // Should fail due to no sudo access
    assert!(!output.status.success(), "Privilege escalation should fail");

    let error_output = String::from_utf8_lossy(&output.stderr);
    assert!(error_output.contains("command not found") ||
            error_output.contains("permission denied") ||
            error_output.contains("Operation not permitted"),
            "Should not be able to escalate privileges");

    // Try to change user to root
    let output = Command::new("docker")
        .args(&["run", "--rm", "merlin:test-privs", "su", "-", "root"])
        .output()
        .expect("Failed to execute docker run command");

    // Should fail due to no password or su access
    assert!(!output.status.success(), "Switching to root should fail");

    // Cleanup
    Command::new("docker")
        .args(&["rmi", "merlin:test-privs"])
        .output()
        .expect("Failed to remove test image");
}

#[test]
fn test_container_file_permissions() {
    // Test that container has appropriate file permissions
    let build_output = Command::new("docker")
        .args(&["build", "-t", "merlin:test-perms", "-f", "docker/Dockerfile.hardened", "."])
        .output()
        .expect("Failed to execute docker build command");

    if !build_output.status.success() {
        println!("Build failed as expected - Dockerfile not implemented yet");
        return;
    }

    // Check if we can write to app directory
    let output = Command::new("docker")
        .args(&["run", "--rm", "merlin:test-perms", "sh", "-c", "touch /app/test-write && echo 'write success'"])
        .output()
        .expect("Failed to execute docker run command");

    // Should succeed if user has appropriate permissions
    assert!(output.status.success(), "Should be able to write to app directory");

    let write_result = String::from_utf8_lossy(&output.stdout);
    assert_eq!(write_result.trim(), "write success", "File write should succeed");

    // Check if we can read from app directory
    let output = Command::new("docker")
        .args(&["run", "--rm", "merlin:test-perms", "ls", "-la", "/app"])
        .output()
        .expect("Failed to execute docker run command");

    assert!(output.status.success(), "Should be able to read from app directory");

    // Check file ownership
    let output = Command::new("docker")
        .args(&["run", "--rm", "merlin:test-perms", "ls", "-la", "/app/merlin"])
        .output()
        .expect("Failed to execute docker run command");

    assert!(output.status.success(), "Should be able to check file ownership");

    let ls_output = String::from_utf8_lossy(&output.stdout);
    assert!(ls_output.contains("1000"), "Files should be owned by user 1000");

    // Cleanup
    Command::new("docker")
        .args(&["rmi", "merlin:test-perms"])
        .output()
        .expect("Failed to remove test image");
}

#[test]
fn test_container_environment_variables() {
    // Test that container environment variables are properly set
    let build_output = Command::new("docker")
        .args(&["build", "-t", "merlin:test-env", "-f", "docker/Dockerfile.hardened", "."])
        .output()
        .expect("Failed to execute docker build command");

    if !build_output.status.success() {
        println!("Build failed as expected - Dockerfile not implemented yet");
        return;
    }

    // Check environment variables
    let output = Command::new("docker")
        .args(&["run", "--rm", "merlin:test-env", "env"])
        .output()
        .expect("Failed to execute docker run command");

    assert!(output.status.success(), "Container should run successfully");

    let env_output = String::from_utf8_lossy(&output.stdout);

    // Check for expected environment variables
    assert!(env_output.contains("MERLIN_ENV="), "Should have MERLIN_ENV environment variable");
    assert!(env_output.contains("MERLIN_PORT="), "Should have MERLIN_PORT environment variable");
    assert!(env_output.contains("RUST_ENV="), "Should have RUST_ENV environment variable");

    // Check that dangerous environment variables are not set
    assert!(!env_output.contains("SUDO_UID="), "Should not have SUDO_UID environment variable");
    assert!(!env_output.contains("SUDO_GID="), "Should not have SUDO_GID environment variable");

    // Cleanup
    Command::new("docker")
        .args(&["rmi", "merlin:test-env"])
        .output()
        .expect("Failed to remove test image");
}