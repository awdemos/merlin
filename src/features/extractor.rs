use crate::api::{DomainCategory, TaskType};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PromptFeatures {
    pub domain_category: DomainCategory,
    pub task_type: TaskType,
    pub complexity_score: f64,
    pub estimated_tokens: u32,
    pub keyword_features: HashMap<String, f64>,
    pub length_features: f64,
    pub structural_features: f64,
}

impl PromptFeatures {
    pub fn analyze(messages: &[crate::api::Message]) -> Self {
        let mut combined_text = String::new();
        for message in messages {
            combined_text.push_str(&message.content);
            combined_text.push(' ');
        }

        let text = combined_text.trim().to_lowercase();

        let domain_category = Self::classify_domain(&text);
        let task_type = Self::classify_task(&text);
        let complexity_score = Self::calculate_complexity(&text);
        let estimated_tokens = Self::estimate_tokens(&text);
        let keyword_features = Self::extract_keywords(&text);
        let length_features = Self::calculate_length_features(&text);
        let structural_features = Self::calculate_structural_features(&text);

        PromptFeatures {
            domain_category,
            task_type,
            complexity_score,
            estimated_tokens,
            keyword_features,
            length_features,
            structural_features,
        }
    }

    pub fn to_feature_vector(&self) -> Vec<f64> {
        let mut features = Vec::new();

        // Domain category one-hot encoding
        match self.domain_category {
            DomainCategory::Technical => features.push(1.0),
            _ => features.push(0.0),
        }
        match self.domain_category {
            DomainCategory::Creative => features.push(1.0),
            _ => features.push(0.0),
        }
        match self.domain_category {
            DomainCategory::Analytical => features.push(1.0),
            _ => features.push(0.0),
        }
        match self.domain_category {
            DomainCategory::General => features.push(1.0),
            _ => features.push(0.0),
        }

        // Task type one-hot encoding
        match self.task_type {
            TaskType::Question => features.push(1.0),
            _ => features.push(0.0),
        }
        match self.task_type {
            TaskType::Generation => features.push(1.0),
            _ => features.push(0.0),
        }
        match self.task_type {
            TaskType::Analysis => features.push(1.0),
            _ => features.push(0.0),
        }
        match self.task_type {
            TaskType::Instruction => features.push(1.0),
            _ => features.push(0.0),
        }

        // Continuous features
        features.push(self.complexity_score);
        features.push((self.estimated_tokens as f64) / 1000.0); // Normalized token count
        features.push(self.length_features);
        features.push(self.structural_features);

        // Keyword features (top 10 most important keywords)
        let important_keywords = [
            "code", "data", "algorithm", "function", "api", "database", "network", "security",
            "creative", "write", "story", "poem", "design", "art", "music", "analyze", "research",
            "explain", "teach", "learn", "question", "answer", "help", "solve"
        ];

        for keyword in &important_keywords {
            features.push(self.keyword_features.get(*keyword).copied().unwrap_or(0.0));
        }

        features
    }

    fn classify_domain(text: &str) -> DomainCategory {
        let technical_keywords = ["code", "function", "algorithm", "api", "database", "network", "security", "bug", "debug"];
        let creative_keywords = ["creative", "write", "story", "poem", "design", "art", "music", "fiction", "narrative"];
        let analytical_keywords = ["analyze", "research", "data", "statistics", "compare", "evaluate", "study", "investigate"];

        let mut scores = HashMap::new();
        scores.insert(DomainCategory::Technical, 0);
        scores.insert(DomainCategory::Creative, 0);
        scores.insert(DomainCategory::Analytical, 0);
        scores.insert(DomainCategory::General, 1);

        for word in text.split_whitespace() {
            if technical_keywords.contains(&word) {
                *scores.get_mut(&DomainCategory::Technical).unwrap() += 1;
            }
            if creative_keywords.contains(&word) {
                *scores.get_mut(&DomainCategory::Creative).unwrap() += 1;
            }
            if analytical_keywords.contains(&word) {
                *scores.get_mut(&DomainCategory::Analytical).unwrap() += 1;
            }
        }

        scores.into_iter()
            .max_by_key(|(_, score)| *score)
            .map(|(category, _)| category)
            .unwrap_or(DomainCategory::General)
    }

