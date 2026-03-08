//! Integration tests for security hardening system

use merlin::security::hardening::{
    SecurityHardeningService, SecurityHardeningConfig, SecurityControl, SecurityProfile,
    ResourceLimits, SecurityContext, CapabilityConfig, NetworkMode, HardeningResult
};
use merlin::models::docker_config::DockerContainerConfig;
use merlin::models::security_scan_config::ComplianceStandard;

#[tokio::test]
async fn test_security_hardening_service_creation() {
    let service = SecurityHardeningService::new();

    // Initialize default profiles
    service.initialize_default_profiles().await.expect("Failed to initialize default profiles");

    // Verify service was created successfully
    let profiles = service.get_security_profiles().await;
    assert!(profiles.len() >= 2); // Should have at least minimal and strict profiles

    // Verify profile names
    let profile_names: Vec<String> = profiles.iter().map(|p| p.name.clone()).collect();
    assert!(profile_names.contains(&"minimal".to_string()));
    assert!(profile_names.contains(&"strict".to_string()));
}

#[tokio::test]
async fn test_security_hardening_config() {
    let config = SecurityHardeningConfig {
        enable_seccomp: true,
        enable_apparmor: true,
        enable_selinux: false,
        read_only_root_filesystem: true,
        drop_all_capabilities: true,
        no_new_privileges: true,
        user_namespace_remap: false,
        network_mode: NetworkMode::Bridge,
        resource_limits: ResourceLimits {
            max_memory_mb: 512,
            max_cpu_cores: 1.0,
            max_pids: 100,
            max_open_files: 1024,
            max_processes: 50,
        },
        security_context: SecurityContext {
            run_as_user: Some(1000),
            run_as_group: Some(1000),
            fs_group: Some(1000),
            privileged: false,
            allow_privilege_escalation: false,
            read_only_root_filesystem: true,
            run_as_non_root: true,
            capabilities: CapabilityConfig {
                add: vec![],
                drop: vec!["ALL".to_string()],
            },
        },
        compliance_standards: vec![ComplianceStandard::CISDockerBenchmark],
    };

    assert!(config.enable_seccomp);
    assert!(config.enable_apparmor);
    assert!(!config.enable_selinux);
    assert!(config.read_only_root_filesystem);
    assert!(config.drop_all_capabilities);
    assert!(config.no_new_privileges);
    assert!(!config.user_namespace_remap);
    assert_eq!(config.resource_limits.max_memory_mb, 512);
    assert_eq!(config.compliance_standards.len(), 1);
}

#[tokio::test]
async fn test_security_profile_creation() {
    let service = SecurityHardeningService::new();

    let profile = SecurityProfile {
        name: "test-profile".to_string(),
        description: "Test security profile".to_string(),
        controls: vec![
            SecurityControl::NonRootUser,
            SecurityControl::ReadOnlyFilesystem,
        ],
        configuration: SecurityHardeningConfig::default(),
        compliance_standards: vec![ComplianceStandard::CISDockerBenchmark],
    };

    service.create_security_profile(profile.clone()).await
        .expect("Failed to create security profile");

    // Verify profile was created
    let retrieved_profile = service.get_security_profile("test-profile").await;
    assert!(retrieved_profile.is_some());

    let retrieved = retrieved_profile.unwrap();
    assert_eq!(retrieved.name, "test-profile");
    assert_eq!(retrieved.description, "Test security profile");
    assert_eq!(retrieved.controls.len(), 2);
}

#[tokio::test]
async fn test_security_hardening_application() {
    let service = SecurityHardeningService::new();
    service.initialize_default_profiles().await.expect("Failed to initialize default profiles");

    // Create a test container configuration
    let config = DockerContainerConfig::builder()
        .image_name("nginx:latest".to_string())
        .dockerfile_path("Dockerfile".to_string())
        .build()
        .expect("Failed to create container config");

    // Apply hardening with minimal profile
    let minimal_profile = service.get_security_profile("minimal").await
        .expect("Minimal profile not found");

    let result = service.apply_hardening(&config, minimal_profile.configuration).await
        .expect("Failed to apply hardening");

    // Verify result
    assert!(result.applied_controls.len() > 0);
    assert!(result.compliance_score >= 0.0);
    assert!(result.compliance_score <= 100.0);
    assert_eq!(result.config_id, config.id);
}

#[tokio::test]
async fn test_security_recommendations() {
    let service = SecurityHardeningService::new();

    // Create an insecure container configuration
    let config = DockerContainerConfig::builder()
        .image_name("nginx:latest".to_string())
        .dockerfile_path("Dockerfile".to_string())
        .privileged(true)
        .read_only(Some(false))
        .user(Some("root".to_string()))
        .build()
        .expect("Failed to create container config");

    let recommendations = service.generate_recommendations(&config).await;

    // Should have multiple recommendations for this insecure config
    assert!(recommendations.len() >= 3);

    // Check for specific recommendations
    let recommendation_strings: Vec<String> = recommendations.iter().map(|r| r.to_lowercase()).collect();
    assert!(recommendation_strings.iter().any(|r| r.contains("read-only")));
    assert!(recommendation_strings.iter().any(|r| r.contains("non-root")));
    assert!(recommendation_strings.iter().any(|r| r.contains("privileged")));
}

