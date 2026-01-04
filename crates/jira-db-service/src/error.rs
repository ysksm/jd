//! Service layer errors

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("Not initialized")]
    NotInitialized,

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("JIRA API error: {0}")]
    JiraApi(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<jira_db_core::DomainError> for ServiceError {
    fn from(e: jira_db_core::DomainError) -> Self {
        ServiceError::Database(e.to_string())
    }
}

impl From<anyhow::Error> for ServiceError {
    fn from(e: anyhow::Error) -> Self {
        ServiceError::Internal(e.to_string())
    }
}

impl From<std::io::Error> for ServiceError {
    fn from(e: std::io::Error) -> Self {
        ServiceError::Io(e.to_string())
    }
}

pub type ServiceResult<T> = Result<T, ServiceError>;
