// src/server/ab_testing.rs
use axum::{
    extract::{State, Path},
    http::StatusCode,
    response::Json,
    routing::{get, post, put, delete},
    Router,
};
use crate::api::ab_testing::*;
use crate::server::AppState;

pub fn create_ab_testing_routes() -> Router<AppState> {
    Router::new()
        .route("/experiments", post(create_experiment))
        .route("/experiments", get(list_experiments))
        .route("/experiments/:id", get(get_experiment))
        .route("/experiments/:id", put(update_experiment))
        .route("/experiments/:id", delete(delete_experiment))
        .route("/experiments/:id/assign", post(assign_user))
        .route("/experiments/:id/metrics", post(record_metrics))
        .route("/experiments/:id/results", get(get_results))
        .route("/experiments/:id/start", post(start_experiment))
        .route("/experiments/:id/pause", post(pause_experiment))
        .route("/experiments/:id/complete", post(complete_experiment))
}

// === Experiment CRUD Operations ===

async fn create_experiment(
    State(app_state): State<AppState>,
    Json(request): Json<CreateExperimentRequest>,
) -> Result<Json<ExperimentResponse>, (StatusCode, Json<crate::server::ErrorResponse>)> {
    // Validate request
    if request.name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(crate::server::ErrorResponse {
                error: "Experiment name cannot be empty".to_string(),
            }),
        ));
    }

    if request.variants.len() < 2 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(crate::server::ErrorResponse {
                error: "At least 2 variants are required".to_string(),
            }),
        ));
    }

    if request.traffic_allocation <= 0.0 || request.traffic_allocation > 1.0 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(crate::server::ErrorResponse {
                error: "Traffic allocation must be between 0.0 and 1.0".to_string(),
            }),
        ));
    }

    // Convert request to experiment config
    let experiment_config: crate::ab_testing::config::ExperimentConfig = request.into();

    // Validate the experiment
    if let Err(validation_error) = experiment_config.validate() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(crate::server::ErrorResponse {
                error: format!("Experiment validation failed: {}", validation_error),
            }),
        ));
    }

    // Get experiment runner from state
    let mut runner = app_state.experiment_runner.lock().await;

    // Create experiment
    match crate::ab_testing::experiment::Experiment::new(experiment_config.clone()) {
        Ok(experiment) => {
            // Store experiment
            runner.experiments.insert(experiment_config.id.clone(), experiment);

            // Save to storage
            if let Err(storage_error) = runner.storage.save_experiment_config(&experiment_config).await {
                tracing::error!("Failed to save experiment to storage: {}", storage_error);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(crate::server::ErrorResponse {
                        error: format!("Failed to save experiment: {}", storage_error),
                    }),
                ));
            }

            Ok(Json(ExperimentResponse {
                success: true,
                experiment: Some(experiment_config),
                message: "Experiment created successfully".to_string(),
            }))
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(crate::server::ErrorResponse {
                error: format!("Failed to create experiment: {}", e),
            }),
        )),
    }
}

async fn list_experiments(
    State(app_state): State<AppState>,
) -> Result<Json<ExperimentsListResponse>, (StatusCode, Json<crate::server::ErrorResponse>)> {
    let runner = app_state.experiment_runner.lock().await;

    let experiments: Vec<crate::ab_testing::config::ExperimentConfig> = runner.experiments
        .values()
        .map(|exp| exp.config.clone())
        .collect();

    Ok(Json(ExperimentsListResponse {
        success: true,
        experiments,
        message: "Experiments retrieved successfully".to_string(),
    }))
}

async fn get_experiment(
    State(app_state): State<AppState>,
    Path(experiment_id): Path<String>,
) -> Result<Json<ExperimentResponse>, (StatusCode, Json<crate::server::ErrorResponse>)> {
    let runner = app_state.experiment_runner.lock().await;

    match runner.experiments.get(&experiment_id) {
        Some(experiment) => Ok(Json(ExperimentResponse {
            success: true,
            experiment: Some(experiment.config.clone()),
            message: "Experiment retrieved successfully".to_string(),
        })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(crate::server::ErrorResponse {
                error: "Experiment not found".to_string(),
            }),
        )),
    }
}

