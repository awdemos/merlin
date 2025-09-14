// src/ab_testing/metrics.rs
use crate::ab_testing::config::MetricType;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentMetrics {
    pub variant_id: String,
    pub total_interactions: u32,
    pub successful_interactions: u32,
    pub total_response_time_ms: u64,
    pub total_cost: f64,
    pub user_ratings: Vec<u8>,
    pub errors: Vec<String>,
    pub response_times: Vec<u32>, // For percentile calculations
    pub metrics_by_type: HashMap<MetricType, f64>,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl ExperimentMetrics {
    pub fn new(variant_id: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            variant_id,
            total_interactions: 0,
            successful_interactions: 0,
            total_response_time_ms: 0,
            total_cost: 0.0,
            user_ratings: Vec::new(),
            errors: Vec::new(),
            response_times: Vec::new(),
            metrics_by_type: HashMap::new(),
            start_time: now,
            end_time: None,
        }
    }

    pub fn record_interaction(&mut self, metrics: &super::experiment::InteractionMetrics) {
        self.total_interactions += 1;

        if metrics.success {
            self.successful_interactions += 1;
        }

        self.total_response_time_ms += metrics.response_time_ms as u64;
        self.total_cost += metrics.cost;

        if let Some(rating) = metrics.user_rating {
            self.user_ratings.push(rating);
        }

        if let Some(error) = &metrics.error_message {
            self.errors.push(error.clone());
        }

        self.response_times.push(metrics.response_time_ms);

        // Update derived metrics
        self.update_derived_metrics();
    }

    fn update_derived_metrics(&mut self) {
        // Success rate
        let success_rate = if self.total_interactions > 0 {
            self.successful_interactions as f64 / self.total_interactions as f64
        } else {
            0.0
        };
        self.metrics_by_type.insert(MetricType::SuccessRate, success_rate);

        // Average response time
        let avg_response_time = if self.total_interactions > 0 {
            (self.total_response_time_ms as f64 / self.total_interactions as f64) / 1000.0
        } else {
            0.0
        };
        self.metrics_by_type.insert(MetricType::ResponseTime, avg_response_time);

        // User satisfaction (average rating)
        let avg_rating = if !self.user_ratings.is_empty() {
            self.user_ratings.iter().sum::<u8>() as f64 / self.user_ratings.len() as f64
        } else {
            0.0
        };
        self.metrics_by_type.insert(MetricType::UserSatisfaction, avg_rating);

        // Cost efficiency (success rate per cost)
        let cost_efficiency = if self.total_cost > 0.0 {
            success_rate / self.total_cost
        } else {
            0.0
        };
        self.metrics_by_type.insert(MetricType::CostEfficiency, cost_efficiency);

        // Error rate
        let error_rate = if self.total_interactions > 0 {
            self.errors.len() as f64 / self.total_interactions as f64
        } else {
            0.0
        };
        self.metrics_by_type.insert(MetricType::ErrorRate, error_rate);

        // Throughput (interactions per second)
        let duration = self.end_time.unwrap_or(chrono::Utc::now()) - self.start_time;
        let throughput = if duration.num_seconds() > 0 {
            self.total_interactions as f64 / duration.num_seconds() as f64
        } else {
            0.0
        };
        self.metrics_by_type.insert(MetricType::Throughput, throughput);
    }

    pub fn average_success_rate(&self) -> f64 {
        self.metrics_by_type.get(&MetricType::SuccessRate).copied().unwrap_or(0.0)
    }

    pub fn success_rate_std_dev(&self) -> f64 {
        // Simple standard deviation calculation for binary outcome (success/failure)
        let p = self.average_success_rate();
        (p * (1.0 - p)).sqrt()
    }

    pub fn percentile_response_time(&self, percentile: f64) -> Option<u32> {
        if self.response_times.is_empty() {
            return None;
        }

        let mut times = self.response_times.clone();
        times.sort();

        let index = (percentile / 100.0 * (times.len() - 1) as f64) as usize;
        Some(times[index.min(times.len() - 1)])
    }

    pub fn get_summary(&self) -> MetricsSummary {
        MetricsSummary {
            variant_id: self.variant_id.clone(),
            total_interactions: self.total_interactions,
            success_rate: self.average_success_rate(),
            avg_response_time_sec: self.total_response_time_ms as f64 / (self.total_interactions.max(1) as f64) / 1000.0,
            p50_response_time: self.percentile_response_time(50.0),
            p95_response_time: self.percentile_response_time(95.0),
            p99_response_time: self.percentile_response_time(99.0),
            avg_user_rating: if !self.user_ratings.is_empty() {
                Some(self.user_ratings.iter().sum::<u8>() as f64 / self.user_ratings.len() as f64)
            } else {
                None
            },
            total_cost: self.total_cost,
            avg_cost_per_interaction: if self.total_interactions > 0 {
                Some(self.total_cost / self.total_interactions as f64)
            } else {
                None
            },
            error_rate: if self.total_interactions > 0 {
                Some(self.errors.len() as f64 / self.total_interactions as f64)
            } else {
                None
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    pub variant_id: String,
    pub total_interactions: u32,
    pub success_rate: f64,
    pub avg_response_time_sec: f64,
    pub p50_response_time: Option<u32>,
    pub p95_response_time: Option<u32>,
    pub p99_response_time: Option<u32>,
    pub avg_user_rating: Option<f64>,
    pub total_cost: f64,
    pub avg_cost_per_interaction: Option<f64>,
    pub error_rate: Option<f64>,
}

pub struct MetricsCollector {
    pub experiment_metrics: HashMap<String, ExperimentMetrics>,
    pub global_metrics: GlobalMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalMetrics {
    pub total_experiments: u32,
    pub active_experiments: u32,
    pub total_users: u32,
    pub total_interactions: u64,
    pub start_time: chrono::DateTime<chrono::Utc>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            experiment_metrics: HashMap::new(),
            global_metrics: GlobalMetrics {
                total_experiments: 0,
                active_experiments: 0,
                total_users: 0,
                total_interactions: 0,
                start_time: chrono::Utc::now(),
            },
        }
    }

    pub fn record_experiment_interaction(
        &mut self,
        _experiment_id: &str,
        variant_id: &str,
        _user_id: &str,
        metrics: &super::experiment::InteractionMetrics,
    ) {
        // Get or create metrics for this variant
        let variant_metrics = self.experiment_metrics
            .entry(variant_id.to_string())
            .or_insert_with(|| ExperimentMetrics::new(variant_id.to_string()));

        variant_metrics.record_interaction(metrics);

        // Update global metrics
        self.global_metrics.total_interactions += 1;
        // Note: In a real implementation, we'd track unique users
    }

    pub fn get_experiment_metrics(&self, variant_id: &str) -> Option<&ExperimentMetrics> {
        self.experiment_metrics.get(variant_id)
    }

    pub fn get_global_summary(&self) -> GlobalMetricsSummary {
        GlobalMetricsSummary {
            total_experiments: self.global_metrics.total_experiments,
            active_experiments: self.global_metrics.active_experiments,
            total_users: self.global_metrics.total_users,
            total_interactions: self.global_metrics.total_interactions,
            avg_success_rate: self.calculate_global_success_rate(),
            avg_response_time_sec: self.calculate_global_avg_response_time(),
            total_cost: self.experiment_metrics.values()
                .map(|m| m.total_cost)
                .sum(),
        }
    }

    fn calculate_global_success_rate(&self) -> f64 {
        let total_interactions: u32 = self.experiment_metrics.values()
            .map(|m| m.total_interactions)
            .sum();

        if total_interactions == 0 {
            return 0.0;
        }

        let total_successes: u32 = self.experiment_metrics.values()
            .map(|m| m.successful_interactions)
            .sum();

        total_successes as f64 / total_interactions as f64
    }

    fn calculate_global_avg_response_time(&self) -> f64 {
        let total_interactions: u32 = self.experiment_metrics.values()
            .map(|m| m.total_interactions)
            .sum();

        if total_interactions == 0 {
            return 0.0;
        }

        let total_time: u64 = self.experiment_metrics.values()
            .map(|m| m.total_response_time_ms)
            .sum();

        (total_time as f64 / total_interactions as f64) / 1000.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalMetricsSummary {
    pub total_experiments: u32,
    pub active_experiments: u32,
    pub total_users: u32,
    pub total_interactions: u64,
    pub avg_success_rate: f64,
    pub avg_response_time_sec: f64,
    pub total_cost: f64,
}