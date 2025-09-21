// src/api/mod.rs
pub mod enhanced_model_select;
pub mod model_select;
pub mod preferences;
pub mod ab_testing;

pub use enhanced_model_select::*;
// Use specific imports to avoid UserPreferences conflicts
pub use model_select::{
    Message, ModelSelectRequest, ModelSelectResponse, OptimizationTarget,
    FeedbackRequest, FeedbackResponse, FeedbackType, ModelAlternative,
    PromptFeatures, DomainCategory, TaskType, ModelUserPreferences
};
// Import preferences types explicitly to avoid conflict
// Note: Use UserPreferences from preferences module (the canonical one)
pub use preferences::{
    CreateUserPreferenceRequest, UpdateUserPreferenceRequest, UserPreferenceResponse,
    DeleteUserPreferenceResponse, ListUsersResponse, PreferenceValidationRequest,
    PreferenceValidationResponse, PromptInteractionFeatures, UserInteraction, UserStats,
    PreferenceUpdateRequest, UserPreferences
};
pub use ab_testing::*;