async fn update_experiment(
    State(app_state): State<AppState>,
    Path(experiment_id): Path<String>,
    Json(request): Json<UpdateExperimentRequest>,
) -> Result<Json<ExperimentResponse>, (StatusCode, Json<crate::server::ErrorResponse>)> {
    let mut runner = app_state.experiment_runner.lock().await;

    let experiment_config = match runner.experiments.get_mut(&experiment_id) {
        Some(exp) => {
            // Update fields
            if let Some(name) = &request.name {
                exp.config.name = name.clone();
            }
            if let Some(description) = &request.description {
                exp.config.description = description.clone();
            }
            if let Some(traffic_allocation) = request.traffic_allocation {
                exp.config.traffic_allocation = traffic_allocation;
            }
            if let Some(status_str) = &request.status {
                if let Some(status) = string_to_experiment_status(status_str) {
                    exp.config.status = status;
                } else {
                    return Err((
                        StatusCode::BAD_REQUEST,
                        Json(crate::server::ErrorResponse {
                            error: "Invalid experiment status".to_string(),
                        }),
                    ));
                }
            }

            // Update timestamp
            exp.config.updated_at = chrono::Utc::now();
            exp.config.clone()
        }
        None => return Err((
            StatusCode::NOT_FOUND,
            Json(crate::server::ErrorResponse {
                error: "Experiment not found".to_string(),
            }),
        )),
    };

    // Save to storage
    if let Err(storage_error) = runner.storage.update_experiment_config(&experiment_config).await {
        tracing::error!("Failed to update experiment in storage: {}", storage_error);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(crate::server::ErrorResponse {
                error: format!("Failed to update experiment: {}", storage_error),
            }),
        ));
    }

    Ok(Json(ExperimentResponse {
        success: true,
        experiment: Some(experiment_config),
        message: "Experiment updated successfully".to_string(),
    }))
}

async fn delete_experiment(
    State(app_state): State<AppState>,
    Path(experiment_id): Path<String>,
) -> Result<Json<ExperimentResponse>, (StatusCode, Json<crate::server::ErrorResponse>)> {
    let mut runner = app_state.experiment_runner.lock().await;

    // Remove from memory
    let removed = runner.experiments.remove(&experiment_id).is_some();

    // Remove from storage
    let storage_removed = runner.storage.delete_experiment_config(&experiment_id).await.unwrap_or(false);

    Ok(Json(ExperimentResponse {
        success: removed || storage_removed,
        experiment: None,
        message: if removed || storage_removed {
            "Experiment deleted successfully".to_string()
        } else {
            "Experiment not found".to_string()
        },
    }))
}

// === Experiment Lifecycle Management ===

async fn start_experiment(
    State(app_state): State<AppState>,
    Path(experiment_id): Path<String>,
) -> Result<Json<ExperimentResponse>, (StatusCode, Json<crate::server::ErrorResponse>)> {
    let mut runner = app_state.experiment_runner.lock().await;

    let experiment_config = match runner.experiments.get_mut(&experiment_id) {
        Some(exp) => {
            exp.config.status = crate::ab_testing::config::ExperimentStatus::Running;
            exp.config.updated_at = chrono::Utc::now();
            exp.config.clone()
        }
        None => return Err((
            StatusCode::NOT_FOUND,
            Json(crate::server::ErrorResponse {
                error: "Experiment not found".to_string(),
            }),
        )),
    };

    // Save to storage
    if let Err(storage_error) = runner.storage.update_experiment_config(&experiment_config).await {
        tracing::error!("Failed to start experiment in storage: {}", storage_error);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(crate::server::ErrorResponse {
                error: format!("Failed to start experiment: {}", storage_error),
            }),
        ));
    }

    Ok(Json(ExperimentResponse {
        success: true,
        experiment: Some(experiment_config),
        message: "Experiment started successfully".to_string(),
    }))
}

async fn pause_experiment(
    State(app_state): State<AppState>,
    Path(experiment_id): Path<String>,
) -> Result<Json<ExperimentResponse>, (StatusCode, Json<crate::server::ErrorResponse>)> {
    let mut runner = app_state.experiment_runner.lock().await;

    let experiment_config = match runner.experiments.get_mut(&experiment_id) {
        Some(exp) => {
            exp.config.status = crate::ab_testing::config::ExperimentStatus::Paused;
            exp.config.updated_at = chrono::Utc::now();
            exp.config.clone()
        }
        None => return Err((
            StatusCode::NOT_FOUND,
            Json(crate::server::ErrorResponse {
                error: "Experiment not found".to_string(),
            }),
        )),
    };

    // Save to storage
    if let Err(storage_error) = runner.storage.update_experiment_config(&experiment_config).await {
        tracing::error!("Failed to pause experiment in storage: {}", storage_error);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(crate::server::ErrorResponse {
                error: format!("Failed to pause experiment: {}", storage_error),
            }),
        ));
    }

    Ok(Json(ExperimentResponse {
        success: true,
        experiment: Some(experiment_config),
        message: "Experiment paused successfully".to_string(),
    }))
}

