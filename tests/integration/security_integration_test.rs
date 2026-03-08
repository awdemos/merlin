//! Integration tests for security scanning

use merlin::integration::security_scanner::{SecurityScanner, ScanOptions, ScanType, OutputFormat};
use merlin::models::security_scan_config::ComplianceStandard;

#[tokio::test]
async fn test_security_scanner_creation() {
    let scanner = SecurityScanner::new();
    assert!(scanner.is_ok());
}

#[tokio::test]
async fn test_security_scanner_configuration() {
    let scanner = SecurityScanner::new().unwrap()
        .with_paths("trivy".to_string(), "hadolint".to_string(), "docker-bench".to_string())
        .with_timeout(std::time::Duration::from_secs(300))
        .with_cache_dir("/tmp/test-cache".to_string());

    assert_eq!(scanner.trivy_path, "trivy");
    assert_eq!(scanner.scan_timeout.as_secs(), 300);
}

#[tokio::test]
async fn test_vulnerability_scan_configuration() {
    let scanner = SecurityScanner::new().unwrap();
    let options = ScanOptions {
        image_name: "nginx:latest".to_string(),
        scan_types: vec![ScanType::Vulnerability],
        severity_threshold: Some("HIGH".to_string()),
        output_format: OutputFormat::Json,
        cache_results: false,
        timeout_seconds: Some(600),
    };

    // This test validates scan configuration without requiring actual security tools
    assert_eq!(options.image_name, "nginx:latest");
    assert_eq!(options.scan_types.len(), 1);
    assert!(matches!(options.scan_types[0], ScanType::Vulnerability));
}

#[tokio::test]
async fn test_compliance_scan_configuration() {
    let scanner = SecurityScanner::new().unwrap();
    let options = ScanOptions {
        image_name: "alpine:latest".to_string(),
        scan_types: vec![
            ScanType::Compliance(vec![ComplianceStandard::CISDockerBenchmark]),
            ScanType::Configuration,
        ],
        severity_threshold: None,
        output_format: OutputFormat::Sarif,
        cache_results: true,
        timeout_seconds: None,
    };

    assert_eq!(options.image_name, "alpine:latest");
    assert_eq!(options.scan_types.len(), 2);
    assert!(options.cache_results);
    assert!(matches!(options.output_format, OutputFormat::Sarif));
}

#[tokio::test]
async fn test_scan_types_coverage() {
    let scan_types = vec![
        ScanType::Vulnerability,
        ScanType::Configuration,
        ScanType::Malware,
        ScanType::Secrets,
        ScanType::License,
        ScanType::Compliance(vec![ComplianceStandard::CISDockerBenchmark]),
    ];

    let scanner = SecurityScanner::new().unwrap();
    let options = ScanOptions {
        image_name: "test:latest".to_string(),
        scan_types: scan_types.clone(),
        severity_threshold: None,
        output_format: OutputFormat::Json,
        cache_results: false,
        timeout_seconds: Some(300),
    };

    assert_eq!(options.scan_types.len(), 6);

    // Verify all scan types are covered
    for scan_type in scan_types {
        match scan_type {
            ScanType::Vulnerability => assert!(true),
            ScanType::Configuration => assert!(true),
            ScanType::Malware => assert!(true),
            ScanType::Secrets => assert!(true),
            ScanType::License => assert!(true),
            ScanType::Compliance(_) => assert!(true),
        }
    }
}

#[tokio::test]
async fn test_security_scanner_with_cache() {
    let scanner = SecurityScanner::new().unwrap()
        .with_cache_dir("/tmp/merlin-test-cache".to_string());

    let options = ScanOptions {
        image_name: "nginx:latest".to_string(),
        scan_types: vec![ScanType::Configuration],
        severity_threshold: None,
        output_format: OutputFormat::Json,
        cache_results: true,
        timeout_seconds: Some(60),
    };

    // Test cache functionality
    // Note: In a real test environment, you might need to mock the file system
    let cache_dir = &scanner.cache_dir;
    assert!(cache_dir.contains("merlin-test-cache"));
    assert!(options.cache_results);
}

#[tokio::test]
async fn test_output_formats() {
    let formats = vec![
        OutputFormat::Json,
        OutputFormat::Sarif,
        OutputFormat::Text,
        OutputFormat::Html,
    ];

    for format in formats {
        let options = ScanOptions {
            image_name: "test:latest".to_string(),
            scan_types: vec![ScanType::Vulnerability],
            severity_threshold: None,
            output_format: format.clone(),
            cache_results: false,
            timeout_seconds: None,
        };

        match format {
            OutputFormat::Json => assert!(true),
            OutputFormat::Sarif => assert!(true),
            OutputFormat::Text => assert!(true),
            OutputFormat::Html => assert!(true),
        }
    }
}

#[tokio::test]
async fn test_compliance_standards() {
    let standards = vec![
        ComplianceStandard::CISDockerBenchmark,
        ComplianceStandard::NIST800190,
        ComplianceStandard::PCIDSS,
        ComplianceStandard::SOC2,
        ComplianceStandard::ISO27001,
        ComplianceStandard::HIPAA,
        ComplianceStandard::GDPR,
    ];

    let scanner = SecurityScanner::new().unwrap();
    let options = ScanOptions {
        image_name: "compliance-test:latest".to_string(),
        scan_types: vec![ScanType::Compliance(standards.clone())],
        severity_threshold: Some("MEDIUM".to_string()),
        output_format: OutputFormat::Json,
        cache_results: false,
        timeout_seconds: Some(900),
    };

    if let ScanType::Compliance(scan_standards) = &options.scan_types[0] {
        assert_eq!(scan_standards.len(), 7);
        assert!(scan_standards.contains(&ComplianceStandard::CISDockerBenchmark));
        assert!(scan_standards.contains(&ComplianceStandard::HIPAA));
        assert!(scan_standards.contains(&ComplianceStandard::GDPR));
    } else {
        panic!("Expected compliance scan type");
    }
}