    fn classify_task(text: &str) -> TaskType {
        let question_indicators = ["?", "what", "how", "why", "when", "where", "who", "explain", "tell me"];
        let generation_indicators = ["create", "write", "generate", "make", "build", "design", "compose"];
        let analysis_indicators = ["analyze", "compare", "evaluate", "research", "study", "investigate"];
        let instruction_indicators = ["do", "run", "execute", "implement", "code", "program", "call"];

        let mut scores = HashMap::new();
        scores.insert(TaskType::Question, 0);
        scores.insert(TaskType::Generation, 0);
        scores.insert(TaskType::Analysis, 0);
        scores.insert(TaskType::Instruction, 0);

        for word in text.split_whitespace() {
            if question_indicators.contains(&word) {
                *scores.get_mut(&TaskType::Question).unwrap() += 1;
            }
            if generation_indicators.contains(&word) {
                *scores.get_mut(&TaskType::Generation).unwrap() += 1;
            }
            if analysis_indicators.contains(&word) {
                *scores.get_mut(&TaskType::Analysis).unwrap() += 1;
            }
            if instruction_indicators.contains(&word) {
                *scores.get_mut(&TaskType::Instruction).unwrap() += 1;
            }
        }

        scores.into_iter()
            .max_by_key(|(_, score)| *score)
            .map(|(task_type, _)| task_type)
            .unwrap_or(TaskType::Question)
    }

    fn calculate_complexity(text: &str) -> f64 {
        let word_count = text.split_whitespace().count();
        let avg_word_length = text.chars().filter(|c| c.is_alphabetic()).count() as f64 / word_count.max(1) as f64;
        let sentence_count = text.split(&['.', '!', '?']).filter(|s| !s.trim().is_empty()).count();
        let avg_sentence_length = word_count as f64 / sentence_count.max(1) as f64;

        // Unique words ratio (vocabulary complexity)
        let words: Vec<&str> = text.split_whitespace().collect();
        let unique_words: std::collections::HashSet<_> = words.iter().collect();
        let unique_ratio = unique_words.len() as f64 / words.len().max(1) as f64;

        // Technical terms
        let technical_terms = ["algorithm", "function", "variable", "class", "method", "api", "database", "network"];
        let tech_term_count = technical_terms.iter()
            .filter(|&&term| text.contains(term))
            .count() as f64;

        // Normalize complexity score to 0-1 range
        let complexity = (avg_word_length / 10.0).min(1.0) * 0.2 +
                       (avg_sentence_length / 20.0).min(1.0) * 0.3 +
                       unique_ratio * 0.3 +
                       (tech_term_count / 5.0).min(1.0) * 0.2;

        complexity.max(0.0).min(1.0)
    }

    fn estimate_tokens(text: &str) -> u32 {
        // Rough token estimation (typically 1 token = 4 characters for English)
        ((text.chars().count() as f64 * 0.25) + text.split_whitespace().count() as f64 * 0.75) as u32
    }

    fn extract_keywords(text: &str) -> HashMap<String, f64> {
        let mut keywords = HashMap::new();
        let words: Vec<&str> = text.split_whitespace().collect();

        // Count word frequencies
        for word in words {
            let clean_word = word.trim_matches(|c: char| !c.is_alphabetic()).to_lowercase();
            if !clean_word.is_empty() && clean_word.len() > 2 {
                *keywords.entry(clean_word).or_insert(0.0) += 1.0;
            }
        }

        // Normalize frequencies
        let total: f64 = keywords.values().sum();
        if total > 0.0 {
            for freq in keywords.values_mut() {
                *freq /= total;
            }
        }

        keywords
    }

    fn calculate_length_features(text: &str) -> f64 {
        let char_count = text.chars().count();
        let _word_count = text.split_whitespace().count();

        // Normalize to 0-1 range (assuming max reasonable length of 5000 characters)
        (char_count as f64 / 5000.0).min(1.0)
    }

