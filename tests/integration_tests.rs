use merlin::*;
use tokio;

#[tokio::test]
async fn test_ollama_provider() {
    let provider = OllamaProvider::new("http://localhost:11434".to_string(), "llama2".to_string());

    // Test provider name
    assert_eq!(provider.name(), "Ollama");

    // Note: This test would require Ollama to be running
    // Uncomment if you have Ollama running locally:
    // let response = provider.chat("Hello, how are you?").await;
    // assert!(response.is_ok());
}

#[tokio::test]
async fn test_routing_policy_selection() {
    let epsilon_policy = RoutingPolicy::EpsilonGreedy { epsilon: 0.1 };

    // Test that it selects valid indices
    for _ in 0..100 {
        let index = epsilon_policy.select_index(5);
        assert!(index < 5);
    }
}

#[tokio::test]
async fn test_multiple_providers() {
    let ollama_provider =
        OllamaProvider::new("http://localhost:11434".to_string(), "llama2".to_string());

    assert_eq!(ollama_provider.name(), "Ollama");

    // Test would expand with OpenAI provider when API keys are available
}