async fn complete_experiment(
    State(app_state): State<AppState>,
    Path(experiment_id): Path<String>,
) -> Result<Json<ExperimentResponse>, (StatusCode, Json<crate::server::ErrorResponse>)> {
    let mut runner = app_state.experiment_runner.lock().await;

    let experiment_config = match runner.experiments.get_mut(&experiment_id) {
        Some(exp) => {
            exp.config.status = crate::ab_testing::config::ExperimentStatus::Completed;
            exp.config.updated_at = chrono::Utc::now();
            exp.config.clone()
        }
        None => return Err((
            StatusCode::NOT_FOUND,
            Json(crate::server::ErrorResponse {
                error: "Experiment not found".to_string(),
            }),
        )),
    };

    // Save to storage
    if let Err(storage_error) = runner.storage.update_experiment_config(&experiment_config).await {
        tracing::error!("Failed to complete experiment in storage: {}", storage_error);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(crate::server::ErrorResponse {
                error: format!("Failed to complete experiment: {}", storage_error),
            }),
        ));
    }

    Ok(Json(ExperimentResponse {
        success: true,
        experiment: Some(experiment_config),
        message: "Experiment completed successfully".to_string(),
    }))
}

// === User Assignment ===

async fn assign_user(
    State(app_state): State<AppState>,
    Path(experiment_id): Path<String>,
    Json(request): Json<UserAssignmentRequest>,
) -> Result<Json<UserAssignmentResponse>, (StatusCode, Json<crate::server::ErrorResponse>)> {
    if request.user_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(crate::server::ErrorResponse {
                error: "User ID cannot be empty".to_string(),
            }),
        ));
    }

    if request.experiment_id != experiment_id {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(crate::server::ErrorResponse {
                error: "Experiment ID in path and body must match".to_string(),
            }),
        ));
    }

    let mut runner = app_state.experiment_runner.lock().await;

    match runner.get_variant_for_user(&experiment_id, &request.user_id) {
        Some(variant) => Ok(Json(UserAssignmentResponse {
            success: true,
            variant_id: Some(variant.config.id.clone()),
            variant_name: Some(variant.config.name.clone()),
            experiment_id,
            message: "User assigned to variant successfully".to_string(),
        })),
        None => Err((
            StatusCode::NOT_FOUND,
            Json(crate::server::ErrorResponse {
                error: "Experiment not found or user not eligible".to_string(),
            }),
        )),
    }
}

// === Metrics Recording ===

async fn record_metrics(
    State(app_state): State<AppState>,
    Path(experiment_id): Path<String>,
    Json(request): Json<RecordMetricsRequest>,
) -> Result<Json<RecordMetricsResponse>, (StatusCode, Json<crate::server::ErrorResponse>)> {
    // Validate request
    if request.user_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(crate::server::ErrorResponse {
                error: "User ID cannot be empty".to_string(),
            }),
        ));
    }

    if request.variant_id.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(crate::server::ErrorResponse {
                error: "Variant ID cannot be empty".to_string(),
            }),
        ));
    }

    // Clone needed fields before converting to metrics
    let user_id = request.user_id.clone();

    // Convert request to interaction metrics
    let metrics: crate::ab_testing::experiment::InteractionMetrics = request.into();

    let mut runner = app_state.experiment_runner.lock().await;
    runner.record_interaction(&experiment_id, &user_id, &metrics);

    // Save results to storage
    if let Err(storage_error) = runner.save_results().await {
        tracing::error!("Failed to save experiment results: {}", storage_error);
        // Don't fail the request, just log the error
    }

    Ok(Json(RecordMetricsResponse {
        success: true,
        message: "Metrics recorded successfully".to_string(),
    }))
}

// === Results Retrieval ===

async fn get_results(
    State(app_state): State<AppState>,
    Path(experiment_id): Path<String>,
) -> Result<Json<ExperimentResultsResponse>, (StatusCode, Json<crate::server::ErrorResponse>)> {
    let runner = app_state.experiment_runner.lock().await;

    match runner.experiments.get(&experiment_id) {
        Some(experiment) => {
            let results = experiment.get_results();
            Ok(Json(ExperimentResultsResponse {
                success: true,
                results: Some(results),
                message: "Results retrieved successfully".to_string(),
            }))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(crate::server::ErrorResponse {
                error: "Experiment not found".to_string(),
            }),
        )),
    }
}