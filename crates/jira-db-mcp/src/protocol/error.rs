//! Protocol error types

use thiserror::Error;

use super::jsonrpc::{JsonRpcError, JsonRpcErrorResponse, RequestId};

/// Protocol errors
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum ProtocolError {
    /// JSON parsing error
    #[error("JSON parse error: {0}")]
    JsonParse(#[from] serde_json::Error),

    /// Invalid JSON-RPC request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Method not found
    #[error("Method not found: {0}")]
    MethodNotFound(String),

    /// Invalid parameters
    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    /// Internal server error
    #[error("Internal error: {0}")]
    Internal(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Tool execution error
    #[error("Tool error: {0}")]
    ToolError(String),
}

impl ProtocolError {
    /// Convert to a JSON-RPC error response
    pub fn to_error_response(&self, id: Option<RequestId>) -> JsonRpcErrorResponse {
        let error = match self {
            ProtocolError::JsonParse(e) => JsonRpcError::parse_error(e.to_string()),
            ProtocolError::InvalidRequest(msg) => JsonRpcError::invalid_request(msg),
            ProtocolError::MethodNotFound(method) => JsonRpcError::method_not_found(method),
            ProtocolError::InvalidParams(msg) => JsonRpcError::invalid_params(msg),
            ProtocolError::Internal(msg) => JsonRpcError::internal_error(msg),
            ProtocolError::Io(e) => JsonRpcError::internal_error(e.to_string()),
            ProtocolError::ToolError(msg) => JsonRpcError::internal_error(msg),
        };

        JsonRpcErrorResponse::new(error, id)
    }
}

/// Result type for protocol operations
pub type ProtocolResult<T> = Result<T, ProtocolError>;
