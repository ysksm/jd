//! JSON-RPC 2.0 types
//!
//! Implements the JSON-RPC 2.0 specification for MCP communication.
//! See: https://www.jsonrpc.org/specification

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 version constant
pub const JSONRPC_VERSION: &str = "2.0";

/// JSON-RPC Request ID (can be string, number, or null)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Number(i64),
}

impl std::fmt::Display for RequestId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RequestId::String(s) => write!(f, "{}", s),
            RequestId::Number(n) => write!(f, "{}", n),
        }
    }
}

/// JSON-RPC Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (must be "2.0")
    pub jsonrpc: String,

    /// Request method name
    pub method: String,

    /// Request parameters (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,

    /// Request ID (absent for notifications)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<RequestId>,
}

impl JsonRpcRequest {
    /// Create a new request with an ID
    pub fn new(method: impl Into<String>, params: Option<Value>, id: RequestId) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params,
            id: Some(id),
        }
    }

    /// Create a notification (no ID, no response expected)
    pub fn notification(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params,
            id: None,
        }
    }

    /// Check if this is a notification (no response expected)
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }
}

/// JSON-RPC Success Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version (must be "2.0")
    pub jsonrpc: String,

    /// Result value
    pub result: Value,

    /// Request ID (must match request)
    pub id: RequestId,
}

impl JsonRpcResponse {
    /// Create a new success response
    pub fn success(result: Value, id: RequestId) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result,
            id,
        }
    }
}

/// JSON-RPC Error Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorResponse {
    /// JSON-RPC version (must be "2.0")
    pub jsonrpc: String,

    /// Error object
    pub error: JsonRpcError,

    /// Request ID (null if couldn't be determined)
    pub id: Option<RequestId>,
}

impl JsonRpcErrorResponse {
    /// Create a new error response
    pub fn new(error: JsonRpcError, id: Option<RequestId>) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            error,
            id,
        }
    }
}

/// JSON-RPC Error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code
    pub code: i32,

    /// Error message
    pub message: String,

    /// Additional error data (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Standard JSON-RPC error codes
pub mod error_codes {
    /// Parse error - Invalid JSON was received
    pub const PARSE_ERROR: i32 = -32700;

    /// Invalid Request - The JSON sent is not a valid Request object
    pub const INVALID_REQUEST: i32 = -32600;

    /// Method not found - The method does not exist / is not available
    pub const METHOD_NOT_FOUND: i32 = -32601;

    /// Invalid params - Invalid method parameter(s)
    pub const INVALID_PARAMS: i32 = -32602;

    /// Internal error - Internal JSON-RPC error
    pub const INTERNAL_ERROR: i32 = -32603;
}

impl JsonRpcError {
    /// Create a parse error
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self {
            code: error_codes::PARSE_ERROR,
            message: message.into(),
            data: None,
        }
    }

    /// Create an invalid request error
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: error_codes::INVALID_REQUEST,
            message: message.into(),
            data: None,
        }
    }

    /// Create a method not found error
    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self {
            code: error_codes::METHOD_NOT_FOUND,
            message: format!("Method not found: {}", method.into()),
            data: None,
        }
    }

    /// Create an invalid params error
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: error_codes::INVALID_PARAMS,
            message: message.into(),
            data: None,
        }
    }

    /// Create an internal error
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self {
            code: error_codes::INTERNAL_ERROR,
            message: message.into(),
            data: None,
        }
    }
}

/// A message that could be either a request or response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum JsonRpcMessage {
    Request(JsonRpcRequest),
    Response(JsonRpcResponse),
    ErrorResponse(JsonRpcErrorResponse),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_serialization() {
        let req = JsonRpcRequest::new(
            "test_method",
            Some(serde_json::json!({"key": "value"})),
            RequestId::Number(1),
        );

        let json = serde_json::to_string(&req).unwrap();
        let parsed: JsonRpcRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.method, "test_method");
        assert_eq!(parsed.id, Some(RequestId::Number(1)));
    }

    #[test]
    fn test_notification() {
        let notif = JsonRpcRequest::notification("notify", None);
        assert!(notif.is_notification());
    }

    #[test]
    fn test_response_serialization() {
        let resp = JsonRpcResponse::success(
            serde_json::json!({"result": "ok"}),
            RequestId::String("abc".to_string()),
        );

        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"result\""));
    }

    #[test]
    fn test_error_response() {
        let err = JsonRpcErrorResponse::new(
            JsonRpcError::method_not_found("unknown"),
            Some(RequestId::Number(1)),
        );

        assert_eq!(err.error.code, error_codes::METHOD_NOT_FOUND);
    }
}
