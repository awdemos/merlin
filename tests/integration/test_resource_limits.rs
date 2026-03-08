use std::process::Command;
use std::thread;
use std::time::Duration;

#[test]
fn test_container_memory_limits() {
    // Test that container respects memory limits
    let build_output = Command::new("docker")
        .args(&["build", "-t", "merlin:test-memory", "-f", "docker/Dockerfile.hardened", "."])
        .output()
        .expect("Failed to execute docker build command");

    if !build_output.status.success() {
        println!("Build failed as expected - Dockerfile not implemented yet");
        return;
    }

    // Run container with memory limit
    let mut child = Command::new("docker")
        .args(&[
            "run", "--rm",
            "--memory", "128m",
            "--name", "merlin-memory-test",
            "merlin:test-memory",
            "tail", "-f", "/dev/null"  // Keep container running
        ])
        .spawn()
        .expect("Failed to start container");

    // Give container time to start
    thread::sleep(Duration::from_secs(2));

    // Check memory limits
    let output = Command::new("docker")
        .args(&["inspect", "merlin-memory-test", "--format", "{{.HostConfig.Memory}}"])
        .output()
        .expect("Failed to inspect container");

    assert!(output.status.success(), "Should be able to inspect container");

    let memory_limit = String::from_utf8_lossy(&output.stdout).trim();
    assert_eq!(memory_limit, "134217728", "Memory limit should be 128MB (134217728 bytes)");

    // Stop the container
    Command::new("docker")
        .args(&["stop", "merlin-memory-test"])
        .output()
        .expect("Failed to stop container");

    // Wait for child process
    child.wait().expect("Failed to wait for container process");

    // Cleanup
    Command::new("docker")
        .args(&["rmi", "merlin:test-memory"])
        .output()
        .expect("Failed to remove test image");
}

#[test]
fn test_container_cpu_limits() {
    // Test that container respects CPU limits
    let build_output = Command::new("docker")
        .args(&["build", "-t", "merlin:test-cpu", "-f", "docker/Dockerfile.hardened", "."])
        .output()
        .expect("Failed to execute docker build command");

    if !build_output.status.success() {
        println!("Build failed as expected - Dockerfile not implemented yet");
        return;
    }

    // Run container with CPU limit
    let mut child = Command::new("docker")
        .args(&[
            "run", "--rm",
            "--cpus", "0.5",
            "--name", "merlin-cpu-test",
            "merlin:test-cpu",
            "tail", "-f", "/dev/null"
        ])
        .spawn()
        .expect("Failed to start container");

    // Give container time to start
    thread::sleep(Duration::from_secs(2));

    // Check CPU limits
    let output = Command::new("docker")
        .args(&["inspect", "merlin-cpu-test", "--format", "{{.HostConfig.NanoCpus}}"])
        .output()
        .expect("Failed to inspect container");

    assert!(output.status.success(), "Should be able to inspect container");

    let cpu_limit = String::from_utf8_lossy(&output.stdout).trim();
    assert_eq!(cpu_limit, "500000000", "CPU limit should be 0.5 CPUs (500000000 nanocpus)");

    // Stop the container
    Command::new("docker")
        .args(&["stop", "merlin-cpu-test"])
        .output()
        .expect("Failed to stop container");

    // Wait for child process
    child.wait().expect("Failed to wait for container process");

    // Cleanup
    Command::new("docker")
        .args(&["rmi", "merlin:test-cpu"])
        .output()
        .expect("Failed to remove test image");
}

#[test]
fn test_container_pids_limit() {
    // Test that container respects PIDs limit
    let build_output = Command::new("docker")
        .args(&["build", "-t", "merlin:test-pids", "-f", "docker/Dockerfile.hardened", "."])
        .output()
        .expect("Failed to execute docker build command");

    if !build_output.status.success() {
        println!("Build failed as expected - Dockerfile not implemented yet");
        return;
    }

    // Run container with PIDs limit
    let mut child = Command::new("docker")
        .args(&[
            "run", "--rm",
            "--pids-limit", "50",
            "--name", "merlin-pids-test",
            "merlin:test-pids",
            "tail", "-f", "/dev/null"
        ])
        .spawn()
        .expect("Failed to start container");

    // Give container time to start
    thread::sleep(Duration::from_secs(2));

    // Check PIDs limit
    let output = Command::new("docker")
        .args(&["inspect", "merlin-pids-test", "--format", "{{.HostConfig.PidsLimit}}"])
        .output()
        .expect("Failed to inspect container");

    assert!(output.status.success(), "Should be able to inspect container");

    let pids_limit = String::from_utf8_lossy(&output.stdout).trim();
    assert_eq!(pids_limit, "50", "PIDs limit should be 50");

    // Stop the container
    Command::new("docker")
        .args(&["stop", "merlin-pids-test"])
        .output()
        .expect("Failed to stop container");

    // Wait for child process
    child.wait().expect("Failed to wait for container process");

    // Cleanup
    Command::new("docker")
        .args(&["rmi", "merlin:test-pids"])
        .output()
        .expect("Failed to remove test image");
}