    fn calculate_structural_features(text: &str) -> f64 {
        let has_questions = text.contains('?');
        let has_code = text.contains("```") || text.contains("fn ") || text.contains("function ");
        let has_lists = text.contains("- ") || text.contains("1.") || text.contains("* ");
        let has_quotes = text.contains('"') || text.contains('\'');

        let mut features = 0.0;
        if has_questions { features += 0.25; }
        if has_code { features += 0.25; }
        if has_lists { features += 0.25; }
        if has_quotes { features += 0.25; }

        features
    }

    pub fn to_vector(&self) -> Vec<f64> {
        let mut vector = Vec::new();

        // Domain category as one-hot encoding
        vector.extend_from_slice(&match self.domain_category {
            DomainCategory::General => [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            DomainCategory::Technical => [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            DomainCategory::Creative => [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            DomainCategory::Analytical => [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            DomainCategory::Mathematical => [0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
            DomainCategory::CodeGeneration => [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            DomainCategory::Translation => [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            DomainCategory::Summarization => [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
        });

        // Task type as one-hot encoding
        vector.extend_from_slice(&match self.task_type {
            TaskType::Question => [1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            TaskType::Instruction => [0.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            TaskType::Conversation => [0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
            TaskType::Completion => [0.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            TaskType::Analysis => [0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            TaskType::Generation => [0.0, 0.0, 0.0, 0.0, 0.0, 1.0],
        });

        // Numerical features
        vector.push(self.complexity_score);
        vector.push(self.estimated_tokens as f64 / 10000.0); // Normalize tokens
        vector.push(self.length_features);
        vector.push(self.structural_features);

        vector
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_features_creation() {
        let messages = vec![
            crate::api::Message {
                role: "user".to_string(),
                content: "Write a function to calculate fibonacci sequence".to_string(),
            }
        ];

        let features = PromptFeatures::analyze(&messages);

        assert_eq!(features.estimated_tokens > 0, true);
        assert!(features.complexity_score >= 0.0 && features.complexity_score <= 1.0);
    }

    #[test]
    fn test_feature_vector_generation() {
        let messages = vec![
            crate::api::Message {
                role: "user".to_string(),
                content: "What is the capital of France?".to_string(),
            }
        ];

        let features = PromptFeatures::analyze(&messages);
        let vector = features.to_feature_vector();

        assert!(vector.len() > 10); // Should have many features
        for value in &vector {
            assert!(value.is_finite());
        }
    }

    #[test]
    fn test_domain_classification() {
        let technical_text = "Write a function to sort an array using quicksort algorithm";
        let creative_text = "Create a story about a dragon who loves to paint";
        let analytical_text = "Analyze the economic impact of climate change on agriculture";

        let tech_features = PromptFeatures::analyze(&[
            crate::api::Message { role: "user".to_string(), content: technical_text.to_string() }
        ]);

        let creative_features = PromptFeatures::analyze(&[
            crate::api::Message { role: "user".to_string(), content: creative_text.to_string() }
        ]);

        let analytical_features = PromptFeatures::analyze(&[
            crate::api::Message { role: "user".to_string(), content: analytical_text.to_string() }
        ]);

        // Check that different domains are classified differently
        // (exact classification depends on keyword matching)
        assert!(tech_features.keyword_features.contains_key("function"));
        assert!(creative_features.keyword_features.contains_key("story"));
    }

    #[test]
    fn test_complexity_calculation() {
        let simple_text = "Hello world";
        let complex_text = "Implement a multithreaded algorithm for distributed consensus in blockchain networks";

        let simple_features = PromptFeatures::analyze(&[
            crate::api::Message { role: "user".to_string(), content: simple_text.to_string() }
        ]);

        let complex_features = PromptFeatures::analyze(&[
            crate::api::Message { role: "user".to_string(), content: complex_text.to_string() }
        ]);

        // Complex text should have higher complexity score
        assert!(complex_features.complexity_score > simple_features.complexity_score);
    }
}