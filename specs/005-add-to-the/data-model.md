# Data Model: API Reference Endpoints

**Feature**: API Reference Endpoints for Merlin AI Router
**Date**: 2025-09-21
**Status**: Complete

## Entity Overview

This document defines the data models for the API reference endpoints feature. All entities are designed to integrate with the existing Merlin AI Router architecture while maintaining constitutional compliance.

## Core Entities

### 1. ModelSelectionRequest

Represents a user's request to select a specific AI model for processing.

```rust
pub struct ModelSelectionRequest {
    pub id: Uuid,
    pub user_id: String,
    pub prompt: String,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f64>,
    pub model_preferences: Option<ModelPreferences>,
    pub context: Option<RequestContext>,
    pub created_at: SystemTime,
}
```

**Fields**:
- **id**: Unique identifier for the selection request
- **user_id**: Identifier of the user making the request
- **prompt**: The text prompt to be processed
- **max_tokens**: Optional maximum token limit for response
- **temperature**: Optional creativity parameter (0.0-1.0)
- **model_preferences**: Optional user preferences for model selection
- **context**: Optional request context information
- **created_at**: Timestamp of request creation

**Validation Rules**:
- `user_id`: Required, non-empty string
- `prompt`: Required, 1-10000 characters
- `max_tokens`: Optional, if present must be 1-32000
- `temperature`: Optional, if present must be 0.0-1.0

### 2. ModelSelectionResponse

Represents the response from a model selection request.

```rust
pub struct ModelSelectionResponse {
    pub request_id: Uuid,
    pub selected_model: String,
    pub response: String,
    pub tokens_used: usize,
    pub processing_time: Duration,
    pub confidence_score: f64,
    pub metadata: ResponseMetadata,
}
```

**Fields**:
- **request_id**: Corresponding request identifier
- **selected_model**: Name of the selected model
- **response**: The generated response text
- **tokens_used**: Number of tokens consumed
- **processing_time**: Time taken to generate response
- **confidence_score**: Model's confidence in the response
- **metadata**: Additional response metadata

### 3. FeedbackSubmission

Represents user feedback on model performance.

```rust
pub struct FeedbackSubmission {
    pub id: Uuid,
    pub user_id: String,
    pub request_id: Option<Uuid>,
    pub model_name: String,
    pub rating: i32,
    pub feedback_text: Option<String>,
    pub category: FeedbackCategory,
    pub created_at: SystemTime,
}
```

**Fields**:
- **id**: Unique identifier for the feedback
- **user_id**: Identifier of the user providing feedback
- **request_id**: Optional related request identifier
- **model_name**: Name of the model being rated
- **rating**: Numerical rating (1-5)
- **feedback_text**: Optional detailed feedback text
- **category**: Category of feedback
- **created_at**: Timestamp of feedback creation

**Validation Rules**:
- `user_id`: Required, non-empty string
- `model_name`: Required, must be a valid model name
- `rating`: Required, must be 1-5
- `feedback_text`: Optional, if present 1-1000 characters

### 4. UserPreference

Represents a user's stored preferences.

```rust
pub struct UserPreference {
    pub id: Uuid,
    pub user_id: String,
    pub preference_key: String,
    pub preference_value: PreferenceValue,
    pub category: PreferenceCategory,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub version: i32,
}
```

**Fields**:
- **id**: Unique identifier for the preference
- **user_id**: Identifier of the preference owner
- **preference_key**: Unique key for the preference
- **preference_value**: The preference value (flexible type)
- **category**: Category of the preference
- **created_at**: Timestamp of preference creation
- **updated_at**: Timestamp of last update
- **version**: Version number for optimistic locking

**Validation Rules**:
- `user_id`: Required, non-empty string
- `preference_key`: Required, valid preference key
- `preference_value`: Required, valid for the preference type
- `category`: Required, valid preference category

### 5. APIResponse

Standardized response format for all API endpoints.

```rust
pub struct APIResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<APIError>,
    pub metadata: ResponseMetadata,
    pub request_id: Uuid,
}
```

**Fields**:
- **success**: Whether the request was successful
- **data**: Response data (if successful)
- **error**: Error details (if unsuccessful)
- **metadata**: Response metadata
- **request_id**: Unique request identifier

## Supporting Enums and Types

### ModelPreferences

```rust
pub struct ModelPreferences {
    pub preferred_models: Vec<String>,
    pub excluded_models: Vec<String>,
    pub max_cost: Option<f64>,
    pub min_performance: Option<f64>,
}
```

