use crate::models::container_state::{ContainerState, ContainerStatus, ContainerMetrics, ContainerEvent};
use crate::models::docker_config::{DockerContainerConfig, DockerConfigError};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use warp::{Filter, Rejection, Reply};

/// Service for managing container state and monitoring
#[derive(Clone)]
pub struct ContainerStateService {
    /// Store for container states
    container_states: Arc<RwLock<HashMap<uuid::Uuid, ContainerState>>>,

    /// Store for container metrics
    container_metrics: Arc<RwLock<HashMap<uuid::Uuid, ContainerMetrics>>>,

    /// Store for container events
    container_events: Arc<RwLock<HashMap<uuid::Uuid, Vec<ContainerEvent>>>>,

    /// Store for active deployments
    active_deployments: Arc<RwLock<HashMap<uuid::Uuid, String>>>,
}

impl ContainerStateService {
    /// Create a new container state service
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            container_states: Arc::new(RwLock::new(HashMap::new())),
            container_metrics: Arc::new(RwLock::new(HashMap::new())),
            container_events: Arc::new(RwLock::new(HashMap::new())),
            active_deployments: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Create a new container state
    pub async fn create_container_state(&self, config_id: uuid::Uuid, image_id: String) -> Result<uuid::Uuid, DockerConfigError> {
        let state_id = uuid::Uuid::new_v4();
        let now = chrono::Utc::now();

        let state = ContainerState {
            id: state_id,
            config_id,
            image_id,
            status: ContainerStatus::Created,
            health_status: None,
            restart_count: 0,
            last_updated: now,
            started_at: None,
            finished_at: None,
            exit_code: None,
            error_message: None,
            host_port_mappings: HashMap::new(),
            network_settings: HashMap::new(),
            security_context: HashMap::new(),
        };

        let mut states = self.container_states.write().await;
        states.insert(state_id, state);

        Ok(state_id)
    }

    /// Get container state by ID
    pub async fn get_container_state(&self, state_id: uuid::Uuid) -> Result<Option<ContainerState>, DockerConfigError> {
        let states = self.container_states.read().await;
        Ok(states.get(&state_id).cloned())
    }

