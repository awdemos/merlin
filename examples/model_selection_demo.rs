// examples/model_selection_demo.rs
use merlin::api::{ModelSelectRequest, Message, UserPreferences, OptimizationTarget};
use merlin::IntelligentModelSelector;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧙 Merlin Model Selection Demo\n");
    
    // Initialize the intelligent model selector
    let mut selector = match IntelligentModelSelector::new().await {
        Ok(selector) => {
            println!("✅ Model selector initialized successfully");
            selector
        },
        Err(e) => {
            println!("❌ Failed to initialize model selector: {}", e);
            println!("💡 Make sure Redis is running for full functionality");
            return Err(e);
        }
    };

    // Demo 1: Technical Query - Should prefer GPT-4
    println!("\n🔍 Demo 1: Technical Analysis Query");
    let tech_request = ModelSelectRequest {
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "Analyze the time complexity of quicksort vs mergesort algorithms and explain when to use each".to_string(),
            }
        ],
        models: vec![
            "gpt-4".to_string(),
            "claude-3".to_string(),
            "llama-3.1".to_string(),
        ],
        preferences: Some(UserPreferences {
            optimize_for: Some(OptimizationTarget::Quality),
            max_tokens: Some(1500),
            user_id: None,
            temperature: None,
            custom_weights: None,
        }),
        session_id: None,
    };

    match selector.select_model(tech_request).await {
        Ok(response) => {
            println!("🎯 Recommended Model: {}", response.recommended_model);
            println!("🎲 Confidence: {:.2}", response.confidence);
            println!("🧠 Reasoning: {}", response.reasoning);
            println!("💰 Estimated Cost: ${:.4}", response.estimated_cost.unwrap_or(0.0));
            println!("⏱️  Estimated Latency: {}ms", response.estimated_latency_ms.unwrap_or(0));
            
            println!("📊 Alternatives:");
            for alt in &response.alternatives {
                println!("   • {}: confidence {:.2}, cost ${:.4}", 
                        alt.model, alt.confidence, alt.estimated_cost.unwrap_or(0.0));
            }
        },
        Err(e) => println!("❌ Error: {}", e),
    }

    // Demo 2: Simple Query - Should prefer cost-effective option
    println!("\n🔍 Demo 2: Simple Query (Cost Optimized)");
    let simple_request = ModelSelectRequest {
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "What's the capital of France?".to_string(),
            }
        ],
        models: vec![
            "gpt-4".to_string(),
            "claude-3".to_string(),
            "llama-3.1".to_string(),
        ],
        preferences: Some(UserPreferences {
            optimize_for: Some(OptimizationTarget::Cost),
            max_tokens: Some(50),
            user_id: None,
            temperature: None,
            custom_weights: None,
        }),
        session_id: None,
    };

    match selector.select_model(simple_request).await {
        Ok(response) => {
            println!("🎯 Recommended Model: {}", response.recommended_model);
            println!("🎲 Confidence: {:.2}", response.confidence);
            println!("🧠 Reasoning: {}", response.reasoning);
            println!("💰 Estimated Cost: ${:.4}", response.estimated_cost.unwrap_or(0.0));
            println!("⏱️  Estimated Latency: {}ms", response.estimated_latency_ms.unwrap_or(0));
        },
        Err(e) => println!("❌ Error: {}", e),
    }

    // Demo 3: Creative Task - Should prefer Claude
    println!("\n🔍 Demo 3: Creative Writing Task");
    let creative_request = ModelSelectRequest {
        messages: vec![
            Message {
                role: "user".to_string(),
                content: "Write a short creative story about a robot who discovers emotions".to_string(),
            }
        ],
        models: vec![
            "gpt-4".to_string(),
            "claude-3".to_string(),
            "llama-3.1".to_string(),
        ],
        preferences: Some(UserPreferences {
            optimize_for: Some(OptimizationTarget::Balanced),
            max_tokens: Some(800),
            user_id: None,
            temperature: None,
            custom_weights: None,
        }),
        session_id: None,
    };

    match selector.select_model(creative_request).await {
        Ok(response) => {
            println!("🎯 Recommended Model: {}", response.recommended_model);
            println!("🎲 Confidence: {:.2}", response.confidence);
            println!("🧠 Reasoning: {}", response.reasoning);
            println!("💰 Estimated Cost: ${:.4}", response.estimated_cost.unwrap_or(0.0));
            println!("⏱️  Estimated Latency: {}ms", response.estimated_latency_ms.unwrap_or(0));
        },
        Err(e) => println!("❌ Error: {}", e),
    }

    println!("\n🎉 Demo completed! Merlin's intelligent routing is working.");
    println!("💡 Next steps: Try the /modelSelect API endpoint or integrate with your application.");

    Ok(())
}
