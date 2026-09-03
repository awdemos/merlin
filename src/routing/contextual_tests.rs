#[cfg(test)]
mod tests {
    use crate::routing::{ContextualArm, RoutingPolicy};

    #[test]
    fn test_contextual_arm_creation() {
        let feature_dim = 5;
        let arm = ContextualArm::new(feature_dim);

        assert_eq!(arm.weights.len(), feature_dim);
        assert_eq!(arm.total_reward, 0.0);
        assert_eq!(arm.num_pulls, 0);
        assert_eq!(arm.feature_norm, 1.0);
    }

    #[test]
    fn test_contextual_arm_prediction() {
        let feature_dim = 3;
        let mut arm = ContextualArm::new(feature_dim);

        // Set some weights
        arm.weights = vec![1.0, 2.0, 3.0];

        let features = vec![1.0, 0.5, 0.33];
        let prediction = arm.predict(&features);

        // Expected: 1.0*1.0 + 2.0*0.5 + 3.0*0.33 = 1.0 + 1.0 + 0.99 = 2.99
        assert!((prediction - 2.99).abs() < 0.01);
    }

    #[test]
    fn test_contextual_arm_update() {
        let feature_dim = 2;
        let mut arm = ContextualArm::new(feature_dim);

        let features = vec![1.0, 0.5];
        let reward = 0.8;
        let learning_rate = 0.1;

        arm.update(&features, reward, learning_rate);

        assert_eq!(arm.num_pulls, 1);
        assert!((arm.total_reward - reward).abs() < 0.001);

        // Weights should have been updated
        assert_ne!(arm.weights, vec![0.0, 0.0]);
    }

    #[test]
    fn test_contextual_routing_policy_creation() {
        let num_providers = 3;
        let feature_dim = 4;
        let learning_rate = 0.1;
        let exploration_rate = 0.2;

        let policy = RoutingPolicy::new_contextual(
            num_providers,
            feature_dim,
            learning_rate,
            exploration_rate,
        );

        match policy {
            RoutingPolicy::Contextual {
                arms,
                feature_dim: fd,
                learning_rate: lr,
                exploration_rate: er,
            } => {
                assert_eq!(arms.len(), num_providers);
                assert_eq!(fd, feature_dim);
                assert!((lr - learning_rate).abs() < 0.001);
                assert!((er - exploration_rate).abs() < 0.001);
            }
            _ => panic!("Expected Contextual routing policy"),
        }
    }

    #[test]
    fn test_contextual_routing_selection() {
        let num_providers = 2;
        let feature_dim = 3;
        let policy = RoutingPolicy::new_contextual(num_providers, feature_dim, 0.1, 0.0); // No exploration

        let context_features = vec![1.0, 0.5, 0.25];
        let selected_index = policy.select_index_with_context(num_providers, &context_features);

        assert!(selected_index < num_providers);
    }

    #[test]
    fn test_contextual_routing_with_exploration() {
        let num_providers = 3;
        let feature_dim = 2;
        let policy = RoutingPolicy::new_contextual(num_providers, feature_dim, 0.1, 1.0); // Always explore

        let context_features = vec![1.0, 0.5];

        // With 100% exploration, should be random
        let selections: Vec<usize> = (0..100)
            .map(|_| policy.select_index_with_context(num_providers, &context_features))
            .collect();

        // Should see different selections due to exploration
        let unique_selections: std::collections::HashSet<_> = selections.iter().collect();
        assert!(unique_selections.len() > 1);
    }

    #[test]
    fn test_contextual_routing_update() {
        let num_providers = 2;
        let feature_dim = 3;
        let mut policy = RoutingPolicy::new_contextual(num_providers, feature_dim, 0.1, 0.1);

        let context_features = vec![1.0, 0.5, 0.25];
        let provider_index = 0;
        let reward_score = 0.8;

        policy.update_reward_with_context(provider_index, &context_features, reward_score);

        // Policy should still be valid after update
        let _selected = policy.select_index_with_context(num_providers, &context_features);
    }