    /// Update container status
    pub async fn update_container_status(&self, state_id: uuid::Uuid, status: ContainerStatus) -> Result<bool, DockerConfigError> {
        let mut states = self.container_states.write().await;
        if let Some(state) = states.get_mut(&state_id) {
            state.status = status.clone();
            state.last_updated = chrono::Utc::now();

            // Update timestamps based on status
            match status {
                ContainerStatus::Running => {
                    if state.started_at.is_none() {
                        state.started_at = Some(chrono::Utc::now());
                    }
                }
                ContainerStatus::Exited | ContainerStatus::Failed => {
                    if state.finished_at.is_none() {
                        state.finished_at = Some(chrono::Utc::now());
                    }
                }
                _ => {}
            }

            // Log status change event
            self.log_container_event(state_id, format!("Status changed to {:?}", status), "status_change").await?;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Start a container
    pub async fn start_container(&self, state_id: uuid::Uuid) -> Result<bool, DockerConfigError> {
        let success = self.update_container_status(state_id, ContainerStatus::Running).await?;
        if success {
            self.log_container_event(state_id, "Container started".to_string(), "start").await?;
        }
        Ok(success)
    }

    /// Stop a container
    pub async fn stop_container(&self, state_id: uuid::Uuid) -> Result<bool, DockerConfigError> {
        let success = self.update_container_status(state_id, ContainerStatus::Stopped).await?;
        if success {
            self.log_container_event(state_id, "Container stopped".to_string(), "stop").await?;
        }
        Ok(success)
    }

    /// Restart a container
    pub async fn restart_container(&self, state_id: uuid::Uuid) -> Result<bool, DockerConfigError> {
        let mut states = self.container_states.write().await;
        if let Some(state) = states.get_mut(&state_id) {
            state.restart_count += 1;
            state.last_updated = chrono::Utc::now();
        }
        drop(states);

        self.log_container_event(state_id, format!("Container restarted (restart count: {})", {
            let states = self.container_states.read().await;
            states.get(&state_id).map(|s| s.restart_count).unwrap_or(0)
        }), "restart").await?;

        // Start the container again
        self.start_container(state_id).await
    }

    /// Remove a container
    pub async fn remove_container(&self, state_id: uuid::Uuid) -> Result<bool, DockerConfigError> {
        let mut states = self.container_states.write().await;
        let mut metrics = self.container_metrics.write().await;
        let mut events = self.container_events.write().await;
        let mut deployments = self.active_deployments.write().await;

        let removed = states.remove(&state_id).is_some();
        metrics.remove(&state_id);
        events.remove(&state_id);
        deployments.remove(&state_id);

        if removed {
            self.log_container_event(state_id, "Container removed".to_string(), "remove").await?;
        }

        Ok(removed)
    }

    /// Update container metrics
    pub async fn update_metrics(&self, state_id: uuid::Uuid, metrics: ContainerMetrics) -> Result<bool, DockerConfigError> {
        let mut container_metrics = self.container_metrics.write().await;
        container_metrics.insert(state_id, metrics.clone());
        Ok(true)
    }

    /// Get container metrics
    pub async fn get_metrics(&self, state_id: uuid::Uuid) -> Result<Option<ContainerMetrics>, DockerConfigError> {
        let metrics = self.container_metrics.read().await;
        Ok(metrics.get(&state_id).cloned())
    }

    /// Log container event
    pub async fn log_container_event(&self, state_id: uuid::Uuid, message: String, event_type: &str) -> Result<bool, DockerConfigError> {
        let event = ContainerEvent {
            id: uuid::Uuid::new_v4(),
            container_id: state_id,
            event_type: event_type.to_string(),
            message,
            timestamp: chrono::Utc::now(),
            level: "info".to_string(),
            details: HashMap::new(),
        };

        let mut events = self.container_events.write().await;
        events.entry(state_id)
            .or_insert_with(Vec::new)
            .push(event);

        Ok(true)
    }

    /// Get container events
    pub async fn get_container_events(&self, state_id: uuid::Uuid) -> Result<Vec<ContainerEvent>, DockerConfigError> {
        let events = self.container_events.read().await;
        Ok(events.get(&state_id).cloned().unwrap_or_else(Vec::new))
    }

    /// Get all container states
    pub async fn list_container_states(&self) -> Vec<ContainerState> {
        let states = self.container_states.read().await;
        states.values().cloned().collect()
    }

    /// Get containers by status
    pub async fn get_containers_by_status(&self, status: ContainerStatus) -> Vec<ContainerState> {
        let states = self.container_states.read().await;
        states.values()
            .filter(|state| state.status == status)
            .cloned()
            .collect()
    }

    /// Get active deployment info
    pub async fn get_deployment_info(&self, state_id: uuid::Uuid) -> Result<Option<String>, DockerConfigError> {
        let deployments = self.active_deployments.read().await;
        Ok(deployments.get(&state_id).cloned())
    }

    /// Register deployment
    pub async fn register_deployment(&self, state_id: uuid::Uuid, deployment_name: String) -> Result<bool, DockerConfigError> {
        let mut deployments = self.active_deployments.write().await;
        deployments.insert(state_id, deployment_name);
        Ok(true)
    }

    /// Get container health summary
    pub async fn get_health_summary(&self) -> serde_json::Value {
        let states = self.container_states.read().await;
        let metrics = self.container_metrics.read().await;

        let total_containers = states.len();
        let running_containers = states.values().filter(|s| s.status == ContainerStatus::Running).count();
        let failed_containers = states.values().filter(|s| s.status == ContainerStatus::Failed).count();

        let total_memory_mb: f64 = metrics.values()
            .map(|m| m.memory_usage_mb as f64)
            .sum();

        let total_cpu_percent: f64 = metrics.values()
            .map(|m| m.cpu_usage_percent)
            .sum();

        json!({
            "total_containers": total_containers,
            "running_containers": running_containers,
            "failed_containers": failed_containers,
            "healthy_containers": running_containers,
            "total_memory_usage_mb": total_memory_mb,
            "total_cpu_usage_percent": total_cpu_percent,
            "average_memory_mb": if total_containers > 0 { total_memory_mb / total_containers as f64 } else { 0.0 },
            "average_cpu_percent": if total_containers > 0 { total_cpu_percent / total_containers as f64 } else { 0.0 },
            "timestamp": chrono::Utc::now().to_rfc3339()
        })
    }

    /// Warp filter for container status endpoint
    pub fn status_endpoint(&self) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("api" / "v1" / "containers" / "status")
            .and(warp::get())
            .and(with_service(self.clone()))
            .and_then(handle_container_status)
    }

    /// Warp filter for container metrics endpoint
    pub fn metrics_endpoint(&self) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("api" / "v1" / "containers" / "metrics")
            .and(warp::get())
            .and(with_service(self.clone()))
            .and_then(handle_container_metrics)
    }

