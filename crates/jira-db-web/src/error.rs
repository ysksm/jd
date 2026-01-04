//! Error types for web handlers

use actix_web::{HttpResponse, ResponseError};
use jira_db_service::ServiceError;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub code: String,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

#[derive(Debug)]
pub struct ApiError {
    pub message: String,
    pub status: actix_web::http::StatusCode,
}

impl ApiError {
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status: actix_web::http::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status: actix_web::http::StatusCode::NOT_FOUND,
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status: actix_web::http::StatusCode::BAD_REQUEST,
        }
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let code = match self.status {
            actix_web::http::StatusCode::NOT_FOUND => "NOT_FOUND",
            actix_web::http::StatusCode::BAD_REQUEST => "BAD_REQUEST",
            _ => "INTERNAL_ERROR",
        };

        HttpResponse::build(self.status).json(ErrorResponse {
            error: self.message.clone(),
            code: code.to_string(),
        })
    }
}

impl From<ServiceError> for ApiError {
    fn from(e: ServiceError) -> Self {
        match e {
            ServiceError::NotInitialized => ApiError::bad_request("Not initialized"),
            ServiceError::NotFound(msg) => ApiError::not_found(msg),
            ServiceError::InvalidRequest(msg) => ApiError::bad_request(msg),
            ServiceError::Database(msg) => ApiError::internal(format!("Database error: {}", msg)),
            ServiceError::JiraApi(msg) => ApiError::internal(format!("JIRA API error: {}", msg)),
            ServiceError::Config(msg) => ApiError::internal(format!("Config error: {}", msg)),
            ServiceError::Io(msg) => ApiError::internal(format!("IO error: {}", msg)),
            ServiceError::Internal(msg) => ApiError::internal(msg),
        }
    }
}
