//! Initialize handler

use crate::protocol::{
    InitializeParams, InitializeResult, ProtocolResult, ServerCapabilities,
    ServerInfo, ToolsCapability, MCP_PROTOCOL_VERSION,
};

/// Package version from Cargo.toml
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Server name
const SERVER_NAME: &str = "jira-db-mcp";

/// Handle initialize request
pub fn handle_initialize(params: InitializeParams) -> ProtocolResult<InitializeResult> {
    tracing::info!(
        "Initializing MCP server. Client: {} v{}, Protocol: {}",
        params.client_info.name,
        params.client_info.version,
        params.protocol_version
    );

    let result = InitializeResult {
        protocol_version: MCP_PROTOCOL_VERSION.to_string(),
        capabilities: ServerCapabilities {
            tools: Some(ToolsCapability { list_changed: false }),
            resources: None,
            prompts: None,
            logging: None,
            experimental: None,
        },
        server_info: ServerInfo {
            name: SERVER_NAME.to_string(),
            version: VERSION.to_string(),
        },
        instructions: Some(
            "JIRA Database MCP Server - Query and search JIRA issues stored in a local DuckDB database. \
             Available tools: search_issues, get_issue, get_issue_history, list_projects, \
             get_project_metadata, get_schema, execute_sql, semantic_search.".to_string()
        ),
    };

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{ClientCapabilities, ClientInfo};

    #[test]
    fn test_handle_initialize() {
        let params = InitializeParams {
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            capabilities: ClientCapabilities::default(),
            client_info: ClientInfo {
                name: "test-client".to_string(),
                version: "1.0.0".to_string(),
            },
        };

        let result = handle_initialize(params).unwrap();

        assert_eq!(result.protocol_version, MCP_PROTOCOL_VERSION);
        assert_eq!(result.server_info.name, SERVER_NAME);
        assert!(result.capabilities.tools.is_some());
    }
}
