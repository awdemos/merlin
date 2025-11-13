// tests/priority1_features_test.rs
use merlin::api::{ModelSelectRequest, Message, Tradeoff};
use merlin::IntelligentModelSelector;

#[tokio::test]
async fn test_tradeoff_feature() {
    // Test that the tradeoff feature works without Redis dependency issues
    if let Ok(mut selector) = IntelligentModelSelector::new().await {
        let request = ModelSelectRequest {
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: "Explain quantum computing algorithms in detail".to_string(),
                }
            ],
            models: vec![
                "openai.gpt-4-turbo".to_string(),
                "anthropic.claude-3-opus-20240229".to_string(),
                "mistral.mistral-large-latest".to_string(),
            ],
            preferences: None,
            session_id: None,
            tradeoff: Some(Tradeoff::Quality), // NEW: Test tradeoff feature
            timeout: None,
            default_model: None,
        };

        let response = selector.select_model(request).await;
        
        // Print the error if there is one
        if let Err(ref e) = response {
            println!("Error in tradeoff test: {:?}", e);
        }
        
        // Should succeed without panicking
        assert!(response.is_ok(), "Model selection failed: {:?}", response);
        
        if let Ok(resp) = response {
            // Should have a session ID
            assert!(resp.session_id.is_some());
            
            // Should have reasoning that mentions tradeoff
            assert!(resp.reasoning.contains("Optimized for quality output"));
            
            println!("✅ Tradeoff test passed!");
            println!("   Recommended model: {}", resp.recommended_model);
            println!("   Reasoning: {}", resp.reasoning);
            println!("   Session ID: {}", resp.session_id.unwrap());
        }
    } else {
        println!("⚠️  Could not create selector (Redis not available)");
    }
}

#[tokio::test]
async fn test_timeout_and_fallback_features() {
    if let Ok(mut selector) = IntelligentModelSelector::new().await {
        let request = ModelSelectRequest {
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: "What's the weather like?".to_string(),
                }
            ],
            models: vec![
                "openai.gpt-4-turbo".to_string(),
                "anthropic.claude-3-opus-20240229".to_string(),
            ],
            preferences: None,
            session_id: None,
            tradeoff: Some(Tradeoff::Latency), // NEW: Optimize for speed
            timeout: Some(5), // NEW: 5 second timeout
            default_model: Some("anthropic.claude-3-opus-20240229".to_string()), // NEW: Fallback model
        };

        let response = selector.select_model(request).await;
        
        // Should succeed without panicking
        assert!(response.is_ok());
        
        if let Ok(resp) = response {
            // Should have a session ID
            assert!(resp.session_id.is_some());
            
            // Should have reasoning that mentions latency optimization
            assert!(resp.reasoning.contains("Optimized for low latency"));
            
            println!("✅ Timeout and fallback test passed!");
            println!("   Recommended model: {}", resp.recommended_model);
            println!("   Reasoning: {}", resp.reasoning);
            println!("   Session ID: {}", resp.session_id.unwrap());
        }
    } else {
        println!("⚠️  Could not create selector (Redis not available)");
    }
}

#[tokio::test]
async fn test_session_id_generation() {
    if let Ok(mut selector) = IntelligentModelSelector::new().await {
        let request1 = ModelSelectRequest {
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: "Test message 1".to_string(),
                }
            ],
            models: vec!["openai.gpt-4-turbo".to_string()],
            preferences: None,
            session_id: None, // Let system generate one
            tradeoff: None,
            timeout: None,
            default_model: None,
        };

        let request2 = ModelSelectRequest {
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: "Test message 2".to_string(),
                }
            ],
            models: vec!["openai.gpt-4-turbo".to_string()],
            preferences: None,
            session_id: Some("custom-session-123".to_string()), // Custom session ID
            tradeoff: None,
            timeout: None,
            default_model: None,
        };

        let response1 = selector.select_model(request1).await.unwrap();
        let response2 = selector.select_model(request2).await.unwrap();
        
        // First request should have generated session ID
        assert!(response1.session_id.is_some());
        let generated_id = response1.session_id.unwrap();
        assert!(!generated_id.is_empty());
        
        // Second request should use custom session ID
        assert_eq!(response2.session_id.clone().unwrap(), "custom-session-123");
        
        println!("✅ Session ID generation test passed!");
        println!("   Generated session ID: {}", generated_id);
        println!("   Custom session ID: {}", response2.session_id.clone().unwrap());
    } else {
        println!("⚠️  Could not create selector (Redis not available)");
    }
}