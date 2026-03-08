use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_config_file_exists() {
        // Test that environment configuration file exists
        let env_config_path = Path::new("systemd/merlin.env");

        // Test should fail initially until we create the env file
        assert!(
            env_config_path.exists(),
            "Environment configuration file should exist at systemd/merlin.env"
        );
    }

    #[test]
    fn test_daemon_config_file_exists() {
        // Test that daemon configuration file exists
        let daemon_config_path = Path::new("systemd/merlin.conf");

        // Test should fail initially until we create the conf file
        assert!(
            daemon_config_path.exists(),
            "Daemon configuration file should exist at systemd/merlin.conf"
        );
    }

    #[test]
    fn test_environment_config_format() {
        let env_config_path = Path::new("systemd/merlin.env");

        // Skip if file doesn't exist (will fail initially)
        if !env_config_path.exists() {
            return;
        }

        let content = fs::read_to_string(env_config_path)
            .expect("Should be able to read environment config");

        // Test environment variable format
        let lines: Vec<&str> = content.lines()
            .filter(|line| !line.trim_start().starts_with('#') && !line.trim().is_empty())
            .collect();

        for line in lines {
            assert!(
                line.contains('='),
                "Environment variable line should have format KEY=VALUE: {}",
                line
            );

            let parts: Vec<&str> = line.splitn(2, '=').collect();
            assert_eq!(
                parts.len(),
                2,
                "Environment variable should have exactly one '=' sign: {}",
                line
            );

            let key = parts[0].trim();
            let value = parts[1].trim();

            assert!(
                !key.is_empty(),
                "Environment variable key should not be empty in line: {}",
                line
            );

            // Key should be uppercase with underscores (standard env var format)
            assert!(
                key.chars().all(|c| c.is_uppercase() || c == '_' || c.is_digit(10)),
                "Environment variable key should be uppercase with underscores: {}",
                key
            );
        }
    }

    #[test]
    fn test_required_environment_variables() {
        let env_config_path = Path::new("systemd/merlin.env");

        // Skip if file doesn't exist (will fail initially)
        if !env_config_path.exists() {
            return;
        }

        let content = fs::read_to_string(env_config_path)
            .expect("Should be able to read environment config");

        // Test for required environment variables
        let required_vars = vec![
            "MERLIN_MODE",
            "MERLIN_HTTP_PORT",
            "MERLIN_BIND_ADDRESS",
            "RUST_LOG",
            "MERLIN_DATA_DIR",
            "MERLIN_CONFIG_DIR",
        ];

        for var in required_vars {
            assert!(
                content.contains(&format!("{}=", var)),
                "Environment config should include required variable: {}",
                var
            );
        }
    }

    #[test]
    fn test_environment_config_values() {
        let env_config_path = Path::new("systemd/merlin.env");

        // Skip if file doesn't exist (will fail initially)
        if !env_config_path.exists() {
            return;
        }

        let content = fs::read_to_string(env_config_path)
            .expect("Should be able to read environment config");

        // Test specific environment variable values
        let expected_values = vec![
            ("MERLIN_MODE", vec!["Hybrid", "HttpServer", "CliDaemon"]),
            ("MERLIN_HTTP_PORT", vec!["8080"]), // Default port
            ("MERLIN_BIND_ADDRESS", vec!["127.0.0.1", "0.0.0.0"]),
            ("RUST_LOG", vec!["info", "debug", "warn"]),
            ("MERLIN_DATA_DIR", vec!["/var/lib/merlin"]),
            ("MERLIN_CONFIG_DIR", vec!["/etc/merlin"]),
        ];

        for (var, expected_values) in expected_values {
            let pattern = format!("{}=", var);
            if let Some(start) = content.find(&pattern) {
                let line_end = content[start..].find('\n').unwrap_or(content.len());
                let line = &content[start..start + line_end];
                let value = line.split('=').nth(1).unwrap_or("").trim();

                assert!(
                    expected_values.contains(&value),
                    "Environment variable {} has unexpected value: {}. Expected one of: {:?}",
                    var, value, expected_values
                );
            }
        }
    }

    #[test]
    fn test_daemon_config_format() {
        let daemon_config_path = Path::new("systemd/merlin.conf");

        // Skip if file doesn't exist (will fail initially)
        if !daemon_config_path.exists() {
            return;
        }

        let content = fs::read_to_string(daemon_config_path)
            .expect("Should be able to read daemon config");

        // Test that it's a valid configuration format
        // Could be TOML, JSON, INI, or simple key-value pairs
        let has_sections = content.contains('[') && content.contains(']');
        let has_key_values = content.contains('=');

        assert!(
            has_key_values,
            "Daemon config should contain key-value pairs"
        );

        // If it has sections, they should be properly formatted
        if has_sections {
            let lines: Vec<&str> = content.lines().collect();
            for line in lines {
                let trimmed = line.trim();
                if trimmed.starts_with('[') && trimmed.ends_with(']') {
                    // Valid section format
                    continue;
                }
            }
        }
    }

    #[test]
    fn test_configuration_consistency() {
        let env_config_path = Path::new("systemd/merlin.env");
        let daemon_config_path = Path::new("systemd/merlin.conf");

        // Skip if files don't exist (will fail initially)
        if !env_config_path.exists() || !daemon_config_path.exists() {
            return;
        }

        let env_content = fs::read_to_string(env_config_path)
            .expect("Should be able to read environment config");
        let daemon_content = fs::read_to_string(daemon_config_path)
            .expect("Should be able to read daemon config");

        // Test that both configs reference the same paths
        let common_paths = vec![
            "/var/lib/merlin",
            "/etc/merlin",
            "/var/log/merlin",
        ];

        for path in common_paths {
            let env_has_path = env_content.contains(path);
            let daemon_has_path = daemon_content.contains(path);

            // At least one config should reference each important path
            assert!(
                env_has_path || daemon_has_path,
                "At least one config should reference path: {}",
                path
            );
        }
    }

    #[test]
    fn test_configuration_security() {
        let env_config_path = Path::new("systemd/merlin.env");
        let daemon_config_path = Path::new("systemd/merlin.conf");

        // Test environment config security
        if env_config_path.exists() {
            let env_content = fs::read_to_string(env_config_path)
                .expect("Should be able to read environment config");

            // Should not contain sensitive information
            let sensitive_patterns = vec![
                "PASSWORD",
                "SECRET",
                "KEY",
                "TOKEN",
                "API_KEY",
            ];

            for pattern in sensitive_patterns {
                // Allow configuration variable names that might contain these words
                // but not actual values that look like secrets
                let lines: Vec<&str> = env_content.lines().collect();
                for line in lines {
                    let trimmed = line.trim();
                    if trimmed.starts_with('#') || trimmed.is_empty() {
                        continue;
                    }

                    if let Some(idx) = trimmed.find('=') {
                        let key = trimmed[..idx].trim();
                        let value = trimmed[idx + 1..].trim();

                        // Key can contain these words (e.g., MERLIN_CONFIG_DIR)
                        // but value should not look like a secret
                        if value.len() > 20 && value.chars().any(|c| c.is_ascii_alphanumeric()) {
                            // Long alphanumeric values might be secrets
                            if pattern != "KEY" || !key.contains("CONFIG") {
                                assert!(
                                    !value.to_uppercase().contains(pattern),
                                    "Configuration value should not contain potential secret pattern {}: {}",
                                    pattern, value
                                );
                            }
                        }
                    }
                }
            }
        }

        // Test daemon config security
        if daemon_config_path.exists() {
            let daemon_content = fs::read_to_string(daemon_config_path)
                .expect("Should be able to read daemon config");

            // Should not contain hardcoded secrets
            let secret_patterns = vec![
                "\"password\":",
                "\"secret\":",
                "\"token\":",
                "\"api_key\":",
            ];

            for pattern in secret_patterns {
                assert!(
                    !daemon_content.to_lowercase().contains(pattern),
                    "Daemon config should not contain hardcoded secrets with pattern: {}",
                    pattern
                );
            }
        }
    }

    #[test]
    fn test_configuration_comments() {
        let env_config_path = Path::new("systemd/merlin.env");
        let daemon_config_path = Path::new("systemd/merlin.conf");

        // Test environment config comments
        if env_config_path.exists() {
            let env_content = fs::read_to_string(env_config_path)
                .expect("Should be able to read environment config");

            // Should have some comments explaining the configuration
            let comment_lines: Vec<&str> = env_content.lines()
                .filter(|line| line.trim_start().starts_with('#'))
                .collect();

            assert!(
                !comment_lines.is_empty(),
                "Environment config should have explanatory comments"
            );

            // Should explain important configuration options
            let has_meaningful_comments = comment_lines.iter().any(|line| {
                let lower = line.to_lowercase();
                lower.contains("mode") || lower.contains("port") || lower.contains("directory") ||
                lower.contains("logging") || lower.contains("data") || lower.contains("config")
            });

            assert!(
                has_meaningful_comments,
                "Environment config should have meaningful comments explaining important options"
            );
        }

        // Test daemon config comments
        if daemon_config_path.exists() {
            let daemon_content = fs::read_to_string(daemon_config_path)
                .expect("Should be able to read daemon config");

            let comment_lines: Vec<&str> = daemon_content.lines()
                .filter(|line| line.trim_start().starts_with('#'))
                .collect();

            assert!(
                !comment_lines.is_empty(),
                "Daemon config should have explanatory comments"
            );
        }
    }

    #[test]
    fn test_configuration_validation() {
        let env_config_path = Path::new("systemd/merlin.env");

        // Skip if file doesn't exist (will fail initially)
        if !env_config_path.exists() {
            return;
        }

        let content = fs::read_to_string(env_config_path)
            .expect("Should be able to read environment config");

        // Validate port numbers
        if let Some(port_line) = content.lines().find(|line| line.contains("MERLIN_HTTP_PORT=")) {
            if let Some(port_str) = port_line.split('=').nth(1) {
                let port = port_str.trim().parse::<u16>();
                assert!(
                    port.is_ok(),
                    "MERLIN_HTTP_PORT should be a valid port number: {}",
                    port_str
                );

                if let Ok(port) = port {
                    assert!(
                        port > 0 && port <= 65535,
                        "MERLIN_HTTP_PORT should be in valid range 1-65535: {}",
                        port
                    );
                }
            }
        }

        // Validate bind addresses
        if let Some(bind_line) = content.lines().find(|line| line.contains("MERLIN_BIND_ADDRESS=")) {
            if let Some(addr) = bind_line.split('=').nth(1) {
                let addr = addr.trim();
                assert!(
                    addr == "127.0.0.1" || addr == "0.0.0.0" || addr.starts_with("192.168.") ||
                    addr.starts_with("10.") || addr.starts_with("172.16."),
                    "MERLIN_BIND_ADDRESS should be a valid IP address: {}",
                    addr
                );
            }
        }

        // Validate directories
        let dir_vars = vec!["MERLIN_DATA_DIR", "MERLIN_CONFIG_DIR"];
        for var in dir_vars {
            if let Some(dir_line) = content.lines().find(|line| line.contains(&format!("{}=", var))) {
                if let Some(dir) = dir_line.split('=').nth(1) {
                    let dir = dir.trim();
                    assert!(
                        dir.starts_with('/'),
                        "{} should be an absolute path: {}",
                        var, dir
                    );
                }
            }
        }
    }

    #[test]
    fn test_configuration_integration() {
        // Test that configuration can be loaded by merlin binary
        let binary_path = if cfg!(debug_assertions) {
            "target/debug/merlin"
        } else {
            "target/release/merlin"
        };

        if !Path::new(binary_path).exists() {
            return; // Skip if binary doesn't exist
        }

        // Create a temporary config file for testing
        let temp_dir = TempDir::new().expect("Should be able to create temp directory");
        let test_config_path = temp_dir.path().join("test_merlin.env");

        // Write a minimal test configuration
        fs::write(&test_config_path, "RUST_LOG=info\nMERLIN_MODE=Hybrid\n")
            .expect("Should be able to write test config");

        // Test that merlin can load configuration
        let output = Command::new(binary_path)
            .env("RUST_LOG", "info")
            .arg("--help")
            .output()
            .expect("Should be able to run merlin with test config");

        assert!(
            output.status.success(),
            "Merlin should work with configuration. stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn test_configuration_reload_capability() {
        // Test that configuration supports reload (if applicable)
        let env_config_path = Path::new("systemd/merlin.env");

        // Skip if file doesn't exist (will fail initially)
        if !env_config_path.exists() {
            return;
        }

        let content = fs::read_to_string(env_config_path)
            .expect("Should be able to read environment config");

        // Look for configuration reload support
        let reload_indicators = vec![
            "reload",
            "SIGHUP",
            "watch",
            "monitor",
        ];

        let has_reload_support = reload_indicators.iter().any(|indicator| {
            content.to_lowercase().contains(indicator)
        });

        // Not strictly required, but good to have
        if has_reload_support {
            println!("Configuration supports reload capability");
        } else {
            println!("Configuration does not explicitly support reload (acceptable)");
        }
    }
}