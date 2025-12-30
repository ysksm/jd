use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Entity not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Repository error: {0}")]
    Repository(String),

    #[error("External service error: {0}")]
    ExternalService(String),
}

pub type DomainResult<T> = std::result::Result<T, DomainError>;
