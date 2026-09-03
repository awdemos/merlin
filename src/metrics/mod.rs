//! Metrics and observability using Prometheus.

use prometheus::{Counter, Encoder, Histogram, Registry, TextEncoder};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Metrics {
    registry: Arc<Registry>,
    requests_total: Counter,
    errors_total: Counter,
    latency: Histogram,
    tokens_total: Counter,
}

impl Metrics {
    pub fn new() -> anyhow::Result<Self> {
        let registry = Registry::new();

        let requests_total =
            Counter::new("merlin_requests_total", "Total number of requests routed")?;
        let errors_total = Counter::new(
            "merlin_errors_total",
            "Total number of failed upstream requests",
        )?;
        let latency = Histogram::with_opts(
            prometheus::HistogramOpts::new("merlin_latency_seconds", "End-to-end request latency")
                .buckets(prometheus::exponential_buckets(0.001, 2.0, 16).unwrap_or_default()),
        )?;
        let tokens_total = Counter::new("merlin_tokens_total", "Total tokens consumed")?;

        registry.register(Box::new(requests_total.clone()))?;
        registry.register(Box::new(errors_total.clone()))?;
        registry.register(Box::new(latency.clone()))?;
        registry.register(Box::new(tokens_total.clone()))?;

        Ok(Self {
            registry: Arc::new(registry),
            requests_total,
            errors_total,
            latency,
            tokens_total,
        })
    }

    pub fn record_request(
        &self,
        _route: &str,
        target: &str,
        success: bool,
        duration: std::time::Duration,
    ) {
        let labels = format!("{{target=\"{}\"}}", target);
        if success {
            self.requests_total.inc();
        } else {
            self.errors_total.inc();
        }
        self.latency.observe(duration.as_secs_f64());
        let _ = labels;
    }

    pub fn record_tokens(&self, _route: &str, _target: &str, tokens: u64) {
        self.tokens_total.inc_by(tokens as f64);
    }

    pub fn record_fallback(&self, _route: &str, _target: &str) {
        self.errors_total.inc();
    }

    pub fn encode(&self) -> anyhow::Result<String> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}
