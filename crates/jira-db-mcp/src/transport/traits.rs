//! Transport trait definitions

use async_trait::async_trait;
use serde_json::Value;

use crate::protocol::{JsonRpcRequest, ProtocolResult};

/// Transport trait for sending and receiving MCP messages
///
/// This trait abstracts the underlying transport mechanism (stdio, HTTP, etc.)
/// allowing the server logic to be transport-agnostic.
#[async_trait]
pub trait Transport: Send + Sync {
    /// Read the next JSON-RPC request from the transport
    ///
    /// Returns None if the transport is closed.
    async fn read_request(&mut self) -> ProtocolResult<Option<JsonRpcRequest>>;

    /// Send a JSON-RPC response through the transport
    ///
    /// The response can be either a success or error response (as JSON Value).
    async fn send_response(&mut self, response: Value) -> ProtocolResult<()>;

    /// Close the transport
    async fn close(&mut self) -> ProtocolResult<()>;
}
