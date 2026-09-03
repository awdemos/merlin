//! Routing engine: owns clients, algorithms, metrics, and executes requests.

use crate::clients::translating::TranslatingLlmClient;
use crate::clients::LlmClient;
use crate::config::{AlgorithmConfig, MerlinConfig, RouteConfig, TargetConfig};
use crate::metrics::Metrics;
use crate::protocol::{
    AggLlmResponse, LlmResponse, LlmResponseStream, Request, RoutingOutcome, TargetRef,
};
use crate::routing::{
    ContextualBandit, EpsilonGreedy, LlmClassifier, RandomRouter, RoutingAlgorithm,
    ThompsonSampling, UpperConfidenceBound,
};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug)]
pub struct RouterEngine {
    config: MerlinConfig,
    clients: HashMap<String, Arc<dyn LlmClient>>,
    routes: HashMap<String, Route>,
    metrics: Metrics,
}

#[derive(Debug)]
struct Route {
    config: RouteConfig,
    algorithm: Box<dyn RoutingAlgorithm>,
}

impl RouterEngine {
    pub fn new(config: MerlinConfig) -> anyhow::Result<Self> {
        config.validate()?;

        let mut clients: HashMap<String, Arc<dyn LlmClient>> = HashMap::new();
        for (id, client_config) in &config.clients {
            let client = TranslatingLlmClient::new(id.clone(), client_config)?;
            clients.insert(id.clone(), Arc::new(client));
        }

        let mut routes = HashMap::new();
        for (name, route_config) in &config.routes {
            let algorithm =
                build_algorithm(&config, &route_config.algorithm, &route_config.targets)?;
            routes.insert(
                name.clone(),
                Route {
                    config: route_config.clone(),
                    algorithm,
                },
            );
        }

        Ok(Self {
            config,
            clients,
            routes,
            metrics: Metrics::new()?,
        })
    }

    pub fn metrics(&self) -> &Metrics {
        &self.metrics
    }

    pub fn list_routes(&self) -> Vec<String> {
        self.routes.keys().cloned().collect()
    }

    pub fn route_models(&self) -> Vec<serde_json::Value> {
        self.routes
            .iter()
            .map(|(name, route)| {
                serde_json::json!({
                    "id": name,
                    "object": "model",
                    "owned_by": "merlin",
                    "targets": route.config.targets
                })
            })
            .collect()
    }

    pub async fn execute(
        &self,
        route_name: &str,
        mut request: Request,
    ) -> anyhow::Result<LlmResponse> {
        let route = self
            .routes
            .get(route_name)
            .ok_or_else(|| anyhow::anyhow!("unknown route: {}", route_name))?;

        if request.llm_request.model.is_none() {
            request.llm_request.model = route.config.default_model.clone();
        }

        let targets: Vec<TargetConfig> = route
            .config
            .targets
            .iter()
            .filter_map(|t| self.config.targets.get(t).cloned())
            .collect();

        let outcome = route.algorithm.select(&request, &targets)?;
        let selected = self.resolve_target(&outcome.selected)?;
        let fallbacks: Vec<TargetRef> = outcome
            .fallbacks
            .iter()
            .filter_map(|n| self.resolve_target(n).ok())
            .collect();

        let start = std::time::Instant::now();
        let result = self
            .dispatch_with_fallbacks(&selected, &fallbacks, request.clone(), route_name)
            .await;
        let duration = start.elapsed();

        match &result {
            Ok(resp) => {
                self.metrics
                    .record_request(route_name, &selected.name, true, duration);
                if let LlmResponse::Aggregate(agg) = resp {
                    self.metrics.record_tokens(
                        route_name,
                        &selected.name,
                        agg.usage.prompt_tokens + agg.usage.completion_tokens,
                    );
                }
                // Simple reward: success == 1.0.
                route.algorithm.record_reward(&selected.name, &request, 1.0);
            }
            Err(_) => {
                self.metrics
                    .record_request(route_name, &selected.name, false, duration);
                route.algorithm.record_reward(&selected.name, &request, 0.0);
            }
        }

        result
    }

    pub async fn decide(
        &self,
        route_name: &str,
        mut request: Request,
    ) -> anyhow::Result<RoutingOutcome> {
        let route = self
            .routes
            .get(route_name)
            .ok_or_else(|| anyhow::anyhow!("unknown route: {}", route_name))?;
        if request.llm_request.model.is_none() {
            request.llm_request.model = route.config.default_model.clone();
        }
        let targets: Vec<TargetConfig> = route
            .config
            .targets
            .iter()
            .filter_map(|t| self.config.targets.get(t).cloned())
            .collect();
        let outcome = route.algorithm.select(&request, &targets)?;
        Ok(RoutingOutcome {
            target: self.resolve_target(&outcome.selected)?,
            fallback_chain: outcome.fallbacks,
        })
    }

