use merlin::services::docker_config_service::DockerConfigService;
use serde_json::json;
use warp::test::request;

#[tokio::test]
async fn test_post_docker_validate_secure_config() {
    // Test validation of secure Docker configuration
    let secure_config = serde_json::json!({
        "security_profile": {
            "user": "1000:1000",
            "read_only": true,
            "capabilities": {
                "drop": ["ALL"],
                "add": ["CHOWN", "SETGID", "SETUID"]
            },
            "no_new_privileges": true
        },
        "resource_limits": {
            "memory": "512m",
            "cpu": "1.0",
            "pids": 100
        },
        "network_mode": "none"
    });

    let service = DockerConfigService::new().unwrap();
    let response = request()
        .method("POST")
        .path("/api/v1/docker/validate")
        .json(&secure_config)
        .reply(&service.validate_endpoint())
        .await;

    // Should pass validation (will fail until implemented)
    assert_eq!(response.status(), 200, "Secure config should pass validation");

    let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();
    assert_eq!(body["status"], "valid");
    assert!(body["score"].as_u64().unwrap() >= 80, "Secure config should have high security score");
    assert!(body["passed_checks"].as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn test_post_docker_validate_insecure_config() {
    // Test validation of insecure Docker configuration
    let insecure_config = serde_json::json!({
        "security_profile": {
            "user": "root:root",  // Running as root
            "read_only": false,   // Writable filesystem
            "capabilities": {
                "drop": [],       // No capabilities dropped
                "add": ["ALL"]    // All capabilities added
            },
            "no_new_privileges": false
        },
        "resource_limits": {
            "memory": "0",        // No memory limit
            "cpu": "0",           // No CPU limit
            "pids": 0             // No PIDs limit
        }
    });

    let service = DockerConfigService::new().unwrap();
    let response = request()
        .method("POST")
        .path("/api/v1/docker/validate")
        .json(&insecure_config)
        .reply(&service.validate_endpoint())
        .await;

    // Should fail validation
    assert_eq!(response.status(), 400, "Insecure config should fail validation");

    let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();
    assert_eq!(body["status"], "invalid");
    assert!(body["score"].as_u64().unwrap() < 50, "Insecure config should have low security score");
    assert!(body["violations"].as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn test_post_docker_validate_partial_config() {
    // Test validation of partial configuration
    let partial_config = serde_json::json!({
        "security_profile": {
            "user": "1000:1000",
            "read_only": true
            // Missing other security fields
        }
    });

    let service = DockerConfigService::new().unwrap();
    let response = request()
        .method("POST")
        .path("/api/v1/docker/validate")
        .json(&partial_config)
        .reply(&service.validate_endpoint())
        .await;

    // Should provide recommendations for missing security settings
    assert_eq!(response.status(), 200, "Partial config should return validation results");

    let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();
    assert_eq!(body["status"], "partial");
    assert!(body["recommendations"].as_array().unwrap().len() > 0);
    assert!(body["missing_fields"].as_array().unwrap().len() > 0);
}

#[tokio::test]
async fn test_post_docker_validate_compliance_check() {
    // Test compliance validation against security standards
    let compliance_config = serde_json::json!({
        "security_profile": {
            "user": "1000:1000",
            "read_only": true,
            "no_new_privileges": true,
            "security_opt": ["no-new-privileges"],
            "capabilities": {
                "drop": ["ALL"],
                "add": ["CHOWN", "SETGID", "SETUID"]
            }
        },
        "compliance_standards": ["CIS_Docker_Benchmark", "NIST_800_190"]
    });

    let service = DockerConfigService::new().unwrap();
    let response = request()
        .method("POST")
        .path("/api/v1/docker/validate")
        .json(&compliance_config)
        .reply(&service.validate_endpoint())
        .await;

    // Should return compliance results
    assert_eq!(response.status(), 200, "Compliance check should return results");

    let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();
    assert_eq!(body["status"], "valid");
    assert!(body["compliance"].as_object().is_some());

    let compliance = body["compliance"].as_object().unwrap();
    assert!(compliance.contains_key("CIS_Docker_Benchmark"));
    assert!(compliance.contains_key("NIST_800_190"));
}

#[tokio::test]
async fn test_post_docker_validate_invalid_json() {
    // Test validation with invalid JSON structure
    let invalid_config = serde_json::json!({
        "security_profile": {
            "user": 12345,  // Invalid type - should be string
            "read_only": "yes"  // Invalid type - should be boolean
        }
    });

    let service = DockerConfigService::new().unwrap();
    let response = request()
        .method("POST")
        .path("/api/v1/docker/validate")
        .json(&invalid_config)
        .reply(&service.validate_endpoint())
        .await;

    // Should fail due to invalid JSON structure
    assert_eq!(response.status(), 400, "Invalid JSON should fail validation");

    let body: serde_json::Value = serde_json::from_slice(&response.body()).unwrap();
    assert_eq!(body["status"], "error");
    assert!(body["message"].as_str().unwrap().contains("validation"));
}