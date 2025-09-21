use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_systemd_service_file_exists() {
        // This test should fail initially until we create the service file
        let service_file_path = Path::new("systemd/merlin.service");

        // Test should fail: service file doesn't exist yet
        assert!(
            service_file_path.exists(),
            "Systemd service file should exist at systemd/merlin.service"
        );
    }

    #[test]
    fn test_systemd_service_file_valid_format() {
        let service_file_path = Path::new("systemd/merlin.service");

        // Skip if file doesn't exist (will fail initially)
        if !service_file_path.exists() {
            return;
        }

        let content = fs::read_to_string(service_file_path)
            .expect("Should be able to read service file");

        // Test required systemd sections
        assert!(
            content.contains("[Unit]"),
            "Service file should contain [Unit] section"
        );
        assert!(
            content.contains("[Service]"),
            "Service file should contain [Service] section"
        );
        assert!(
            content.contains("[Install]"),
            "Service file should contain [Install] section"
        );

        // Test required Unit section fields
        assert!(
            content.contains("Description="),
            "Service file should have Description"
        );
        assert!(
            content.contains("After="),
            "Service file should have After dependency"
        );

        // Test required Service section fields
        assert!(
            content.contains("ExecStart="),
            "Service file should have ExecStart command"
        );
        assert!(
            content.contains("Type="),
            "Service file should have service Type"
        );

        // Test security settings
        assert!(
            content.contains("DynamicUser="),
            "Service file should configure DynamicUser"
        );
        assert!(
            content.contains("NoNewPrivileges="),
            "Service file should configure NoNewPrivileges"
        );
        assert!(
            content.contains("ProtectSystem="),
            "Service file should configure ProtectSystem"
        );
    }

    #[test]
    fn test_systemd_service_file_security_hardening() {
        let service_file_path = Path::new("systemd/merlin.service");

        // Skip if file doesn't exist (will fail initially)
        if !service_file_path.exists() {
            return;
        }

        let content = fs::read_to_string(service_file_path)
            .expect("Should be able to read service file");

        // Test security hardening requirements
        let security_requirements = vec![
            "DynamicUser=yes",
            "NoNewPrivileges=true",
            "PrivateDevices=true",
            "ProtectSystem=strict",
            "ProtectHome=true",
            "ReadWritePaths=",
            "CapabilityBoundingSet=",
            "AmbientCapabilities=",
            "RemoveIPC=true",
            "PrivateTmp=true",
        ];

        for requirement in security_requirements {
            assert!(
                content.contains(requirement),
                "Service file should include security requirement: {}",
                requirement
            );
        }

        // Test for absence of security anti-patterns
        let anti_patterns = vec![
            "User=root",
            "Group=root",
            "RootDirectory=/",
            "ReadWritePaths=/",
            "NoNewPrivileges=false",
            "PrivateDevices=false",
        ];

        for pattern in anti_patterns {
            assert!(
                !content.contains(pattern),
                "Service file should not contain security anti-pattern: {}",
                pattern
            );
        }
    }

    #[test]
    fn test_systemd_service_file_resource_limits() {
        let service_file_path = Path::new("systemd/merlin.service");

        // Skip if file doesn't exist (will fail initially)
        if !service_file_path.exists() {
            return;
        }

        let content = fs::read_to_string(service_file_path)
            .expect("Should be able to read service file");

        // Test resource limits
        let resource_limits = vec![
            "MemoryMax=",
            "MemoryHigh=",
            "LimitNOFILE=",
            "LimitNPROC=",
            "TimeoutStartSec=",
            "TimeoutStopSec=",
        ];

        for limit in resource_limits {
            assert!(
                content.contains(limit),
                "Service file should configure resource limit: {}",
                limit
            );
        }
    }

    #[test]
    fn test_systemd_service_file_logging_configuration() {
        let service_file_path = Path::new("systemd/merlin.service");

        // Skip if file doesn't exist (will fail initially)
        if !service_file_path.exists() {
            return;
        }

        let content = fs::read_to_string(service_file_path)
            .expect("Should be able to read service file");

        // Test logging configuration
        assert!(
            content.contains("StandardOutput=journal"),
            "Service should log to systemd journal"
        );
        assert!(
            content.contains("StandardError=journal"),
            "Service should log errors to systemd journal"
        );
        assert!(
            content.contains("SyslogIdentifier=merlin"),
            "Service should have proper syslog identifier"
        );
    }

    #[test]
    fn test_systemd_service_file_restart_policy() {
        let service_file_path = Path::new("systemd/merlin.service");

        // Skip if file doesn't exist (will fail initially)
        if !service_file_path.exists() {
            return;
        }

        let content = fs::read_to_string(service_file_path)
            .expect("Should be able to read service file");

        // Test restart policy
        assert!(
            content.contains("Restart="),
            "Service should have restart policy configured"
        );

        // Prefer on-failure restart
        if content.contains("Restart=on-failure") {
            assert!(
                content.contains("RestartSec="),
                "Service should have restart delay when using on-failure"
            );
        }
    }

    #[test]
    fn test_systemd_service_file_installation_section() {
        let service_file_path = Path::new("systemd/merlin.service");

        // Skip if file doesn't exist (will fail initially)
        if !service_file_path.exists() {
            return;
        }

        let content = fs::read_to_string(service_file_path)
            .expect("Should be able to read service file");

        // Test [Install] section
        assert!(
            content.contains("WantedBy="),
            "Service should specify WantedBy target"
        );

        // Should target multi-user.target for system services
        assert!(
            content.contains("multi-user.target"),
            "Service should target multi-user.target"
        );
    }
}