    /// Warp filter for container events endpoint
    pub fn events_endpoint(&self) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("api" / "v1" / "containers" / "events")
            .and(warp::get())
            .and(warp::query::<ContainerEventsQuery>())
            .and(with_service(self.clone()))
            .and_then(handle_container_events)
    }

    /// Warp filter for container operations endpoint
    pub fn operations_endpoint(&self) -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
        warp::path!("api" / "v1" / "containers" / String)
            .and(warp::post())
            .and(warp::body::json())
            .and(with_service(self.clone()))
            .and_then(handle_container_operation)
    }
}

/// Query parameters for container events
#[derive(serde::Deserialize)]
struct ContainerEventsQuery {
    container_id: Option<uuid::Uuid>,
    limit: Option<usize>,
    event_type: Option<String>,
}

/// Helper function to pass service to warp handlers
fn with_service(
    service: ContainerStateService,
) -> impl Filter<Extract = (ContainerStateService,), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || service.clone())
}

/// Handle container status requests
async fn handle_container_status(
    service: ContainerStateService,
) -> Result<impl Reply, Rejection> {
    let states = service.list_container_states().await;
    let response = json!({
        "containers": states,
        "summary": service.get_health_summary().await,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    Ok(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::OK))
}

/// Handle container metrics requests
async fn handle_container_metrics(
    service: ContainerStateService,
) -> Result<impl Reply, Rejection> {
    let metrics = service.get_health_summary().await;
    let response = json!({
        "metrics": metrics,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    Ok(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::OK))
}

/// Handle container events requests
async fn handle_container_events(
    query: ContainerEventsQuery,
    service: ContainerStateService,
) -> Result<impl Reply, Rejection> {
    let events = if let Some(container_id) = query.container_id {
        service.get_container_events(container_id).await.unwrap_or_else(|_| Vec::new())
    } else {
        // Return events from all containers
        let all_events = service.container_events.read().await;
        all_events.values()
            .flat_map(|events| events.clone())
            .collect()
    };

    let mut filtered_events = events;
    if let Some(event_type_filter) = query.event_type {
        filtered_events = filtered_events
            .into_iter()
            .filter(|event| event.event_type == event_type_filter)
            .collect();
    }

    // Sort by timestamp (newest first)
    filtered_events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    // Apply limit
    if let Some(limit) = query.limit {
        filtered_events.truncate(limit);
    }

    let response = json!({
        "events": filtered_events,
        "total_count": filtered_events.len(),
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    Ok(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::OK))
}

/// Handle container operation requests
async fn handle_container_operation(
    container_id: String,
    operation: serde_json::Value,
    service: ContainerStateService,
) -> Result<impl Reply, Rejection> {
    let state_id = match uuid::Uuid::parse_str(&container_id) {
        Ok(id) => id,
        Err(_) => {
            let response = json!({
                "status": "error",
                "message": "Invalid container ID format"
            });
            return Ok(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::BAD_REQUEST));
        }
    };

    let operation_type = operation.get("operation")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let result = match operation_type {
        "start" => service.start_container(state_id).await,
        "stop" => service.stop_container(state_id).await,
        "restart" => service.restart_container(state_id).await,
        "remove" => service.remove_container(state_id).await,
        _ => Err(DockerConfigError::ValidationError(format!("Unknown operation: {}", operation_type))),
    };

    match result {
        Ok(success) => {
            let response = json!({
                "status": if success { "success" } else { "failed" },
                "operation": operation_type,
                "container_id": container_id,
                "timestamp": chrono::Utc::now().to_rfc3339()
            });
            Ok(warp::reply::with_status(warp::reply::json(&response), if success {
                warp::http::StatusCode::OK
            } else {
                warp::http::StatusCode::NOT_FOUND
            }))
        }
        Err(e) => {
            let response = json!({
                "status": "error",
                "operation": operation_type,
                "message": e.to_string(),
                "container_id": container_id
            });
            Ok(warp::reply::with_status(warp::reply::json(&response), warp::http::StatusCode::BAD_REQUEST))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_new_service() {
        let service = ContainerStateService::new();
        assert!(service.is_ok());
    }

    #[tokio::test]
    async fn test_create_container_state() {
        let service = ContainerStateService::new().unwrap();
        let config_id = uuid::Uuid::new_v4();
        let image_id = "test:latest".to_string();

        let result = service.create_container_state(config_id, image_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_update_container_status() {
        let service = ContainerStateService::new().unwrap();
        let config_id = uuid::Uuid::new_v4();
        let image_id = "test:latest".to_string();

        let state_id = service.create_container_state(config_id, image_id).await.unwrap();
        let result = service.update_container_status(state_id, ContainerStatus::Running).await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_get_container_state() {
        let service = ContainerStateService::new().unwrap();
        let config_id = uuid::Uuid::new_v4();
        let image_id = "test:latest".to_string();

        let state_id = service.create_container_state(config_id, image_id).await.unwrap();
        let state = service.get_container_state(state_id).await.unwrap();

        assert!(state.is_some());
        assert_eq!(state.unwrap().config_id, config_id);
    }

    #[tokio::test]
    async fn test_log_container_event() {
        let service = ContainerStateService::new().unwrap();
        let config_id = uuid::Uuid::new_v4();
        let image_id = "test:latest".to_string();

        let state_id = service.create_container_state(config_id, image_id).await.unwrap();
        let result = service.log_container_event(state_id, "Test event".to_string(), "test").await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_get_container_events() {
        let service = ContainerStateService::new().unwrap();
        let config_id = uuid::Uuid::new_v4();
        let image_id = "test:latest".to_string();

        let state_id = service.create_container_state(config_id, image_id).await.unwrap();
        service.log_container_event(state_id, "Test event".to_string(), "test").await.unwrap();

        let events = service.get_container_events(state_id).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].message, "Test event");
    }

    #[tokio::test]
    async fn test_get_containers_by_status() {
        let service = ContainerStateService::new().unwrap();
        let config_id = uuid::Uuid::new_v4();
        let image_id = "test:latest".to_string();

        let state_id = service.create_container_state(config_id, image_id).await.unwrap();
        service.update_container_status(state_id, ContainerStatus::Running).await.unwrap();

        let running_containers = service.get_containers_by_status(ContainerStatus::Running).await;
        assert_eq!(running_containers.len(), 1);
        assert_eq!(running_containers[0].status, ContainerStatus::Running);
    }

    #[tokio::test]
    async fn test_health_summary() {
        let service = ContainerStateService::new().unwrap();
        let config_id = uuid::Uuid::new_v4();
        let image_id = "test:latest".to_string();

        let state_id = service.create_container_state(config_id, image_id).await.unwrap();
        service.update_container_status(state_id, ContainerStatus::Running).await.unwrap();

        let summary = service.get_health_summary().await;
        assert_eq!(summary["total_containers"], 1);
        assert_eq!(summary["running_containers"], 1);
        assert_eq!(summary["failed_containers"], 0);
    }
}