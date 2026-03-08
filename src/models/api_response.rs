use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct APIResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<APIError>,
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct APIError {
    pub code: String,
    pub message: String,
    pub details: Option<Vec<ErrorDetail>>,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ErrorDetail {
    pub field: Option<String>,
    pub message: String,
    pub code: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseMetadata {
    pub request_id: String,
    pub timestamp: String,
    pub version: String,
    pub server_info: ServerInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
    pub environment: String,
    pub region: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaginatedResponse<T> {
    pub success: bool,
    pub data: Vec<T>,
    pub pagination: PaginationInfo,
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PaginationInfo {
    pub page: u32,
    pub per_page: u32,
    pub total: u64,
    pub total_pages: u32,
    pub has_next: bool,
    pub has_prev: bool,
}

// Error types
pub type APIResult<T> = Result<APIResponse<T>, APIError>;

#[derive(Debug, Clone)]
pub enum ErrorCode {
    ValidationError,
    AuthenticationError,
    AuthorizationError,
    NotFoundError,
    ConflictError,
    RateLimitError,
    InternalServerError,
    ServiceUnavailable,
    BadRequest,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::ValidationError => "VALIDATION_ERROR",
            ErrorCode::AuthenticationError => "AUTHENTICATION_ERROR",
            ErrorCode::AuthorizationError => "AUTHORIZATION_ERROR",
            ErrorCode::NotFoundError => "NOT_FOUND",
            ErrorCode::ConflictError => "CONFLICT",
            ErrorCode::RateLimitError => "RATE_LIMIT_ERROR",
            ErrorCode::InternalServerError => "INTERNAL_SERVER_ERROR",
            ErrorCode::ServiceUnavailable => "SERVICE_UNAVAILABLE",
            ErrorCode::BadRequest => "BAD_REQUEST",
        }
    }

    pub fn http_status(&self) -> u16 {
        match self {
            ErrorCode::ValidationError => 400,
            ErrorCode::AuthenticationError => 401,
            ErrorCode::AuthorizationError => 403,
            ErrorCode::NotFoundError => 404,
            ErrorCode::ConflictError => 409,
            ErrorCode::RateLimitError => 429,
            ErrorCode::InternalServerError => 500,
            ErrorCode::ServiceUnavailable => 503,
            ErrorCode::BadRequest => 400,
        }
    }
}

impl<T> APIResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            metadata: ResponseMetadata::new(),
        }
    }

    pub fn error(error: APIError) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            metadata: ResponseMetadata::new(),
        }
    }

    pub fn validation_error(message: String, details: Option<Vec<ErrorDetail>>) -> Self {
        let error = APIError::new(ErrorCode::ValidationError, message, details);
        Self::error(error)
    }

    pub fn not_found(message: String) -> Self {
        let error = APIError::new(ErrorCode::NotFoundError, message, None);
        Self::error(error)
    }

    pub fn conflict(message: String) -> Self {
        let error = APIError::new(ErrorCode::ConflictError, message, None);
        Self::error(error)
    }

    pub fn internal_error(message: String) -> Self {
        let error = APIError::new(ErrorCode::InternalServerError, message, None);
        Self::error(error)
    }

    pub fn unauthorized(message: String) -> Self {
        let error = APIError::new(ErrorCode::AuthenticationError, message, None);
        Self::error(error)
    }

    pub fn forbidden(message: String) -> Self {
        let error = APIError::new(ErrorCode::AuthorizationError, message, None);
        Self::error(error)
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.metadata.request_id = request_id;
        self
    }

    pub fn with_metadata(mut self, metadata: ResponseMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

impl APIError {
    pub fn new(code: ErrorCode, message: String, details: Option<Vec<ErrorDetail>>) -> Self {
        Self {
            code: code.as_str().to_string(),
            message,
            details,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    pub fn validation(message: String, field: Option<String>) -> Self {
        let details = field.map(|f| vec![ErrorDetail {
            field: Some(f),
            message: message.clone(),
            code: Some("INVALID_VALUE".to_string()),
        }]);
        Self::new(ErrorCode::ValidationError, message, details)
    }

    pub fn required_field(field: String) -> Self {
        Self::validation(format!("{} is required", field), Some(field))
    }

    pub fn invalid_value(field: String, value: String) -> Self {
        Self::validation(
            format!("Invalid value '{}' for field {}", value, field),
            Some(field),
        )
    }

    pub fn with_details(mut self, details: Vec<ErrorDetail>) -> Self {
        self.details = Some(details);
        self
    }

    pub fn http_status(&self) -> u16 {
        match self.code.as_str() {
            "VALIDATION_ERROR" => 400,
            "AUTHENTICATION_ERROR" => 401,
            "AUTHORIZATION_ERROR" => 403,
            "NOT_FOUND" => 404,
            "CONFLICT" => 409,
            "RATE_LIMIT_ERROR" => 429,
            "INTERNAL_SERVER_ERROR" => 500,
            "SERVICE_UNAVAILABLE" => 503,
            "BAD_REQUEST" => 400,
            _ => 500,
        }
    }
}

impl ErrorDetail {
    pub fn new(field: Option<String>, message: String) -> Self {
        Self {
            field,
            message,
            code: None,
        }
    }

    pub fn with_code(mut self, code: String) -> Self {
        self.code = Some(code);
        self
    }
}

impl ResponseMetadata {
    pub fn new() -> Self {
        Self {
            request_id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            version: "1.0".to_string(),
            server_info: ServerInfo::new(),
        }
    }

    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = request_id;
        self
    }

    pub fn with_version(mut self, version: String) -> Self {
        self.version = version;
        self
    }

    pub fn with_server_info(mut self, server_info: ServerInfo) -> Self {
        self.server_info = server_info;
        self
    }
}

impl ServerInfo {
    pub fn new() -> Self {
        Self {
            name: "Merlin AI Router".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            environment: Self::get_environment(),
            region: None,
        }
    }

    pub fn with_environment(mut self, environment: String) -> Self {
        self.environment = environment;
        self
    }

    pub fn with_region(mut self, region: String) -> Self {
        self.region = Some(region);
        self
    }

    fn get_environment() -> String {
        std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string())
    }
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, page: u32, per_page: u32, total: u64) -> Self {
        let total_pages = ((total as f64) / (per_page as f64)).ceil() as u32;
        Self {
            success: true,
            data,
            pagination: PaginationInfo {
                page,
                per_page,
                total,
                total_pages,
                has_next: page < total_pages,
                has_prev: page > 1,
            },
            metadata: ResponseMetadata::new(),
        }
    }

    pub fn with_metadata(mut self, metadata: ResponseMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

// Conversion implementations
impl<T> From<T> for APIResponse<T> {
    fn from(data: T) -> Self {
        APIResponse::success(data)
    }
}

impl From<APIError> for APIResponse<String> {
    fn from(error: APIError) -> Self {
        APIResponse::error(error)
    }
}

// Helper macros
#[macro_export]
macro_rules! api_response {
    ($data:expr) => {
        $crate::models::api_response::APIResponse::success($data)
    };
    (validation_error, $message:expr) => {
        $crate::models::api_response::APIResponse::validation_error($message.to_string(), None)
    };
    (validation_error, $message:expr, $details:expr) => {
        $crate::models::api_response::APIResponse::validation_error($message.to_string(), Some($details))
    };
    (not_found, $message:expr) => {
        $crate::models::api_response::APIResponse::not_found($message.to_string())
    };
    (conflict, $message:expr) => {
        $crate::models::api_response::APIResponse::conflict($message.to_string())
    };
    (internal_error, $message:expr) => {
        $crate::models::api_response::APIResponse::internal_error($message.to_string())
    };
    (unauthorized, $message:expr) => {
        $crate::models::api_response::APIResponse::unauthorized($message.to_string())
    };
    (forbidden, $message:expr) => {
        $crate::models::api_response::APIResponse::forbidden($message.to_string())
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_success() {
        let response: APIResponse<String> = APIResponse::success("test data".to_string());
        assert!(response.success);
        assert_eq!(response.data, Some("test data".to_string()));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let error = APIError::new(ErrorCode::ValidationError, "Test error".to_string(), None);
        let response: APIResponse<String> = APIResponse::error(error);
        assert!(!response.success);
        assert!(response.data.is_none());
        assert!(response.error.is_some());
    }

    #[test]
    fn test_api_error_creation() {
        let error = APIError::validation("Invalid field".to_string(), Some("field_name".to_string()));
        assert_eq!(error.code, "VALIDATION_ERROR");
        assert_eq!(error.message, "Invalid field");
        assert!(error.details.is_some());
    }

    #[test]
    fn test_error_code_properties() {
        assert_eq!(ErrorCode::ValidationError.as_str(), "VALIDATION_ERROR");
        assert_eq!(ErrorCode::ValidationError.http_status(), 400);
        assert_eq!(ErrorCode::NotFoundError.http_status(), 404);
        assert_eq!(ErrorCode::AuthenticationError.http_status(), 401);
    }

    #[test]
    fn test_response_metadata_creation() {
        let metadata = ResponseMetadata::new();
        assert!(!metadata.request_id.is_empty());
        assert!(!metadata.timestamp.is_empty());
        assert_eq!(metadata.version, "1.0");
    }

    #[test]
    fn test_server_info_creation() {
        let server_info = ServerInfo::new();
        assert_eq!(server_info.name, "Merlin AI Router");
        assert_eq!(server_info.environment, "development"); // default
    }

    #[test]
    fn test_paginated_response_creation() {
        let data = vec!["item1", "item2", "item3"];
        let response = PaginatedResponse::new(data, 1, 10, 25);

        assert!(response.success);
        assert_eq!(response.data.len(), 3);
        assert_eq!(response.pagination.page, 1);
        assert_eq!(response.pagination.per_page, 10);
        assert_eq!(response.pagination.total, 25);
        assert_eq!(response.pagination.total_pages, 3);
        assert!(response.pagination.has_next);
        assert!(!response.pagination.has_prev);
    }

    #[test]
    fn test_error_detail_creation() {
        let detail = ErrorDetail::new(Some("field_name".to_string()), "Error message".to_string());
        assert_eq!(detail.field, Some("field_name".to_string()));
        assert_eq!(detail.message, "Error message");
    }
}