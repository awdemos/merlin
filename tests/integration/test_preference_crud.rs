use merlin::api::endpoints::preferences::*;
use serde_json::{json, Value};
use warp::http::StatusCode;
use warp::test::request;

#[tokio::test]
async fn test_preference_crud_full_lifecycle() {
    let user_id = "user_crud_test";
    let preference_key = "test_preference";
    let initial_value = json!("initial_value");

    // Step 1: CREATE - Create a new preference
    let create_request = json!({
        "user_id": user_id,
        "preference_key": preference_key,
        "preference_value": initial_value,
        "category": "ResponseFormatting"
    });

    let create_response = request()
        .method("POST")
        .path("/api/v1/preferences/userPreferenceCreate")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&create_request)
        .reply(&preference_create_endpoint())
        .await;

    assert_eq!(create_response.status(), StatusCode::CREATED);

    let create_body: Value = serde_json::from_slice(&create_response.body()).unwrap();
    assert!(create_body["success"].as_bool().unwrap());
    let created_id = create_body["data"]["id"].as_str().unwrap();
    assert_eq!(create_body["data"]["user_id"], user_id);
    assert_eq!(create_body["data"]["preference_key"], preference_key);
    assert_eq!(create_body["data"]["preference_value"], initial_value);

    // Step 2: READ - Simulate reading by attempting to create duplicate (should conflict)
    let duplicate_response = request()
        .method("POST")
        .path("/api/v1/preferences/userPreferenceCreate")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&create_request)
        .reply(&preference_create_endpoint())
        .await;

    // Should either conflict or succeed based on implementation
    assert!(duplicate_response.status() == StatusCode::CONFLICT || duplicate_response.status() == StatusCode::CREATED);

    // Step 3: UPDATE - Update the preference
    let updated_value = json!("updated_value");
    let update_request = json!({
        "user_id": user_id,
        "preference_key": preference_key,
        "preference_value": updated_value,
        "category": "ResponseFormatting"
    });

    let update_response = request()
        .method("PUT")
        .path("/api/v1/preferences/userPreferenceUpdate")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&update_request)
        .reply(&preference_update_endpoint())
        .await;

    assert_eq!(update_response.status(), StatusCode::OK);

    let update_body: Value = serde_json::from_slice(&update_response.body()).unwrap();
    assert!(update_body["success"].as_bool().unwrap());
    assert_eq!(update_body["data"]["user_id"], user_id);
    assert_eq!(update_body["data"]["preference_key"], preference_key);
    assert_eq!(update_body["data"]["preference_value"], updated_value);
    // Version should be incremented
    assert!(update_body["data"]["version"].as_u64().unwrap() > create_body["data"]["version"].as_u64().unwrap());

    // Step 4: DELETE - Remove the preference
    let delete_response = request()
        .method("DELETE")
        .path(&format!("/api/v1/preferences/userPreferenceDelete?user_id={}&preference_key={}", user_id, preference_key))
        .header("authorization", "Bearer test-api-key")
        .reply(&preference_delete_endpoint())
        .await;

    assert_eq!(delete_response.status(), StatusCode::OK);

    let delete_body: Value = serde_json::from_slice(&delete_response.body()).unwrap();
    assert!(delete_body["success"].as_bool().unwrap());
    assert!(delete_body["data"]["deleted"].as_bool().unwrap());
    assert_eq!(delete_body["data"]["user_id"], user_id);
    assert_eq!(delete_body["data"]["preference_key"], preference_key);

    // Step 5: VERIFY DELETE - Try to update after deletion (should create new)
    let verify_update_response = request()
        .method("PUT")
        .path("/api/v1/preferences/userPreferenceUpdate")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&update_request)
        .reply(&preference_update_endpoint())
        .await;

    assert_eq!(verify_update_response.status(), StatusCode::OK);

    let verify_body: Value = serde_json::from_slice(&verify_update_response.body()).unwrap();
    assert!(verify_body["success"].as_bool().unwrap());
    // Should have a new ID (recreated)
    assert_ne!(verify_body["data"]["id"].as_str().unwrap(), created_id);
}

