// tests/debug_models_test.rs
use merlin::IntelligentModelSelector;

#[tokio::test]
async fn debug_available_models() {
    if let Ok(mut selector) = IntelligentModelSelector::new().await {
        // Try to get available models by checking what doesn't error
        let test_models = vec![
            "claude-3-opus-20240229",
            "claude-3-sonnet-20240229", 
            "gpt-4-turbo-preview",
            "gpt-3.5-turbo",
            "llama-3-70b",
            "mixtral-8x7b",
        ];
        
        for model in test_models {
            let request = merlin::api::ModelSelectRequest {
                messages: vec![merlin::api::Message {
                    role: "user".to_string(),
                    content: "test".to_string(),
                }],
                models: vec![model.to_string()],
                preferences: None,
                session_id: None,
                tradeoff: None,
                timeout: None,
                default_model: None,
            };
            
            match selector.select_model(request).await {
                Ok(response) => println!("✅ Model '{}' works: {}", model, response.recommended_model),
                Err(e) => println!("❌ Model '{}' failed: {}", model, e),
            }
        }
    } else {
        println!("❌ Could not create selector");
    }
}