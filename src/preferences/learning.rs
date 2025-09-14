use crate::preferences::models::*;
use crate::features::PromptFeatures;
use std::collections::HashMap;

pub struct PreferenceLearner {
    learning_rate: f64,
    confidence_threshold: f64,
    max_history_size: usize,
}

impl PreferenceLearner {
    pub fn new() -> Self {
        Self {
            learning_rate: 0.1,
            confidence_threshold: 0.7,
            max_history_size: 100,
        }
    }

    pub fn learn_from_session(&self, preferences: &mut UserPreferences, session_interactions: &[UserInteraction]) {
        if !preferences.learning_enabled || session_interactions.is_empty() {
            return;
        }

        // Analyze session patterns
        let session_analysis = self.analyze_session_patterns(session_interactions);

        // Update model weights based on session performance
        self.update_model_weights(preferences, &session_analysis);

        // Learn optimization target preferences
        self.learn_optimization_targets(preferences, &session_analysis);

        // Update model preferences based on success patterns
        self.update_model_preferences(preferences, &session_analysis);
    }

    fn analyze_session_patterns(&self, interactions: &[UserInteraction]) -> SessionAnalysis {
        let mut model_performance: HashMap<String, ModelPerformance> = HashMap::new();
        let mut domain_patterns: HashMap<String, DomainStats> = HashMap::new();
        let mut complexity_distribution = Vec::new();
        let mut overall_satisfaction = 0.0;
        let mut total_cost = 0.0;
        let mut total_time = 0;

        for interaction in interactions {
            // Track model performance
            let performance = model_performance.entry(interaction.model_used.clone())
                .or_insert(ModelPerformance::new());

            if let Some(rating) = interaction.rating {
                performance.add_rating(rating);
                overall_satisfaction += rating as f64;
            }

            if let Some(time) = interaction.response_time_ms {
                performance.add_response_time(time);
                total_time += time;
            }

            if let Some(cost) = interaction.cost {
                performance.add_cost(cost);
                total_cost += cost;
            }

            // Track domain patterns
            let domain_stats = domain_patterns.entry(interaction.prompt_features.domain_category.clone())
                .or_insert(DomainStats::new());

            if let Some(rating) = interaction.rating {
                domain_stats.add_rating(rating);
            }

            // Track complexity distribution
            complexity_distribution.push(interaction.prompt_features.complexity_score);
        }

        // Calculate averages
        let avg_satisfaction = if !interactions.is_empty() {
            overall_satisfaction / interactions.len() as f64
        } else {
            0.0
        };

        let avg_cost_per_request = if !interactions.is_empty() {
            total_cost / interactions.len() as f64
        } else {
            0.0
        };

        let avg_time_per_request = if !interactions.is_empty() {
            total_time as f64 / interactions.len() as f64
        } else {
            0.0
        };

        // Find best and worst performing models
        let mut model_rankings: Vec<_> = model_performance.iter()
            .map(|(model, perf)| (model.clone(), perf.calculate_overall_score()))
            .collect();

        model_rankings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let best_model = model_rankings.first().map(|(model, _)| model.clone());
        let worst_model = model_rankings.last().map(|(model, _)| model.clone());

        SessionAnalysis {
            model_performance,
            domain_patterns,
            complexity_distribution,
            overall_satisfaction: avg_satisfaction,
            avg_cost_per_request,
            avg_time_per_request,
            best_model,
            worst_model,
            total_interactions: interactions.len(),
        }
    }

    fn update_model_weights(&self, preferences: &mut UserPreferences, analysis: &SessionAnalysis) {
        for (model_name, performance) in &analysis.model_performance {
            let current_weight = preferences.custom_weights.get(model_name).copied().unwrap_or(1.0);
            let performance_score = performance.calculate_overall_score();

            // Calculate weight adjustment
            let weight_adjustment = if performance_score > 0.7 {
                // High performance - increase weight
                self.learning_rate * (performance_score - 0.5) * 2.0
            } else if performance_score < 0.3 {
                // Low performance - decrease weight
                -self.learning_rate * (0.5 - performance_score) * 2.0
            } else {
                // Neutral performance - minimal adjustment
                0.0
            };

            let new_weight = (current_weight + weight_adjustment as f32).max(0.1).min(3.0);
            preferences.custom_weights.insert(model_name.clone(), new_weight as f32);
        }
    }

