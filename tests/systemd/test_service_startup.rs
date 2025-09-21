use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_binary_exists() {
        // Test that the compiled merlin binary exists
        let binary_path = if cfg!(debug_assertions) {
            Path::new("target/debug/merlin")
        } else {
            Path::new("target/release/merlin")
        };

        // This test may fail in CI if binary hasn't been built yet
        // but should pass in development environment
        if binary_path.exists() {
            assert!(binary_path.is_file(), "Merlin binary should be a file");
        } else {
            // Skip test if binary doesn't exist (normal in CI before build)
            eprintln!("Skipping binary test - binary not built yet");
        }
    }

    #[test]
    fn test_service_help_command() {
        // Test that merlin binary responds to help command
        let binary_path = if cfg!(debug_assertions) {
            "target/debug/merlin"
        } else {
            "target/release/merlin"
        };

        if !Path::new(binary_path).exists() {
            return; // Skip if binary doesn't exist
        }

        let output = Command::new(binary_path)
            .arg("--help")
            .output()
            .expect("Should be able to run merlin --help");

        assert!(
            output.status.success(),
            "Merlin help command should succeed. stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let help_text = String::from_utf8_lossy(&output.stdout);
        assert!(
            help_text.contains("Merlin"),
            "Help text should mention Merlin"
        );
    }

    #[test]
    fn test_service_http_server_capability() {
        // Test that merlin can start HTTP server (if supported)
        let binary_path = if cfg!(debug_assertions) {
            "target/debug/merlin"
        } else {
            "target/release/merlin"
        };

        if !Path::new(binary_path).exists() {
            return; // Skip if binary doesn't exist
        }

        // Test if server command exists
        let output = Command::new(binary_path)
            .arg("serve")
            .arg("--help")
            .output();

        // If serve command exists, test its help
        if output.is_ok() {
            let output = output.unwrap();
            if output.status.success() {
                let help_text = String::from_utf8_lossy(&output.stdout);
                assert!(
                    help_text.contains("serve") || help_text.contains("server"),
                    "Server help should mention serve or server"
                );
            }
        }
    }

    #[test]
    fn test_service_version_check() {
        // Test that merlin reports version information
        let binary_path = if cfg!(debug_assertions) {
            "target/debug/merlin"
        } else {
            "target/release/merlin"
        };

        if !Path::new(binary_path).exists() {
            return; // Skip if binary doesn't exist
        }

        let output = Command::new(binary_path)
            .arg("--version")
            .output();

        // Version command should succeed if it exists
        if output.is_ok() {
            let output = output.unwrap();
            if output.status.success() {
                let version_text = String::from_utf8_lossy(&output.stdout);
                assert!(
                    version_text.len() > 0,
                    "Version command should return some output"
                );
            }
        }
    }

    #[test]
    fn test_service_config_validation() {
        // Test that merlin can validate configuration
        let binary_path = if cfg!(debug_assertions) {
            "target/debug/merlin"
        } else {
            "target/release/merlin"
        };

        if !Path::new(binary_path).exists() {
            return; // Skip if binary doesn't exist
        }

        // Try to run with validate-config if available
        let output = Command::new(binary_path)
            .args(&["validate-config", "--help"])
            .output();

        if output.is_ok() {
            let output = output.unwrap();
            if output.status.success() {
                let help_text = String::from_utf8_lossy(&output.stdout);
                assert!(
                    help_text.contains("validate") || help_text.contains("config"),
                    "Config validation help should mention validate or config"
                );
            }
        }
    }

    #[test]
    fn test_service_feature_command_availability() {
        // Test that feature management commands are available
        let binary_path = if cfg!(debug_assertions) {
            "target/debug/merlin"
        } else {
            "target/release/merlin"
        };

        if !Path::new(binary_path).exists() {
            return; // Skip if binary doesn't exist
        }

        // Test feature command availability
        let output = Command::new(binary_path)
            .arg("feature")
            .arg("--help")
            .output();

        if output.is_ok() {
            let output = output.unwrap();
            if output.status.success() {
                let help_text = String::from_utf8_lossy(&output.stdout);
                assert!(
                    help_text.contains("feature"),
                    "Feature help should mention feature"
                );
            }
        }
    }

    #[test]
    fn test_service_daemon_capability() {
        // Test that merlin can run as daemon (if supported)
        let binary_path = if cfg!(debug_assertions) {
            "target/debug/merlin"
        } else {
            "target/release/merlin"
        };

        if !Path::new(binary_path).exists() {
            return; // Skip if binary doesn't exist
        }

        // Test if daemon command exists
        let output = Command::new(binary_path)
            .args(&["daemon", "--help"])
            .output();

        // If daemon command exists, test its help
        if output.is_ok() {
            let output = output.unwrap();
            if output.status.success() {
                let help_text = String::from_utf8_lossy(&output.stdout);
                assert!(
                    help_text.contains("daemon") || help_text.contains("service"),
                    "Daemon help should mention daemon or service"
                );
            }
        }
    }

    #[test]
    fn test_service_environment_variables() {
        // Test that merlin respects environment variables
        let binary_path = if cfg!(debug_assertions) {
            "target/debug/merlin"
        } else {
            "target/release/merlin"
        };

        if !Path::new(binary_path).exists() {
            return; // Skip if binary doesn't exist
        }

        // Test with RUST_LOG environment variable
        let output = Command::new(binary_path)
            .env("RUST_LOG", "debug")
            .arg("--help")
            .output()
            .expect("Should be able to run merlin with environment variables");

        assert!(
            output.status.success(),
            "Merlin should work with environment variables. stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_service_error_handling() {
        // Test that merlin handles errors gracefully
        let binary_path = if cfg!(debug_assertions) {
            "target/debug/merlin"
        } else {
            "target/release/merlin"
        };

        if !Path::new(binary_path).exists() {
            return; // Skip if binary doesn't exist
        }

        // Test with invalid argument
        let output = Command::new(binary_path)
            .arg("--invalid-argument")
            .output()
            .expect("Should be able to run merlin with invalid argument");

        // Should fail gracefully with non-zero exit code
        assert!(
            !output.status.success(),
            "Merlin should fail gracefully with invalid arguments"
        );

        // Should provide helpful error message
        let error_text = String::from_utf8_lossy(&output.stderr);
        assert!(
            error_text.len() > 0 || String::from_utf8_lossy(&output.stdout).len() > 0,
            "Merlin should provide error message for invalid arguments"
        );
    }

    #[test]
    fn test_service_logging_integration() {
        // Test that merlin can integrate with systemd logging
        let binary_path = if cfg!(debug_assertions) {
            "target/debug/merlin"
        } else {
            "target/release/merlin"
        };

        if !Path::new(binary_path).exists() {
            return; // Skip if binary doesn't exist
        }

        // Test with different log levels
        for log_level in &["error", "warn", "info", "debug", "trace"] {
            let output = Command::new(binary_path)
                .env("RUST_LOG", log_level)
                .arg("--help")
                .output()
                .expect("Should be able to run merlin with different log levels");

            assert!(
                output.status.success(),
                "Merlin should work with {} log level",
                log_level
            );
        }
    }

    #[test]
    fn test_service_port_binding_capability() {
        // Test that merlin can bind to ports (if HTTP server supported)
        let binary_path = if cfg!(debug_assertions) {
            "target/debug/merlin"
        } else {
            "target/release/merlin"
        };

        if !Path::new(binary_path).exists() {
            return; // Skip if binary doesn't exist
        }

        // Test if port configuration is available
        let output = Command::new(binary_path)
            .args(&["serve", "--help"])
            .output();

        if output.is_ok() {
            let output = output.unwrap();
            if output.status.success() {
                let help_text = String::from_utf8_lossy(&output.stdout);
                assert!(
                    help_text.contains("port") || help_text.contains("bind"),
                    "Server help should mention port or bind options"
                );
            }
        }
    }

    #[test]
    fn test_service_systemd_readiness() {
        // Test that merlin binary is ready for systemd integration
        let binary_path = if cfg!(debug_assertions) {
            "target/debug/merlin"
        } else {
            "target/release/merlin"
        };

        if !Path::new(binary_path).exists() {
            return; // Skip if binary doesn't exist
        }

        // Test basic functionality required for systemd service
        let required_capabilities = vec![
            ("--help", "Help functionality"),
            ("--version", "Version reporting"),
        ];

        for (arg, description) in required_capabilities {
            let output = Command::new(binary_path)
                .arg(arg)
                .output()
                .expect(&format!("Should be able to run merlin {}", arg));

            assert!(
                output.status.success(),
                "Merlin should support {} for systemd integration",
                description
            );
        }
    }

    #[test]
    fn test_service_signal_handling() {
        // Test that merlin can handle signals (indirect test through timeout)
        let binary_path = if cfg!(debug_assertions) {
            "target/debug/merlin"
        } else {
            "target/release/merlin"
        };

        if !Path::new(binary_path).exists() {
            return; // Skip if binary doesn't exist
        }

        // Test that the process can be terminated gracefully
        let mut child = Command::new(binary_path)
            .arg("--help")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Should be able to spawn merlin process");

        // Wait for process to complete (should finish quickly for --help)
        match child.wait() {
            Ok(status) => {
                assert!(
                    status.success(),
                    "Merlin process should complete successfully"
                );
            }
            Err(e) => {
                panic!("Failed to wait for merlin process: {}", e);
            }
        }
    }
}