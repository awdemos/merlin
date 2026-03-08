use crate::api::preferences::{CreateUserPreferenceRequest, UpdateUserPreferenceRequest};
use crate::models::{PreferenceCategory, PreferenceDeleteResponse, UserPreferenceResponse};
use serde_json::json;
use warp::Filter;

pub fn preference_create_endpoint(
) -> impl Filter<Extract = (warp::reply::Json,), Error = warp::Rejection> + Clone {
    warp::path!("api" / "v1" / "preferences" / "userPreferenceCreate")
        .and(warp::post())
        .and(warp::body::json())
        .map(|_req: CreateUserPreferenceRequest| {
            let response = UserPreferenceResponse::new(
                "user123".to_string(),
                "test_preference".to_string(),
                json!("test_value"),
                PreferenceCategory::ModelSelection,
            );
            warp::reply::json(&response)
        })
}

pub fn preference_update_endpoint(
) -> impl Filter<Extract = (warp::reply::Json,), Error = warp::Rejection> + Clone {
    warp::path!("api" / "v1" / "preferences" / "userPreferenceUpdate")
        .and(warp::put())
        .and(warp::body::json())
        .map(|_req: UpdateUserPreferenceRequest| {
            let response = UserPreferenceResponse::new(
                "user123".to_string(),
                "test_preference".to_string(),
                json!("updated_value"),
                PreferenceCategory::ModelSelection,
            );
            warp::reply::json(&response)
        })
}

#[derive(Debug, serde::Deserialize)]
struct DeleteQuery {
    user_id: String,
}

pub fn preference_delete_endpoint(
) -> impl Filter<Extract = (warp::reply::Json,), Error = warp::Rejection> + Clone {
    warp::path!("api" / "v1" / "preferences" / "userPreferenceDelete")
        .and(warp::delete())
        .and(warp::query())
        .map(|_query: DeleteQuery| {
            let response = PreferenceDeleteResponse::new(
                "user123".to_string(),
                "test_preference".to_string(),
                true,
            );
            warp::reply::json(&response)
        })
}