### RequestContext

```rust
pub struct RequestContext {
    pub session_id: Option<String>,
    pub source_application: Option<String>,
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
}
```

### ResponseMetadata

```rust
pub struct ResponseMetadata {
    pub server_version: String,
    pub processing_time_ms: u64,
    pub request_timestamp: SystemTime,
    pub response_timestamp: SystemTime,
    pub rate_limit_remaining: Option<u32>,
}
```

### APIError

```rust
pub struct APIError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
    pub field_errors: Option<Vec<FieldError>>,
}
```

### FeedbackCategory

```rust
pub enum FeedbackCategory {
    Accuracy,
    Relevance,
    Coherence,
    Helpfulness,
    Performance,
    Safety,
    Other,
}
```

### PreferenceCategory

```rust
pub enum PreferenceCategory {
    ModelSelection,
    ResponseFormatting,
    ContentFiltering,
    PrivacySettings,
    NotificationSettings,
    InterfaceSettings,
    Other,
}
```

### PreferenceValue

```rust
pub enum PreferenceValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Json(serde_json::Value),
    StringList(Vec<String>),
}
```

### FieldError

```rust
pub struct FieldError {
    pub field: String,
    pub message: String,
    pub code: String,
}
```

## State Transitions

### UserPreference Lifecycle
1. **Created**: Initial preference creation
2. **Updated**: Preference value modification
3. **Deleted**: Preference removal
4. **Version Bumped**: On each update (optimistic locking)

### ModelSelectionRequest Lifecycle
1. **Received**: Request received and validated
2. **Processing**: Model selection in progress
3. **Completed**: Response generated and returned
4. **Failed**: Error occurred during processing

### FeedbackSubmission Lifecycle
1. **Submitted**: Feedback received and validated
2. **Processed**: Feedback analyzed and stored
3. **Applied**: Feedback incorporated into model selection

## Relationships

### User Preference Relationships
- **User** → 1:N **UserPreference** (one user can have multiple preferences)
- **UserPreference** → 1:1 **PreferenceValue** (each preference has one value)
- **UserPreference** → 1:1 **PreferenceCategory** (each preference belongs to one category)

### Feedback Relationships
- **User** → 1:N **FeedbackSubmission** (one user can submit multiple feedback)
- **FeedbackSubmission** → 0:1 **ModelSelectionRequest** (feedback can be linked to a specific request)
- **FeedbackSubmission** → 1:1 **FeedbackCategory** (each feedback belongs to one category)

### Model Selection Relationships
- **User** → 1:N **ModelSelectionRequest** (one user can make multiple requests)
- **ModelSelectionRequest** → 0:1 **ModelPreferences** (requests can have optional preferences)
- **ModelSelectionRequest** → 1:1 **ModelSelectionResponse** (each request generates one response)

## Storage Considerations

### Redis Storage Schema
- **User Preferences**: `user:preferences:{user_id}:{key}` → JSON
- **Model Selection Cache**: `model:selection:{request_id}` → JSON (temporary)
- **Feedback Cache**: `feedback:{user_id}:{model_name}` → JSON (temporary)

### Persistence Strategy
- **Preferences**: Persistent in Redis with TTL
- **Model Selection**: Temporary cache with short TTL
- **Feedback**: Persistent storage for analysis, temporary cache for recent submissions

## Validation Rules Summary

### Input Validation
- All required fields must be present
- String fields must have valid length limits
- Numeric fields must be within valid ranges
- Enum fields must have valid values
- UUID fields must be valid UUID format

### Business Logic Validation
- Users can only modify their own preferences
- Model names must exist in the system
- Ratings must be within valid range (1-5)
- Preference keys must be valid for their category
- Request timestamps must be reasonable

### Security Validation
- Input sanitization for all string fields
- SQL injection prevention
- XSS prevention for text fields
- Rate limiting for API endpoints
- Authentication and authorization where required

## Integration Points

### With Existing Router System
- **LlmProvider Trait**: New endpoints work with existing provider implementations
- **Routing Policies**: Model selection uses existing routing algorithms
- **Metrics Collection**: Extend existing Prometheus metrics
- **Configuration**: Use existing TOML configuration system

### With Existing Storage
- **Redis**: Leverage existing Redis infrastructure
- **Metrics Storage**: Use existing metrics collection
- **Caching**: Integrate with existing caching strategies

This data model provides a solid foundation for implementing the API reference endpoints while maintaining alignment with the Merlin AI Router's existing architecture and constitutional principles.