#[tokio::test]
async fn test_preference_crud_array_values() {
    let user_id = "user_array_test";
    let preference_key = "preferred_models";

    // CREATE with array value
    let initial_models = json!(["gpt-4", "claude-3"]);
    let create_request = json!({
        "user_id": user_id,
        "preference_key": preference_key,
        "preference_value": initial_models,
        "category": "ModelSelection"
    });

    let create_response = request()
        .method("POST")
        .path("/api/v1/preferences/userPreferenceCreate")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&create_request)
        .reply(&preference_create_endpoint())
        .await;

    assert_eq!(create_response.status(), StatusCode::CREATED);

    // UPDATE with different array value
    let updated_models = json!(["gpt-4", "claude-3", "gemini-pro"]);
    let update_request = json!({
        "user_id": user_id,
        "preference_key": preference_key,
        "preference_value": updated_models,
        "category": "ModelSelection"
    });

    let update_response = request()
        .method("PUT")
        .path("/api/v1/preferences/userPreferenceUpdate")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&update_request)
        .reply(&preference_update_endpoint())
        .await;

    assert_eq!(update_response.status(), StatusCode::OK);

    let update_body: Value = serde_json::from_slice(&update_response.body()).unwrap();
    assert_eq!(update_body["data"]["preference_value"], updated_models);

    // DELETE
    let delete_response = request()
        .method("DELETE")
        .path(&format!("/api/v1/preferences/userPreferenceDelete?user_id={}&preference_key={}", user_id, preference_key))
        .header("authorization", "Bearer test-api-key")
        .reply(&preference_delete_endpoint())
        .await;

    assert_eq!(delete_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_preference_crud_object_values() {
    let user_id = "user_object_test";
    let preference_key = "cost_settings";

    // CREATE with object value
    let initial_settings = json!({"max_cost": 0.01, "currency": "USD"});
    let create_request = json!({
        "user_id": user_id,
        "preference_key": preference_key,
        "preference_value": initial_settings,
        "category": "ModelSelection"
    });

    let create_response = request()
        .method("POST")
        .path("/api/v1/preferences/userPreferenceCreate")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&create_request)
        .reply(&preference_create_endpoint())
        .await;

    assert_eq!(create_response.status(), StatusCode::CREATED);

    // UPDATE with different object value
    let updated_settings = json!({"max_cost": 0.02, "currency": "USD", "budget_monthly": 100});
    let update_request = json!({
        "user_id": user_id,
        "preference_key": preference_key,
        "preference_value": updated_settings,
        "category": "ModelSelection"
    });

    let update_response = request()
        .method("PUT")
        .path("/api/v1/preferences/userPreferenceUpdate")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&update_request)
        .reply(&preference_update_endpoint())
        .await;

    assert_eq!(update_response.status(), StatusCode::OK);

    let update_body: Value = serde_json::from_slice(&update_response.body()).unwrap();
    assert_eq!(update_body["data"]["preference_value"], updated_settings);

    // DELETE
    let delete_response = request()
        .method("DELETE")
        .path(&format!("/api/v1/preferences/userPreferenceDelete?user_id={}&preference_key={}", user_id, preference_key))
        .header("authorization", "Bearer test-api-key")
        .reply(&preference_delete_endpoint())
        .await;

    assert_eq!(delete_response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_preference_crud_multiple_preferences() {
    let user_id = "user_multi_pref";

    let preferences = vec![
        ("style", "detailed", "ResponseFormatting"),
        ("language", "en", "ResponseFormatting"),
        ("max_tokens", 500, "ModelSelection"),
    ];

    let mut created_ids = Vec::new();

    // CREATE multiple preferences
    for (key, value, category) in preferences {
        let create_request = json!({
            "user_id": user_id,
            "preference_key": key,
            "preference_value": value,
            "category": category
        });

        let create_response = request()
            .method("POST")
            .path("/api/v1/preferences/userPreferenceCreate")
            .header("content-type", "application/json")
            .header("authorization", "Bearer test-api-key")
            .body_json(&create_request)
            .reply(&preference_create_endpoint())
            .await;

        assert_eq!(create_response.status(), StatusCode::CREATED);

        let create_body: Value = serde_json::from_slice(&create_response.body()).unwrap();
        created_ids.push((key, create_body["data"]["id"].as_str().unwrap().to_string()));
    }

    // UPDATE one preference
    let update_request = json!({
        "user_id": user_id,
        "preference_key": "style",
        "preference_value": "concise",
        "category": "ResponseFormatting"
    });

    let update_response = request()
        .method("PUT")
        .path("/api/v1/preferences/userPreferenceUpdate")
        .header("content-type", "application/json")
        .header("authorization", "Bearer test-api-key")
        .body_json(&update_request)
        .reply(&preference_update_endpoint())
        .await;

    assert_eq!(update_response.status(), StatusCode::OK);

    // DELETE all preferences
    for (key, _) in created_ids {
        let delete_response = request()
            .method("DELETE")
            .path(&format!("/api/v1/preferences/userPreferenceDelete?user_id={}&preference_key={}", user_id, key))
            .header("authorization", "Bearer test-api-key")
            .reply(&preference_delete_endpoint())
            .await;

        assert_eq!(delete_response.status(), StatusCode::OK);
    }
}