//! MCP (Model Context Protocol) types
//!
//! Implements the MCP specification types for server communication.
//! See: https://modelcontextprotocol.io/specification

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// MCP Protocol version
pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

//=============================================================================
// Initialization
//=============================================================================

/// Client information sent during initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientInfo {
    /// Client name
    pub name: String,

    /// Client version
    pub version: String,
}

/// Server information returned during initialization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfo {
    /// Server name
    pub name: String,

    /// Server version
    pub version: String,
}

/// Initialize request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeParams {
    /// Protocol version the client supports
    pub protocol_version: String,

    /// Client capabilities
    pub capabilities: ClientCapabilities,

    /// Client information
    pub client_info: ClientInfo,
}

/// Client capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapabilities {
    /// Whether client supports roots
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,

    /// Whether client supports sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<Value>,

    /// Experimental capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,
}

/// Roots capability
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RootsCapability {
    /// Whether list changed notifications are supported
    #[serde(default)]
    pub list_changed: bool,
}

/// Initialize response result
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeResult {
    /// Protocol version the server is using
    pub protocol_version: String,

    /// Server capabilities
    pub capabilities: ServerCapabilities,

    /// Server information
    pub server_info: ServerInfo,

    /// Optional instructions for the client
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

/// Server capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCapabilities {
    /// Tools capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<ToolsCapability>,

    /// Resources capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourcesCapability>,

    /// Prompts capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsCapability>,

    /// Logging capability
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<Value>,

    /// Experimental capabilities
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental: Option<Value>,
}

/// Tools capability
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolsCapability {
    /// Whether list changed notifications are supported
    #[serde(default)]
    pub list_changed: bool,
}

/// Resources capability
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesCapability {
    /// Whether subscriptions are supported
    #[serde(default)]
    pub subscribe: bool,

    /// Whether list changed notifications are supported
    #[serde(default)]
    pub list_changed: bool,
}

/// Prompts capability
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptsCapability {
    /// Whether list changed notifications are supported
    #[serde(default)]
    pub list_changed: bool,
}

//=============================================================================
// Tools
//=============================================================================

/// Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tool {
    /// Tool name
    pub name: String,

    /// Tool description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Input schema (JSON Schema)
    pub input_schema: ToolInputSchema,
}

/// Tool input schema (JSON Schema format)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInputSchema {
    /// Schema type (usually "object")
    #[serde(rename = "type")]
    pub schema_type: String,

    /// Properties definition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Value>,

    /// Required properties
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

impl ToolInputSchema {
    /// Create an empty object schema
    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: Some(serde_json::json!({})),
            required: None,
        }
    }

    /// Create a schema from a JSON Schema value
    pub fn from_json_schema(schema: Value) -> Self {
        let properties = schema.get("properties").cloned();
        let required = schema
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

        Self {
            schema_type: "object".to_string(),
            properties,
            required,
        }
    }
}

/// List tools response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListToolsResult {
    /// Available tools
    pub tools: Vec<Tool>,
}

/// Call tool request parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolParams {
    /// Tool name to call
    pub name: String,

    /// Tool arguments
    #[serde(default)]
    pub arguments: Value,
}

/// Call tool response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallToolResult {
    /// Content returned by the tool
    pub content: Vec<ToolContent>,

    /// Whether the tool call resulted in an error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl CallToolResult {
    /// Create a successful text result
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Text {
                text: content.into(),
            }],
            is_error: None,
        }
    }

    /// Create an error result
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Text {
                text: message.into(),
            }],
            is_error: Some(true),
        }
    }
}

/// Tool content types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ToolContent {
    /// Text content
    #[serde(rename = "text")]
    Text {
        /// The text content
        text: String,
    },

    /// Image content
    #[serde(rename = "image")]
    Image {
        /// Base64-encoded image data
        data: String,
        /// MIME type of the image
        mime_type: String,
    },

    /// Resource reference
    #[serde(rename = "resource")]
    Resource {
        /// Resource URI
        uri: String,
        /// Resource MIME type
        #[serde(skip_serializing_if = "Option::is_none")]
        mime_type: Option<String>,
        /// Resource text content
        #[serde(skip_serializing_if = "Option::is_none")]
        text: Option<String>,
    },
}

//=============================================================================
// MCP Method Names
//=============================================================================

/// Standard MCP method names
pub mod methods {
    /// Initialize the connection
    pub const INITIALIZE: &str = "initialize";

    /// Notification that initialization is complete
    pub const INITIALIZED: &str = "notifications/initialized";

    /// List available tools
    pub const TOOLS_LIST: &str = "tools/list";

    /// Call a tool
    pub const TOOLS_CALL: &str = "tools/call";

    /// Ping/pong for connection health
    pub const PING: &str = "ping";

    /// Shutdown notification
    pub const SHUTDOWN: &str = "shutdown";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_params() {
        let params = InitializeParams {
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: ClientInfo {
                name: "test-client".to_string(),
                version: "1.0.0".to_string(),
            },
        };

        let json = serde_json::to_string(&params).unwrap();
        let parsed: InitializeParams = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.protocol_version, MCP_PROTOCOL_VERSION);
        assert_eq!(parsed.client_info.name, "test-client");
    }

    #[test]
    fn test_tool_definition() {
        let tool = Tool {
            name: "test_tool".to_string(),
            description: Some("A test tool".to_string()),
            input_schema: ToolInputSchema {
                schema_type: "object".to_string(),
                properties: Some(serde_json::json!({
                    "query": {"type": "string"}
                })),
                required: Some(vec!["query".to_string()]),
            },
        };

        let json = serde_json::to_string_pretty(&tool).unwrap();
        assert!(json.contains("test_tool"));
        assert!(json.contains("inputSchema"));
    }

    #[test]
    fn test_call_tool_result() {
        let result = CallToolResult::text("Hello, World!");
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("Hello, World!"));

        let error = CallToolResult::error("Something went wrong");
        assert!(error.is_error == Some(true));
    }
}