    fn learn_optimization_targets(&self, preferences: &mut UserPreferences, analysis: &SessionAnalysis) {
        let mut target_scores = HashMap::new();

        // Score each optimization target based on session performance
        target_scores.insert(OptimizationTarget::Quality, self.score_quality_optimization(analysis));
        target_scores.insert(OptimizationTarget::Speed, self.score_speed_optimization(analysis));
        target_scores.insert(OptimizationTarget::Cost, self.score_cost_optimization(analysis));
        target_scores.insert(OptimizationTarget::Balanced, self.score_balanced_optimization(analysis));

        // Find the best optimization target
        let best_target = target_scores.into_iter()
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(target, _)| target)
            .unwrap_or(OptimizationTarget::Balanced);

        // Only change if significantly better and above confidence threshold
        let current_target_score = match preferences.optimize_for {
            OptimizationTarget::Quality => self.score_quality_optimization(analysis),
            OptimizationTarget::Speed => self.score_speed_optimization(analysis),
            OptimizationTarget::Cost => self.score_cost_optimization(analysis),
            OptimizationTarget::Balanced => self.score_balanced_optimization(analysis),
        };

        let best_target_score = match best_target {
            OptimizationTarget::Quality => self.score_quality_optimization(analysis),
            OptimizationTarget::Speed => self.score_speed_optimization(analysis),
            OptimizationTarget::Cost => self.score_cost_optimization(analysis),
            OptimizationTarget::Balanced => self.score_balanced_optimization(analysis),
        };

