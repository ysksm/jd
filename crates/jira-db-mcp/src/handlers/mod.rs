//! Request handlers for MCP methods
//!
//! This module contains handlers for each MCP method.
//! Handlers are responsible for processing requests and returning responses.

mod initialize;
mod tools;

pub use initialize::*;
pub use tools::*;

use serde_json::Value;

use crate::protocol::{
    CallToolParams, InitializeParams, JsonRpcRequest, JsonRpcResponse, ProtocolError,
    ProtocolResult, RequestId, methods,
};
use crate::tools::ToolRegistry;

/// MCP request handler
///
/// Routes incoming requests to the appropriate handler based on the method.
pub struct RequestHandler {
    tool_registry: ToolRegistry,
    initialized: bool,
}

impl RequestHandler {
    /// Create a new request handler with the given tool registry
    pub fn new(tool_registry: ToolRegistry) -> Self {
        Self {
            tool_registry,
            initialized: false,
        }
    }

    /// Handle an incoming JSON-RPC request
    ///
    /// Returns None if the request is a notification (no response expected).
    pub async fn handle(&mut self, request: JsonRpcRequest) -> ProtocolResult<Option<Value>> {
        tracing::debug!("Handling method: {}", request.method);

        // Handle notifications (no response)
        if request.is_notification() {
            self.handle_notification(&request.method, request.params)
                .await?;
            return Ok(None);
        }

        let id = request.id.unwrap();
        let result = self
            .handle_request(&request.method, request.params, &id)
            .await?;

        let response = JsonRpcResponse::success(result, id);
        Ok(Some(serde_json::to_value(response)?))
    }

    /// Handle a notification (no response expected)
    async fn handle_notification(
        &mut self,
        method: &str,
        _params: Option<Value>,
    ) -> ProtocolResult<()> {
        match method {
            methods::INITIALIZED => {
                tracing::info!("Client sent initialized notification");
                self.initialized = true;
                Ok(())
            }
            methods::SHUTDOWN => {
                tracing::info!("Client requested shutdown");
                Ok(())
            }
            _ => {
                tracing::warn!("Unknown notification: {}", method);
                Ok(())
            }
        }
    }

    /// Handle a request (response expected)
    async fn handle_request(
        &self,
        method: &str,
        params: Option<Value>,
        _id: &RequestId,
    ) -> ProtocolResult<Value> {
        match method {
            methods::INITIALIZE => {
                let params: InitializeParams = params
                    .map(|p| serde_json::from_value(p))
                    .transpose()
                    .map_err(|e| ProtocolError::InvalidParams(e.to_string()))?
                    .ok_or_else(|| {
                        ProtocolError::InvalidParams("Missing initialize params".to_string())
                    })?;

                let result = handle_initialize(params)?;
                Ok(serde_json::to_value(result)?)
            }
            methods::PING => {
                // Simple ping/pong
                Ok(serde_json::json!({}))
            }
            methods::TOOLS_LIST => {
                let result = handle_tools_list(&self.tool_registry)?;
                Ok(serde_json::to_value(result)?)
            }
            methods::TOOLS_CALL => {
                let params: CallToolParams = params
                    .map(|p| serde_json::from_value(p))
                    .transpose()
                    .map_err(|e| ProtocolError::InvalidParams(e.to_string()))?
                    .ok_or_else(|| {
                        ProtocolError::InvalidParams("Missing tool call params".to_string())
                    })?;

                let result = handle_tool_call(&self.tool_registry, params).await?;
                Ok(serde_json::to_value(result)?)
            }
            _ => Err(ProtocolError::MethodNotFound(method.to_string())),
        }
    }
}