    #[test]
    fn test_feature_vector_mismatch_handling() {
        let feature_dim = 3;
        let mut arm = ContextualArm::new(feature_dim);

        // Test with wrong feature dimensions
        let wrong_features = vec![1.0, 0.5]; // Only 2 features instead of 3
        let prediction = arm.predict(&wrong_features);

        // Should return 0.0 for mismatched dimensions
        assert_eq!(prediction, 0.0);

        // Update should handle mismatch gracefully
        arm.update(&wrong_features, 0.8, 0.1);
        assert_eq!(arm.num_pulls, 0); // Should not update
    }

    #[test]
    fn test_normalization_handling() {
        let feature_dim = 2;
        let mut arm = ContextualArm::new(feature_dim);

        // Set weights that would cause division by zero
        arm.weights = vec![0.0, 0.0];
        arm.feature_norm = 0.0;

        let features = vec![1.0, 1.0];
        let prediction = arm.predict(&features);

        // Should handle zero norm gracefully
        assert!(prediction.is_finite());
    }

    #[test]
    fn test_learning_convergence() {
        let feature_dim = 2;
        let mut arm = ContextualArm::new(feature_dim);

        // Initialize with small random weights to avoid zero weights
        arm.weights = vec![0.1, 0.1];

        // Train the arm with consistent patterns
        let features = vec![1.0, 0.5];
        let target_reward = 0.9;

        for _ in 0..200 {
            arm.update(&features, target_reward, 0.05); // Lower learning rate for stability
        }

        // After training, predictions should be close to target
        let prediction = arm.predict(&features);
        // Allow more tolerance since this is simple linear learning
        assert!((prediction - target_reward).abs() < 0.5);
    }

    #[test]
    fn test_score_only_update_moves_bias() {
        let feature_dim = 2;
        let mut arm = ContextualArm::new(feature_dim);

        // Rewards without context features accumulate on the bias
        arm.update_with_score(1.0, 0.1);
        arm.update_with_score(1.0, 0.1);

        assert_eq!(arm.num_pulls, 2);
        assert_eq!(arm.total_reward, 2.0);
        assert!(arm.bias > 0.0, "bias should move toward the reward");
        // Prediction reflects the learned bias
        assert!(arm.predict(&[1.0, 1.0]) > 0.0);
    }

    #[test]
    fn test_binary_reward_updates_contextual_arm() {
        let mut policy = RoutingPolicy::new_contextual(2, 2, 0.1, 0.0);

        // update_reward previously no-op'd for Contextual policies
        policy.update_reward(0, true);

        if let RoutingPolicy::Contextual { arms, .. } = &policy {
            let arm = arms.get(&0).unwrap();
            assert_eq!(arm.num_pulls, 1);
            assert_eq!(arm.total_reward, 1.0);
            assert!(arm.bias > 0.0);
        } else {
            panic!("Expected Contextual routing policy");
        }
    }

    #[test]
    fn test_cold_start_tie_breaks_toward_unpulled_arms() {
        let mut policy = RoutingPolicy::new_contextual(3, 2, 0.1, 0.0);
        let features = vec![1.0, 1.0];

        // All arms predict 0.0 with zero pulls: first arm wins the tie
        assert_eq!(policy.select_index_with_context(3, &features), 0);

        // A failed pull leaves the score at 0.0 but records a pull, so the
        // least-pulled arm (arm 1) must be walked next instead of always
        // re-picking arm 0
        policy.update_reward_with_score(0, 0.0);
        assert_eq!(policy.select_index_with_context(3, &features), 1);

        policy.update_reward_with_score(1, 0.0);
        assert_eq!(policy.select_index_with_context(3, &features), 2);
    }

    #[test]
    fn test_rewards_change_exploit_selection() {
        let mut policy = RoutingPolicy::new_contextual(2, 2, 0.1, 0.0);
        let features = vec![1.0, 1.0];

        // Walk both arms once so neither is preferred for being unpulled
        policy.update_reward_with_score(0, 1.0);
        policy.update_reward_with_score(1, 0.0);

        // Arm 0 received the better reward, so exploiting must pick it —
        // and keep picking it as its bias grows
        for _ in 0..3 {
            assert_eq!(policy.select_index_with_context(2, &features), 0);
            policy.update_reward_with_score(0, 1.0);
        }
    }
}