    fn resolve_target(&self, name: &str) -> anyhow::Result<TargetRef> {
        let target = self
            .config
            .targets
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("target {} not found", name))?;
        let client = self.config.clients.get(&target.client).ok_or_else(|| {
            anyhow::anyhow!("client {} for target {} not found", target.client, name)
        })?;
        Ok(TargetRef {
            name: target.name.clone(),
            client: target.client.clone(),
            model_id: target.model.clone(),
            format: client.format(),
            base_url: client.base_url.clone(),
            timeout_ms: client.timeout_ms,
            extra_headers: client.extra_headers.clone(),
        })
    }

    async fn dispatch_with_fallbacks(
        &self,
        primary: &TargetRef,
        fallbacks: &[TargetRef],
        request: Request,
        route_name: &str,
    ) -> anyhow::Result<LlmResponse> {
        let mut last_err = None;
        for target in std::iter::once(primary).chain(fallbacks.iter()) {
            match self.dispatch_one(target, request.clone()).await {
                Ok(mut resp) => {
                    // Stamp preservation metadata.
                    stamp(&mut resp, route_name, primary, fallbacks);
                    return Ok(resp);
                }
                Err(e) => {
                    tracing::warn!("target {} failed: {}", target.name, e);
                    self.metrics.record_fallback(route_name, &target.name);
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("no targets available")))
    }

    async fn dispatch_one(
        &self,
        target: &TargetRef,
        request: Request,
    ) -> anyhow::Result<LlmResponse> {
        let client = self
            .clients
            .get(&target.client)
            .ok_or_else(|| anyhow::anyhow!("client {} not found", target.client))?;

        let mut req = request;
        req.llm_request.model = Some(target.model_id.clone());

        client.execute(req).await
    }
}

fn stamp(resp: &mut LlmResponse, route_name: &str, primary: &TargetRef, fallbacks: &[TargetRef]) {
    let preservation = crate::protocol::PreservationMetadata {
        selected_target: Some(primary.name.clone()),
        routing_algorithm: Some(route_name.to_string()),
        fallback_path: Some(fallbacks.iter().map(|t| t.name.clone()).collect()),
        extra: Default::default(),
    };
    match resp {
        LlmResponse::Aggregate(agg) => {
            agg.preservation = preservation;
        }
        LlmResponse::Stream(_) => {
            // Stream preservation not yet attached.
        }
    }
}

fn build_algorithm(
    config: &MerlinConfig,
    algo_config: &AlgorithmConfig,
    target_names: &[String],
) -> anyhow::Result<Box<dyn RoutingAlgorithm>> {
    let target_configs: Vec<TargetConfig> = target_names
        .iter()
        .filter_map(|n| config.targets.get(n).cloned())
        .collect();

    match algo_config {
        AlgorithmConfig::Passthrough { target } => {
            let t = config
                .targets
                .get(target)
                .ok_or_else(|| anyhow::anyhow!("passthrough target {} not found", target))?
                .clone();
            Ok(Box::new(crate::routing::Passthrough::new(t)))
        }
        AlgorithmConfig::Random { seed, weights, .. } => Ok(Box::new(RandomRouter::new(
            target_configs,
            *seed,
            weights.clone(),
        ))),
        AlgorithmConfig::EpsilonGreedy { epsilon, .. } => {
            Ok(Box::new(EpsilonGreedy::new(target_configs, *epsilon)))
        }
        AlgorithmConfig::Thompson { .. } => Ok(Box::new(ThompsonSampling::new(target_configs))),
        AlgorithmConfig::Ucb {
            confidence_level, ..
        } => Ok(Box::new(UpperConfidenceBound::new(
            target_configs,
            confidence_level.unwrap_or(2.0),
        ))),
        AlgorithmConfig::Contextual {
            learning_rate,
            exploration_rate,
            ..
        } => Ok(Box::new(ContextualBandit::new(
            target_configs,
            learning_rate.unwrap_or(0.01),
            exploration_rate.unwrap_or(0.15),
        )?)),
        AlgorithmConfig::LlmClassifier {
            classifier_target,
            strong_target,
            weak_target,
            ..
        } => {
            let classifier = config
                .targets
                .get(classifier_target)
                .ok_or_else(|| {
                    anyhow::anyhow!("classifier target {} not found", classifier_target)
                })?
                .clone();
            let strong = config
                .targets
                .get(strong_target)
                .ok_or_else(|| anyhow::anyhow!("strong target {} not found", strong_target))?
                .clone();
            let weak = config
                .targets
                .get(weak_target)
                .ok_or_else(|| anyhow::anyhow!("weak target {} not found", weak_target))?
                .clone();
            Ok(Box::new(LlmClassifier::new(classifier, strong, weak)))
        }
    }
}

/// Aggregate a streamed response into a completed response for callers that
/// cannot consume streams.
pub async fn aggregate_stream(stream: LlmResponseStream) -> anyhow::Result<AggLlmResponse> {
    use futures::StreamExt;
    let mut text = String::new();
    let mut tool_calls: Vec<crate::protocol::ToolCall> = Vec::new();
    let mut usage = crate::protocol::Usage::default();
    let mut stop_reason = crate::protocol::StopReason::Unknown;

    let mut s = stream.inner;
    while let Some(event) = s.next().await {
        match &event.chunk {
            crate::protocol::LlmResponseChunk::ContentDelta { block } => {
                if let crate::protocol::ContentBlock::Text { text: t } = block {
                    text.push_str(t);
                }
            }
            crate::protocol::LlmResponseChunk::ToolCallDelta { calls } => {
                tool_calls.extend(calls.clone());
            }
            crate::protocol::LlmResponseChunk::Usage { usage: u } => {
                usage = *u;
            }
            crate::protocol::LlmResponseChunk::Done => {
                stop_reason = crate::protocol::StopReason::Stop;
            }
            crate::protocol::LlmResponseChunk::Progress => {}
        }
        if let Some(u) = event.usage {
            usage = u;
        }
    }

    Ok(AggLlmResponse {
        id: String::new(),
        model: String::new(),
        content: if text.is_empty() {
            Vec::new()
        } else {
            vec![crate::protocol::ContentBlock::Text { text }]
        },
        tool_calls: if tool_calls.is_empty() {
            None
        } else {
            Some(tool_calls)
        },
        usage,
        stop_reason,
        extra: Default::default(),
        preservation: Default::default(),
    })
}
