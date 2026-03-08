//! Performance optimization module for Merlin AI Router
//! Provides performance monitoring, optimization strategies, and resource management

pub mod optimization;

pub use optimization::{
    PerformanceOptimizationService, PerformanceMetrics, PerformanceConfig,
    OptimizationStrategy, PerformanceAlert, AlertSeverity, OptimizationRecommendation,
    RecommendationPriority, ImplementationCost, RiskLevel, PerformanceSummary,
};