        if best_target_score > current_target_score + 0.1 && best_target_score > self.confidence_threshold {
            preferences.optimize_for = best_target;
        }
    }

    fn update_model_preferences(&self, preferences: &mut UserPreferences, analysis: &SessionAnalysis) {
        // Update preferred models based on high performance
        if let Some(ref best_model) = analysis.best_model {
            if let Some(performance) = analysis.model_performance.get(best_model) {
                if performance.calculate_overall_score() > 0.7 {
                    if !preferences.preferred_models.contains(best_model) {
                        preferences.preferred_models.push(best_model.clone());
                    }
                }
            }
        }

        // Update excluded models based on poor performance
        if let Some(ref worst_model) = analysis.worst_model {
            if let Some(performance) = analysis.model_performance.get(worst_model) {
                if performance.calculate_overall_score() < 0.3 {
                    if !preferences.excluded_models.contains(worst_model) {
                        preferences.excluded_models.push(worst_model.clone());
                    }
                }
            }
        }

        // Remove models from excluded lists if they perform well
        for model in &preferences.excluded_models.clone() {
            if let Some(performance) = analysis.model_performance.get(model) {
                if performance.calculate_overall_score() > 0.6 {
                    preferences.excluded_models.retain(|m| m != model);
                }
            }
        }
    }

    fn score_quality_optimization(&self, analysis: &SessionAnalysis) -> f64 {
        let mut score: f64 = 0.0;

        // High ratings contribute to quality score
        score += analysis.overall_satisfaction / 5.0 * 0.4;

        // Low response time doesn't hurt quality much
        if analysis.avg_time_per_request < 2000.0 {
            score += 0.2;
        }

        // Higher cost is acceptable for quality
        if analysis.avg_cost_per_request > 0.02 {
            score += 0.2;
        }

        // Complex tasks benefit from quality optimization
        let avg_complexity = analysis.complexity_distribution.iter().sum::<f64>() /
                            analysis.complexity_distribution.len().max(1) as f64;
        if avg_complexity > 0.6 {
            score += 0.2;
        }

        return score.min(1.0);
    }

    fn score_speed_optimization(&self, analysis: &SessionAnalysis) -> f64 {
        let mut score: f64 = 0.0;

        // Fast response times are crucial for speed optimization
        if analysis.avg_time_per_request < 1500.0 {
            score += 0.5;
        } else if analysis.avg_time_per_request < 2500.0 {
            score += 0.3;
        }

        // Lower cost is good for speed (cheaper models are often faster)
        if analysis.avg_cost_per_request < 0.015 {
            score += 0.2;
        }

        // Simple tasks benefit from speed optimization
        let avg_complexity = analysis.complexity_distribution.iter().sum::<f64>() /
                            analysis.complexity_distribution.len().max(1) as f64;
        if avg_complexity < 0.4 {
            score += 0.3;
        }

        return score.min(1.0);
    }

    fn score_cost_optimization(&self, analysis: &SessionAnalysis) -> f64 {
        let mut score: f64 = 0.0;

        // Low cost is the primary factor
        if analysis.avg_cost_per_request < 0.01 {
            score += 0.6;
        } else if analysis.avg_cost_per_request < 0.02 {
            score += 0.3;
        }

        // Good ratings help even with low cost
        score += analysis.overall_satisfaction / 5.0 * 0.2;

        // Reasonable response times expected
        if analysis.avg_time_per_request < 3000.0 {
            score += 0.2;
        }

        return score.min(1.0);
    }

    fn score_balanced_optimization(&self, analysis: &SessionAnalysis) -> f64 {
        let mut score: f64 = 0.0;

        // Balanced optimization cares about all factors
        score += analysis.overall_satisfaction / 5.0 * 0.3;

        // Reasonable cost (not too high, not too low)
        if analysis.avg_cost_per_request >= 0.01 && analysis.avg_cost_per_request <= 0.03 {
            score += 0.3;
        }

        // Reasonable response time
        if analysis.avg_time_per_request >= 1000.0 && analysis.avg_time_per_request <= 2500.0 {
            score += 0.3;
        }

        // Good across different complexity levels
        let complexity_variance = analysis.complexity_distribution.iter()
            .map(|&x| (x - analysis.complexity_distribution.iter().sum::<f64>() /
                     analysis.complexity_distribution.len().max(1) as f64).powi(2))
            .sum::<f64>() / analysis.complexity_distribution.len().max(1) as f64;

        if complexity_variance < 0.1 { // Consistent complexity handling
            score += 0.1;
        }

        return score.min(1.0);
    }

    pub fn personalize_model_selection(
        &self,
        preferences: &UserPreferences,
        prompt_features: &PromptFeatures,
        available_models: &[String],
    ) -> Vec<(String, f64)> {
        let mut model_scores = Vec::new();

        let context_features = prompt_features.to_feature_vector();

        for model_name in available_models {
            let base_score = preferences.calculate_model_preference_score(model_name, &context_features);

            // Apply additional personalization based on prompt characteristics
            let personalized_score = self.apply_prompt_personalization(
                preferences,
                model_name,
                prompt_features,
                base_score,
            );

            model_scores.push((model_name.clone(), personalized_score));
        }

        model_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        model_scores
    }

    fn apply_prompt_personalization(
        &self,
        preferences: &UserPreferences,
        model_name: &str,
        prompt_features: &PromptFeatures,
        base_score: f64,
    ) -> f64 {
        let mut score = base_score;

        // Personalize based on user's historical success with similar prompts
        if let Some(similar_performance) = self.find_similar_prompt_performance(preferences, prompt_features, model_name) {
            score *= 1.0 + similar_performance * 0.2;
        }

        // Adjust for user's complexity preferences
        if prompt_features.complexity_score > 0.7 {
            // High complexity prompts
            if preferences.optimize_for == OptimizationTarget::Quality {
                score *= 1.1;
            } else if preferences.optimize_for == OptimizationTarget::Cost {
                score *= 0.9;
            }
        } else if prompt_features.complexity_score < 0.3 {
            // Low complexity prompts
            if preferences.optimize_for == OptimizationTarget::Speed {
                score *= 1.1;
            } else if preferences.optimize_for == OptimizationTarget::Quality {
                score *= 0.95;
            }
        }

        // Personalize based on domain expertise
        let domain_expertise = self.calculate_domain_expertise(preferences, &prompt_features.domain_category.to_string());
        score *= 1.0 + domain_expertise * 0.1;

        score
    }

    fn find_similar_prompt_performance(
        &self,
        preferences: &UserPreferences,
        prompt_features: &PromptFeatures,
        model_name: &str,
    ) -> Option<f64> {
        let mut similar_scores = Vec::new();

        for interaction in &preferences.interaction_history {
            if interaction.model_used == model_name && interaction.rating.is_some() {
                // Calculate similarity based on domain, task type, and complexity
                let domain_similarity = if interaction.prompt_features.domain_category == prompt_features.domain_category.to_string() {
                    1.0
                } else {
                    0.0
                };

                let task_similarity = if interaction.prompt_features.task_type == prompt_features.task_type.to_string() {
                    1.0
                } else {
                    0.0
                };

                let complexity_similarity = 1.0 - (interaction.prompt_features.complexity_score - prompt_features.complexity_score).abs();

                let overall_similarity = (domain_similarity + task_similarity + complexity_similarity) / 3.0;

                if overall_similarity > 0.7 {
                    let rating = interaction.rating.unwrap() as f64 / 5.0;
                    similar_scores.push((overall_similarity, rating));
                }
            }
        }

        if similar_scores.is_empty() {
            return None;
        }

        // Weight by similarity
        let weighted_sum: f64 = similar_scores.iter()
            .map(|(similarity, rating)| similarity * rating)
            .sum();

        let total_similarity: f64 = similar_scores.iter()
            .map(|(similarity, _)| similarity)
            .sum();

        Some(weighted_sum / total_similarity)
    }

    fn calculate_domain_expertise(&self, preferences: &UserPreferences, domain: &str) -> f64 {
        let mut domain_interactions = 0;
        let mut domain_successes = 0;

        for interaction in &preferences.interaction_history {
            if interaction.prompt_features.domain_category == domain {
                domain_interactions += 1;
                if let Some(rating) = interaction.rating {
                    if rating >= 4 {
                        domain_successes += 1;
                    }
                }
            }
        }

        if domain_interactions == 0 {
            0.5 // Neutral when no experience
        } else {
            domain_successes as f64 / domain_interactions as f64
        }
    }
}

