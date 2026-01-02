Start the MCP server for AI integration.

Arguments:
- $ARGUMENTS: Optional port number for HTTP mode (default: stdio mode)

If no arguments provided, start in stdio mode:
```bash
cargo run -p jira-db-mcp -- --database ./data/jira.duckdb
```

If a port number is provided, start in HTTP mode:
```bash
cargo run -p jira-db-mcp -- --database ./data/jira.duckdb --http --port $ARGUMENTS
```

The MCP server provides the following tools for AI assistants:
- search_issues: Full-text search with filters
- get_issue: Get issue details by key
- get_issue_history: Get change history
- list_projects: List synced projects
- get_project_metadata: Get metadata
- get_schema: Get database schema
- execute_sql: Execute read-only SQL
- semantic_search: Vector similarity search
