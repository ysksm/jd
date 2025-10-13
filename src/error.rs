use thiserror::Error;

#[derive(Debug, Error)]
pub enum JiraDbError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("JIRA API error: {0}")]
    JiraApi(String),

    #[error("Database error: {0}")]
    Database(#[from] duckdb::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Project not found: {0}")]
    ProjectNotFound(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

pub type Result<T> = std::result::Result<T, JiraDbError>;