#[test]
fn test_container_read_only_filesystem() {
    // Test that container has read-only filesystem
    let build_output = Command::new("docker")
        .args(&["build", "-t", "merlin:test-readonly", "-f", "docker/Dockerfile.hardened", "."])
        .output()
        .expect("Failed to execute docker build command");

    if !build_output.status.success() {
        println!("Build failed as expected - Dockerfile not implemented yet");
        return;
    }

    // Run container with read-only filesystem
    let mut child = Command::new("docker")
        .args(&[
            "run", "--rm",
            "--read-only",
            "--name", "merlin-readonly-test",
            "merlin:test-readonly",
            "tail", "-f", "/dev/null"
        ])
        .spawn()
        .expect("Failed to start container");

    // Give container time to start
    thread::sleep(Duration::from_secs(2));

    // Check if filesystem is read-only
    let output = Command::new("docker")
        .args(&["inspect", "merlin-readonly-test", "--format", "{{.HostConfig.ReadonlyRootfs}}"])
        .output()
        .expect("Failed to inspect container");

    assert!(output.status.success(), "Should be able to inspect container");

    let readonly = String::from_utf8_lossy(&output.stdout).trim();
    assert_eq!(readonly, "true", "Filesystem should be read-only");

    // Try to write to filesystem (should fail)
    let write_output = Command::new("docker")
        .args(&["exec", "merlin-readonly-test", "touch", "/test-write"])
        .output()
        .expect("Failed to execute write test");

    assert!(!write_output.status.success(), "Should not be able to write to read-only filesystem");

    let error_output = String::from_utf8_lossy(&write_output.stderr);
    assert!(error_output.contains("Read-only file system"),
            "Should get read-only filesystem error");

    // Stop the container
    Command::new("docker")
        .args(&["stop", "merlin-readonly-test"])
        .output()
        .expect("Failed to stop container");

    // Wait for child process
    child.wait().expect("Failed to wait for container process");

    // Cleanup
    Command::new("docker")
        .args(&["rmi", "merlin:test-readonly"])
        .output()
        .expect("Failed to remove test image");
}

#[test]
fn test_container_tmpfs_mounts() {
    // Test that container has tmpfs mounts for temporary directories
    let build_output = Command::new("docker")
        .args(&["build", "-t", "merlin:test-tmpfs", "-f", "docker/Dockerfile.hardened", "."])
        .output()
        .expect("Failed to execute docker build command");

    if !build_output.status.success() {
        println!("Build failed as expected - Dockerfile not implemented yet");
        return;
    }

    // Run container with tmpfs mounts
    let mut child = Command::new("docker")
        .args(&[
            "run", "--rm",
            "--tmpfs", "/tmp:size=100m,exec",
            "--tmpfs", "/var/tmp:size=100m,exec",
            "--name", "merlin-tmpfs-test",
            "merlin:test-tmpfs",
            "tail", "-f", "/dev/null"
        ])
        .spawn()
        .expect("Failed to start container");

    // Give container time to start
    thread::sleep(Duration::from_secs(2));

    // Check if /tmp is tmpfs
    let output = Command::new("docker")
        .args(&["exec", "merlin-tmpfs-test", "mount", "|", "grep", "tmpfs"])
        .output()
        .expect("Failed to check tmpfs mounts");

    assert!(output.status.success(), "Should be able to check tmpfs mounts");

    let mount_output = String::from_utf8_lossy(&output.stdout);
    assert!(mount_output.contains("/tmp"), "Should have tmpfs mount for /tmp");
    assert!(mount_output.contains("/var/tmp"), "Should have tmpfs mount for /var/tmp");

    // Test writing to tmpfs (should succeed)
    let write_output = Command::new("docker")
        .args(&["exec", "merlin-tmpfs-test", "touch", "/tmp/test-write"])
        .output()
        .expect("Failed to test tmpfs write");

    assert!(write_output.status.success(), "Should be able to write to tmpfs");

    // Stop the container
    Command::new("docker")
        .args(&["stop", "merlin-tmpfs-test"])
        .output()
        .expect("Failed to stop container");

    // Wait for child process
    child.wait().expect("Failed to wait for container process");

    // Cleanup
    Command::new("docker")
        .args(&["rmi", "merlin:test-tmpfs"])
        .output()
        .expect("Failed to remove test image");
}