#[derive(Debug)]
struct SessionAnalysis {
    model_performance: HashMap<String, ModelPerformance>,
    domain_patterns: HashMap<String, DomainStats>,
    complexity_distribution: Vec<f64>,
    overall_satisfaction: f64,
    avg_cost_per_request: f64,
    avg_time_per_request: f64,
    best_model: Option<String>,
    worst_model: Option<String>,
    total_interactions: usize,
}

#[derive(Debug, Default)]
struct ModelPerformance {
    ratings: Vec<u8>,
    response_times: Vec<u32>,
    costs: Vec<f64>,
}

impl ModelPerformance {
    fn new() -> Self {
        Self {
            ratings: Vec::new(),
            response_times: Vec::new(),
            costs: Vec::new(),
        }
    }

    fn add_rating(&mut self, rating: u8) {
        self.ratings.push(rating);
    }

    fn add_response_time(&mut self, time: u32) {
        self.response_times.push(time);
    }

    fn add_cost(&mut self, cost: f64) {
        self.costs.push(cost);
    }

    fn calculate_overall_score(&self) -> f64 {
        let mut score: f64 = 0.0;

        // Rating score (40% weight)
        if !self.ratings.is_empty() {
            let avg_rating = self.ratings.iter().sum::<u8>() as f64 / self.ratings.len() as f64;
            score += avg_rating / 5.0 * 0.4;
        }

        // Response time score (30% weight)
        if !self.response_times.is_empty() {
            let avg_time = self.response_times.iter().sum::<u32>() as f64 / self.response_times.len() as f64;
            let time_score = if avg_time < 1500.0 { 1.0 }
                           else if avg_time < 3000.0 { 0.7 }
                           else if avg_time < 5000.0 { 0.4 }
                           else { 0.1 };
            score += time_score * 0.3;
        }

        // Cost score (30% weight)
        if !self.costs.is_empty() {
            let avg_cost = self.costs.iter().sum::<f64>() / self.costs.len() as f64;
            let cost_score = if avg_cost < 0.01 { 1.0 }
                          else if avg_cost < 0.02 { 0.7 }
                          else if avg_cost < 0.03 { 0.4 }
                          else { 0.1 };
            score += cost_score * 0.3;
        }

        score
    }
}

