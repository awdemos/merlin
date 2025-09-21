use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{data_models::*, storage::FeatureStorage, error::Result, FeatureNumberingError};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateFeatureRequest {
    pub name: String,
    pub description: String,
    pub metadata: Option<FeatureMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateFeatureRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub metadata: Option<FeatureMetadata>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateStatusRequest {
    pub status: FeatureStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReserveNumberRequest {
    pub number: u32,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListFeaturesQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub status: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub field: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct FeatureListResponse {
    pub features: Vec<Feature>,
    pub pagination: Pagination,
}

#[derive(Debug, Serialize)]
pub struct Pagination {
    pub page: u32,
    pub limit: u32,
    pub total: u32,
    pub pages: u32,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<Feature>,
    pub total: u32,
}

#[derive(Debug, Serialize)]
pub struct NextNumberResponse {
    pub next_number: u32,
    pub prefix: String,
    pub formatted: String,
}

#[derive(Debug, Serialize)]
pub struct ReservedNumberResponse {
    pub reserved_numbers: Vec<ReservedNumberInfo>,
}

#[derive(Debug, Serialize)]
pub struct ReservedNumberInfo {
    pub number: u32,
    pub reason: Option<String>,
    pub reserved_at: Option<chrono::DateTime<chrono::Utc>>,
}

// Shared state for the API
#[derive(Clone)]
pub struct FeatureApiState {
    pub storage: Arc<RwLock<FeatureStorage>>,
}

impl FeatureApiState {
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(FeatureStorage::new())),
        }
    }

    pub async fn load_storage<P: AsRef<std::path::Path>>(&mut self, path: P) -> Result<()> {
        let storage = FeatureStorage::load(path)?;
        *self.storage.write().await = storage;
        Ok(())
    }

    pub async fn save_storage<P: AsRef<std::path::Path>>(&self, path: P) -> Result<()> {
        self.storage.read().await.save(path)?;
        Ok(())
    }
}

// API Handlers
pub async fn create_feature(
    State(state): State<FeatureApiState>,
    Json(request): Json<CreateFeatureRequest>,
) -> Result<impl IntoResponse> {
    let mut storage = state.storage.write().await;
    let mut feature = storage.create_feature(request.name, request.description)?;

    if let Some(metadata) = request.metadata {
        feature.metadata = Some(metadata);
        storage.update_feature(&feature.id, feature.clone())?;
    }

    Ok((StatusCode::CREATED, Json(feature)))
}

pub async fn get_feature(
    State(state): State<FeatureApiState>,
    Path(feature_id): Path<String>,
) -> Result<impl IntoResponse> {
    let storage = state.storage.read().await;

    match storage.get_feature(&feature_id) {
        Some(feature) => Ok(Json(feature.clone()).into_response()),
        None => Err(FeatureNumberingError::FeatureNotFound(feature_id)),
    }
}

pub async fn list_features(
    State(state): State<FeatureApiState>,
    Query(params): Query<ListFeaturesQuery>,
) -> Result<impl IntoResponse> {
    let storage = state.storage.read().await;
    let all_features = storage.list_features();

    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(20).min(100);

    // Filter features based on query parameters
    let filtered_features: Vec<Feature> = all_features
        .into_iter()
        .filter(|feature| {
            if let Some(ref status_filter) = params.status {
                if format!("{:?}", feature.status) != *status_filter {
                    return false;
                }
            }

            if let Some(ref search_term) = params.search {
                let search_lower = search_term.to_lowercase();
                if !feature.name.to_lowercase().contains(&search_lower) &&
                   !feature.description.to_lowercase().contains(&search_lower) {
                    return false;
                }
            }

            true
        })
        .cloned()
        .collect();

    let total = filtered_features.len() as u32;
    let pages = (total + limit - 1) / limit;

    let paginated_features = filtered_features
        .into_iter()
        .skip(((page - 1) * limit) as usize)
        .take(limit as usize)
        .collect();

    Ok(Json(FeatureListResponse {
        features: paginated_features,
        pagination: Pagination {
            page,
            limit,
            total,
            pages,
        },
    }))
}

