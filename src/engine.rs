//! Routing engine: owns clients, algorithms, metrics, and executes requests.

use crate::clients::translating::TranslatingLlmClient;
use crate::clients::LlmClient;
use crate::config::{AlgorithmConfig, MerlinConfig, RouteConfig, TargetConfig};
use crate::features::FeatureClient;
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
    features: Option<FeatureClient>,
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

        let features = if config.features.enabled {
            Some(FeatureClient::new(
                config.features.url.clone(),
                config.features.timeout_ms,
                config.features.cache_entries,
                config.features.cache_ttl_secs,
            ))
        } else {
            None
        };

        Ok(Self {
            config,
            clients,
            routes,
            metrics: Metrics::new()?,
            features,
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

    /// All target names reachable by a route: its declared targets plus
    /// any targets the algorithm fixes internally (passthrough,
    /// classifier). The gate scopes eligibility over exactly this set.
    pub fn route_scope(&self, route_name: &str) -> Vec<String> {
        let route = match self.routes.get(route_name) {
            Some(route) => route,
            None => return Vec::new(),
        };
        let mut names = route.config.targets.clone();
        names.extend(route.config.algorithm.target_names());
        names.sort();
        names.dedup();
        names
    }

    /// True when the target's client runs on local infrastructure
    /// (loopback/private-range base_url).
    pub fn target_locality(&self, target_name: &str) -> bool {
        self.config
            .targets
            .get(target_name)
            .and_then(|t| self.config.clients.get(&t.client))
            .map(|c| crate::governance::gate::host_is_local(&c.base_url))
            .unwrap_or(false)
    }

    /// (price_input, price_output) in USD per 1M tokens, if priced.
    pub fn target_price(&self, target_name: &str) -> Option<(f64, f64)> {
        let target = self.config.targets.get(target_name)?;
        Some((target.price_input?, target.price_output?))
    }

    /// Like [`RouterEngine::execute`], but restricted to `eligible` target
    /// names (from the governance gate). `None` = no restriction (plain
    /// execute). Dispatched targets are always a subset of the eligible
    /// set: the candidate list is pre-filtered (honored by
    /// passthrough/classifier), the selected target is verified
    /// post-selection (fail-closed `policy_denied` on escape), and
    /// fallbacks are filtered to the eligible set. A denial has no
    /// dispatch, metrics, or reward side effects (sidecar feature
    /// attachment may still run for `needs_features` algorithms).
    pub async fn execute_governed(
        &self,
        route_name: &str,
        request: Request,
        eligible: Option<&[String]>,
    ) -> anyhow::Result<LlmResponse> {
        self.execute_inner(route_name, request, eligible).await
    }

    /// Governed variant of [`RouterEngine::decide`]; same eligible-set
    /// guarantee and fail-closed denial semantics as
    /// [`RouterEngine::execute_governed`].
    pub async fn decide_governed(
        &self,
        route_name: &str,
        request: Request,
        eligible: Option<&[String]>,
    ) -> anyhow::Result<RoutingOutcome> {
        self.decide_inner(route_name, request, eligible).await
    }

    pub async fn execute(&self, route_name: &str, request: Request) -> anyhow::Result<LlmResponse> {
        self.execute_inner(route_name, request, None).await
    }

    async fn execute_inner(
        &self,
        route_name: &str,
        mut request: Request,
        eligible: Option<&[String]>,
    ) -> anyhow::Result<LlmResponse> {
        let route = self
            .routes
            .get(route_name)
            .ok_or_else(|| anyhow::anyhow!("unknown route: {}", route_name))?;

        if request.llm_request.model.is_none() {
            request.llm_request.model = route.config.default_model.clone();
        }

        self.attach_features(route, &mut request).await;

        let outcome = match eligible {
            Some(eligible) => self.select_governed(route, route_name, &request, eligible)?,
            None => {
                let targets: Vec<TargetConfig> = route
                    .config
                    .targets
                    .iter()
                    .filter_map(|t| self.config.targets.get(t).cloned())
                    .collect();
                route.algorithm.select(&request, &targets)?
            }
        };
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
        request: Request,
    ) -> anyhow::Result<RoutingOutcome> {
        self.decide_inner(route_name, request, None).await
    }

    async fn decide_inner(
        &self,
        route_name: &str,
        mut request: Request,
        eligible: Option<&[String]>,
    ) -> anyhow::Result<RoutingOutcome> {
        let route = self
            .routes
            .get(route_name)
            .ok_or_else(|| anyhow::anyhow!("unknown route: {}", route_name))?;
        if request.llm_request.model.is_none() {
            request.llm_request.model = route.config.default_model.clone();
        }
        self.attach_features(route, &mut request).await;
        let outcome = match eligible {
            Some(eligible) => self.select_governed(route, route_name, &request, eligible)?,
            None => {
                let targets: Vec<TargetConfig> = route
                    .config
                    .targets
                    .iter()
                    .filter_map(|t| self.config.targets.get(t).cloned())
                    .collect();
                route.algorithm.select(&request, &targets)?
            }
        };
        Ok(RoutingOutcome {
            target: self.resolve_target(&outcome.selected)?,
            fallback_chain: outcome.fallbacks,
        })
    }

    /// Governed selection: filter candidates to the eligible set, run the
    /// algorithm, then enforce eligibility fail-closed — a selected target
    /// outside the eligible set is a denial (before any dispatch side
    /// effects) and fallbacks are restricted to the eligible set. Candidates
    /// come from the route's declared targets only; algorithm-fixed targets
    /// (passthrough, classifier) are validated by the post-selection check,
    /// never admitted into the candidate list.
    fn select_governed(
        &self,
        route: &Route,
        route_name: &str,
        request: &Request,
        eligible: &[String],
    ) -> anyhow::Result<crate::routing::AlgorithmOutcome> {
        let eligible_set: std::collections::HashSet<&str> =
            eligible.iter().map(String::as_str).collect();
        let candidates: Vec<TargetConfig> = route
            .config
            .targets
            .iter()
            .filter(|t| eligible_set.contains(t.as_str()))
            .filter_map(|t| self.config.targets.get(t).cloned())
            .collect();
        if candidates.is_empty() {
            anyhow::bail!("policy_denied: no eligible targets in route {}", route_name);
        }
        let outcome = route.algorithm.select(request, &candidates)?;
        // Fail-closed enforcement: arm-based algorithms select from their
        // construction-time arm list and ignore the filtered candidate
        // list, so verify the selection against the eligible set and drop
        // ineligible fallbacks. A selection outside the eligible set is a
        // denial, never a dispatch.
        if !eligible_set.contains(outcome.selected.as_str()) {
            anyhow::bail!(
                "policy_denied: selected target '{}' outside eligible set",
                outcome.selected
            );
        }
        let fallbacks: Vec<String> = outcome
            .fallbacks
            .iter()
            .filter(|name| eligible_set.contains(name.as_str()))
            .cloned()
            .collect();
        Ok(crate::routing::AlgorithmOutcome {
            selected: outcome.selected,
            fallbacks,
        })
    }

    /// Fetch sidecar features for algorithms that need them and attach them
    /// to the request so `select` and the later `record_reward` see the same
    /// context. `None` on sidecar failure: algorithms fall back to degraded
    /// local signals.
    async fn attach_features(&self, route: &Route, request: &mut Request) {
        if !route.algorithm.needs_features() {
            return;
        }
        if let Some(client) = &self.features {
            let prompt = request.llm_request.prompt_text();
            request.features = client.features(&prompt).await;
        }
    }

    /// Guardrail pre-filter: asks the sidecar for a verdict and blocks the
    /// request when jailbreak / prompt-injection scores reach their
    /// thresholds. Fail-open: disabled, no client, or sidecar error all
    /// allow the request through.
    pub async fn guard_check(&self, request: &Request) -> Result<(), String> {
        let cfg = &self.config.features;
        if !cfg.enabled || !cfg.guard_enabled {
            return Ok(());
        }
        let client = match &self.features {
            Some(c) => c,
            None => return Ok(()),
        };
        let prompt = request.llm_request.prompt_text();
        match client.guard(&prompt).await {
            None => Ok(()),
            Some(verdict) => match verdict
                .block_reason(cfg.jailbreak_threshold, cfg.prompt_injection_threshold)
            {
                Some(reason) => {
                    tracing::info!("prompt blocked by guard: {}", reason);
                    Err(reason)
                }
                None => Ok(()),
            },
        }
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
            vec![crate::protocol::ContentBlock::Text {
                text: String::new(),
            }]
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