#[derive(Debug, Default)]
struct DomainStats {
    ratings: Vec<u8>,
}

impl DomainStats {
    fn new() -> Self {
        Self {
            ratings: Vec::new(),
        }
    }

    fn add_rating(&mut self, rating: u8) {
        self.ratings.push(rating);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preference_learner_creation() {
        let learner = PreferenceLearner::new();
        assert_eq!(learner.learning_rate, 0.1);
        assert_eq!(learner.confidence_threshold, 0.7);
    }

    #[test]
    fn test_model_performance_scoring() {
        let mut performance = ModelPerformance::new();

        // Add excellent performance
        performance.add_rating(5);
        performance.add_rating(4);
        performance.add_response_time(1200);
        performance.add_cost(0.02);

        let score = performance.calculate_overall_score();
        assert!(score > 0.7);

        // Add poor performance
        let mut poor_performance = ModelPerformance::new();
        poor_performance.add_rating(2);
        poor_performance.add_rating(1);
        poor_performance.add_response_time(5000);
        poor_performance.add_cost(0.05);

        let poor_score = poor_performance.calculate_overall_score();
        assert!(poor_score < 0.3);
    }

    #[test]
    fn test_optimization_target_scoring() {
        let learner = PreferenceLearner::new();

        let analysis = SessionAnalysis {
            model_performance: HashMap::new(),
            domain_patterns: HashMap::new(),
            complexity_distribution: vec![0.8, 0.7, 0.9], // High complexity
            overall_satisfaction: 4.5,
            avg_cost_per_request: 0.025, // Higher cost
            avg_time_per_request: 2800.0, // Slower
            best_model: None,
            worst_model: None,
            total_interactions: 3,
        };

        let quality_score = learner.score_quality_optimization(&analysis);
        let speed_score = learner.score_speed_optimization(&analysis);
        let cost_score = learner.score_cost_optimization(&analysis);

        // High complexity, high satisfaction should favor quality
        assert!(quality_score > speed_score);
        assert!(quality_score > cost_score);
    }

    #[test]
    fn test_personalized_model_scoring() {
        let learner = PreferenceLearner::new();
        let mut preferences = UserPreferences::new("test_user".to_string());

        preferences.custom_weights.insert("gpt-4".to_string(), 1.5);
        preferences.preferred_models.push("claude-3".to_string());
        preferences.excluded_models.push("llama-3.1".to_string());

        let prompt_features = PromptFeatures {
            domain_category: crate::api::DomainCategory::Technical,
            task_type: crate::api::TaskType::Analysis,
            complexity_score: 0.8,
            estimated_tokens: 200,
            keyword_features: HashMap::new(),
            length_features: 0.5,
            structural_features: 0.3,
        };

        let context_features = prompt_features.to_feature_vector();

        let gpt4_score = preferences.calculate_model_preference_score("gpt-4", &context_features);
        let claude_score = preferences.calculate_model_preference_score("claude-3", &context_features);
        let llama_score = preferences.calculate_model_preference_score("llama-3.1", &context_features);

        assert!(gpt4_score > 1.4);
        assert!(claude_score > 1.1);
        assert!(llama_score < 0.2);
    }
}