#[tokio::test]
async fn test_hardening_result_storage() {
    let service = SecurityHardeningService::new();
    service.initialize_default_profiles().await.expect("Failed to initialize default profiles");

    // Create a test container configuration
    let config = DockerContainerConfig::builder()
        .image_name("nginx:latest".to_string())
        .dockerfile_path("Dockerfile".to_string())
        .build()
        .expect("Failed to create container config");

    // Apply hardening
    let minimal_profile = service.get_security_profile("minimal").await
        .expect("Minimal profile not found");

    let result = service.apply_hardening(&config, minimal_profile.configuration).await
        .expect("Failed to apply hardening");

    // Retrieve the result
    let retrieved_result = service.get_hardening_result(result.id).await;
    assert!(retrieved_result.is_some());

    let retrieved = retrieved_result.unwrap();
    assert_eq!(retrieved.id, result.id);
    assert_eq!(retrieved.config_id, config.id);
    assert_eq!(retrieved.applied_controls.len(), result.applied_controls.len());

    // Get results for the configuration
    let config_results = service.get_config_hardening_results(config.id).await;
    assert!(config_results.len() >= 1);
    assert!(config_results.iter().any(|r| r.id == result.id));
}

#[tokio::test]
async fn test_security_control_variants() {
    // Test all security control variants
    let controls = vec![
        SecurityControl::SeccompProfile,
        SecurityControl::AppArmorProfile,
        SecurityControl::SELinuxContext,
        SecurityControl::ReadOnlyFilesystem,
        SecurityControl::CapabilityDropping,
        SecurityControl::NonRootUser,
        SecurityControl::NetworkIsolation,
        SecurityControl::ResourceLimiting,
        SecurityControl::NamespaceIsolation,
        SecurityControl::MountHardening,
        SecurityControl::Custom("custom-control".to_string()),
    ];

    for control in controls {
        match control {
            SecurityControl::SeccompProfile => assert!(true),
            SecurityControl::AppArmorProfile => assert!(true),
            SecurityControl::SELinuxContext => assert!(true),
            SecurityControl::ReadOnlyFilesystem => assert!(true),
            SecurityControl::CapabilityDropping => assert!(true),
            SecurityControl::NonRootUser => assert!(true),
            SecurityControl::NetworkIsolation => assert!(true),
            SecurityControl::ResourceLimiting => assert!(true),
            SecurityControl::NamespaceIsolation => assert!(true),
            SecurityControl::MountHardening => assert!(true),
            SecurityControl::Custom(name) => assert_eq!(name, "custom-control"),
        }
    }
}

#[tokio::test]
async fn test_network_mode_variants() {
    // Test all network mode variants
    let modes = vec![
        NetworkMode::None,
        NetworkMode::Host,
        NetworkMode::Bridge,
        NetworkMode::Custom("custom-network".to_string()),
    ];

    for mode in modes {
        match mode {
            NetworkMode::None => assert!(true),
            NetworkMode::Host => assert!(true),
            NetworkMode::Bridge => assert!(true),
            NetworkMode::Custom(name) => assert_eq!(name, "custom-network"),
        }
    }
}

#[tokio::test]
async fn test_resource_limits() {
    let limits = ResourceLimits {
        max_memory_mb: 1024,
        max_cpu_cores: 2.0,
        max_pids: 200,
        max_open_files: 2048,
        max_processes: 100,
    };

    assert_eq!(limits.max_memory_mb, 1024);
    assert_eq!(limits.max_cpu_cores, 2.0);
    assert_eq!(limits.max_pids, 200);
    assert_eq!(limits.max_open_files, 2048);
    assert_eq!(limits.max_processes, 100);
}

#[tokio::test]
async fn test_security_context() {
    let context = SecurityContext {
        run_as_user: Some(1000),
        run_as_group: Some(1000),
        fs_group: Some(1000),
        privileged: false,
        allow_privilege_escalation: false,
        read_only_root_filesystem: true,
        run_as_non_root: true,
        capabilities: CapabilityConfig {
            add: vec!["NET_BIND_SERVICE".to_string()],
            drop: vec!["ALL".to_string()],
        },
    };

    assert_eq!(context.run_as_user, Some(1000));
    assert_eq!(context.run_as_group, Some(1000));
    assert_eq!(context.fs_group, Some(1000));
    assert!(!context.privileged);
    assert!(!context.allow_privilege_escalation);
    assert!(context.read_only_root_filesystem);
    assert!(context.run_as_non_root);
    assert_eq!(context.capabilities.add.len(), 1);
    assert_eq!(context.capabilities.drop.len(), 1);
}

#[tokio::test]
async fn test_capability_config() {
    let capabilities = CapabilityConfig {
        add: vec!["NET_BIND_SERVICE".to_string(), "CHOWN".to_string()],
        drop: vec!["ALL".to_string(), "SYS_ADMIN".to_string()],
    };

    assert_eq!(capabilities.add.len(), 2);
    assert_eq!(capabilities.drop.len(), 2);
    assert!(capabilities.add.contains(&"NET_BIND_SERVICE".to_string()));
    assert!(capabilities.drop.contains(&"ALL".to_string()));
}

#[tokio::test]
async fn test_strict_vs_minimal_profiles() {
    let service = SecurityHardeningService::new();
    service.initialize_default_profiles().await.expect("Failed to initialize default profiles");

    let minimal_profile = service.get_security_profile("minimal").await
        .expect("Minimal profile not found");
    let strict_profile = service.get_security_profile("strict").await
        .expect("Strict profile not found");

    // Strict profile should have more controls
    assert!(strict_profile.controls.len() > minimal_profile.controls.len());

    // Strict profile should have more compliance standards
    assert!(strict_profile.compliance_standards.len() >= minimal_profile.compliance_standards.len());

    // Strict profile should have tighter resource limits
    assert!(strict_profile.configuration.resource_limits.max_memory_mb < minimal_profile.configuration.resource_limits.max_memory_mb);
    assert!(strict_profile.configuration.resource_limits.max_cpu_cores < minimal_profile.configuration.resource_limits.max_cpu_cores);
}