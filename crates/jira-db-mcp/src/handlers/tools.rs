//! Tool-related handlers

use crate::protocol::{
    CallToolParams, CallToolResult, ListToolsResult, ProtocolError, ProtocolResult,
};
use crate::tools::ToolRegistry;

/// Handle tools/list request
pub fn handle_tools_list(registry: &ToolRegistry) -> ProtocolResult<ListToolsResult> {
    let tools = registry.list_tools();
    Ok(ListToolsResult { tools })
}

/// Handle tools/call request
pub async fn handle_tool_call(
    registry: &ToolRegistry,
    params: CallToolParams,
) -> ProtocolResult<CallToolResult> {
    tracing::info!("Calling tool: {}", params.name);

    registry
        .call_tool(&params.name, params.arguments)
        .await
        .map_err(|e| ProtocolError::ToolError(e.to_string()))
}
