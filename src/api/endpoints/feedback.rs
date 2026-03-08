use crate::models::FeedbackCategory;
use crate::models::{FeedbackResponse, FeedbackSubmission};
use warp::Filter;

pub fn feedback_endpoint(
) -> impl Filter<Extract = (warp::reply::Json,), Error = warp::Rejection> + Clone {
    warp::path!("api" / "v1" / "feedback")
        .and(warp::post())
        .and(warp::body::json())
        .map(|_req: FeedbackSubmission| {
            let response = FeedbackResponse::new(
                "user123".to_string(),
                "gpt-4".to_string(),
                5,
                FeedbackCategory::Helpfulness,
            );
            warp::reply::json(&response)
        })
}