pub async fn update_feature(
    State(state): State<FeatureApiState>,
    Path(feature_id): Path<String>,
    Json(request): Json<UpdateFeatureRequest>,
) -> Result<impl IntoResponse> {
    let mut storage = state.storage.write().await;

    let existing_feature = storage.get_feature(&feature_id)
        .ok_or_else(|| FeatureNumberingError::FeatureNotFound(feature_id.clone()))?
        .clone();

    let mut updated_feature = existing_feature.clone();

    if let Some(name) = request.name {
        updated_feature.name = name;
    }

    if let Some(description) = request.description {
        updated_feature.description = description;
    }

    if let Some(metadata) = request.metadata {
        updated_feature.metadata = Some(metadata);
    }

    storage.update_feature(&feature_id, updated_feature)?;

    Ok(Json(storage.get_feature(&feature_id).unwrap().clone()))
}

pub async fn delete_feature(
    State(state): State<FeatureApiState>,
    Path(feature_id): Path<String>,
) -> Result<impl IntoResponse> {
    let mut storage = state.storage.write().await;
    storage.delete_feature(&feature_id)?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn update_feature_status(
    State(state): State<FeatureApiState>,
    Path(feature_id): Path<String>,
    Json(request): Json<UpdateStatusRequest>,
) -> Result<impl IntoResponse> {
    let mut storage = state.storage.write().await;

    let mut feature = storage.get_feature(&feature_id)
        .ok_or_else(|| FeatureNumberingError::FeatureNotFound(feature_id.clone()))?
        .clone();

    feature.update_status(request.status)?;
    storage.update_feature(&feature_id, feature)?;

    Ok(Json(storage.get_feature(&feature_id).unwrap().clone()))
}

pub async fn get_next_number(
    State(state): State<FeatureApiState>,
) -> Result<impl IntoResponse> {
    let mut storage = state.storage.write().await;
    let next_number = storage.get_next_available_number();

    Ok(Json(NextNumberResponse {
        next_number,
        prefix: "".to_string(),
        formatted: format!("{:03}", next_number),
    }))
}

pub async fn reserve_number(
    State(state): State<FeatureApiState>,
    Json(request): Json<ReserveNumberRequest>,
) -> Result<impl IntoResponse> {
    let mut storage = state.storage.write().await;
    storage.reserve_number(request.number, request.reason)?;
    Ok(StatusCode::CREATED)
}

pub async fn list_reserved_numbers(
    State(state): State<FeatureApiState>,
) -> Result<impl IntoResponse> {
    let storage = state.storage.read().await;
    let reserved_features = storage.get_reserved_numbers();

    let reserved_info: Vec<ReservedNumberInfo> = reserved_features
        .into_iter()
        .map(|fn_| ReservedNumberInfo {
            number: fn_.value,
            reason: None, // TODO: Store reason with reserved numbers
            reserved_at: None, // TODO: Track reservation time
        })
        .collect();

    Ok(Json(ReservedNumberResponse {
        reserved_numbers: reserved_info,
    }))
}

pub async fn search_features(
    State(state): State<FeatureApiState>,
    Query(params): Query<SearchQuery>,
) -> Result<impl IntoResponse> {
    let storage = state.storage.read().await;
    let all_features = storage.list_features();

    let search_term = params.q.to_lowercase();
    let field = params.field.as_deref().unwrap_or("name");
    let limit = params.limit.unwrap_or(10).min(50);

    let matching_features: Vec<Feature> = all_features
        .into_iter()
        .filter(|feature| {
            let matches = match field {
                "name" => feature.name.to_lowercase().contains(&search_term),
                "description" => feature.description.to_lowercase().contains(&search_term),
                "tags" => {
                    if let Some(ref metadata) = feature.metadata {
                        metadata.tags.iter().any(|tag|
                            tag.to_lowercase().contains(&search_term)
                        )
                    } else {
                        false
                    }
                },
                "assignee" => {
                    if let Some(ref metadata) = feature.metadata {
                        metadata.assignee.as_ref().map_or(false, |assignee|
                            assignee.to_lowercase().contains(&search_term)
                        )
                    } else {
                        false
                    }
                },
                _ => false,
            };
            matches
        })
        .cloned()
        .take(limit as usize)
        .collect();

    let total = matching_features.len() as u32;
    Ok(Json(SearchResponse {
        results: matching_features,
        total,
    }))
}

// Error handler
pub async fn handle_api_error(err: FeatureNumberingError) -> impl IntoResponse {
    let status = match err {
        FeatureNumberingError::FeatureNotFound(_) => StatusCode::NOT_FOUND,
        FeatureNumberingError::NumberAlreadyAssigned(_) => StatusCode::CONFLICT,
        FeatureNumberingError::InvalidStatusTransition => StatusCode::BAD_REQUEST,
        FeatureNumberingError::ValidationError(_) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };

    (status, Json(serde_json::json!({
        "error": "FeatureNumberingError",
        "message": err.to_string()
    })))
}

pub fn create_router() -> axum::Router<FeatureApiState> {
    use axum::routing::*;

    Router::new()
        .route("/api/v1/features", get(list_features_handler).post(create_feature_handler))
        .route("/api/v1/features/:id", get(get_feature_handler).put(update_feature_handler).delete(delete_feature_handler))
        .route("/api/v1/features/:id/status", patch(update_feature_status_handler))
        .route("/api/v1/numbers/next", get(get_next_number_handler))
        .route("/api/v1/numbers/reserved", get(list_reserved_numbers_handler).post(reserve_number_handler))
        .route("/api/v1/search", get(search_features_handler))
}

// Wrapper handlers to satisfy axum trait bounds
async fn list_features_handler(
    state: axum::extract::State<FeatureApiState>,
    query: axum::extract::Query<ListFeaturesQuery>,
) -> impl axum::response::IntoResponse {
    list_features(state, query).await
}

async fn create_feature_handler(
    state: axum::extract::State<FeatureApiState>,
    json: axum::Json<CreateFeatureRequest>,
) -> impl axum::response::IntoResponse {
    create_feature(state, json).await
}

async fn get_feature_handler(
    state: axum::extract::State<FeatureApiState>,
    Path(feature_id): Path<String>,
) -> impl axum::response::IntoResponse {
    get_feature(state, Path(feature_id)).await
}

async fn update_feature_handler(
    state: axum::extract::State<FeatureApiState>,
    Path(feature_id): Path<String>,
    json: axum::Json<UpdateFeatureRequest>,
) -> impl axum::response::IntoResponse {
    update_feature(state, Path(feature_id), json).await
}

async fn delete_feature_handler(
    state: axum::extract::State<FeatureApiState>,
    Path(feature_id): Path<String>,
) -> impl axum::response::IntoResponse {
    delete_feature(state, Path(feature_id)).await
}

async fn update_feature_status_handler(
    state: axum::extract::State<FeatureApiState>,
    Path(feature_id): Path<String>,
    json: axum::Json<UpdateStatusRequest>,
) -> impl axum::response::IntoResponse {
    update_feature_status(state, Path(feature_id), json).await
}

async fn get_next_number_handler(
    state: axum::extract::State<FeatureApiState>,
) -> impl axum::response::IntoResponse {
    get_next_number(state).await
}

async fn reserve_number_handler(
    state: axum::extract::State<FeatureApiState>,
    json: axum::Json<ReserveNumberRequest>,
) -> impl axum::response::IntoResponse {
    reserve_number(state, json).await
}

async fn list_reserved_numbers_handler(
    state: axum::extract::State<FeatureApiState>,
) -> impl axum::response::IntoResponse {
    list_reserved_numbers(state).await
}

async fn search_features_handler(
    state: axum::extract::State<FeatureApiState>,
    query: axum::extract::Query<SearchQuery>,
) -> impl axum::response::IntoResponse {
    search_features(state, query).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_state_creation() {
        let state = FeatureApiState::new();
        assert!(state.storage.read().await.list_features().is_empty());
    }
}