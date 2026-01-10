# JIRA Mock Server

Mock JIRA API server for testing and development. Provides a lightweight, in-memory JIRA REST API v3 compatible server with JSON file persistence.

## Features

- **JIRA REST API v3 Compatible**: Implements core endpoints for projects, issues, metadata
- **In-Memory Storage**: Fast, thread-safe data store with JSON file persistence
- **Sample Data**: Automatically generates sample project with statuses, issue types, and priorities
- **Changelog Support**: Tracks status transitions with timestamps
- **JQL Support**: Basic JQL parsing for project and text search filters
- **CORS Enabled**: Ready for web client testing

## Quick Start

### Build and Run

```bash
# Build
cargo build -p jira-mock-server --release

# Run with default settings
cargo run -p jira-mock-server

# Run with custom options
cargo run -p jira-mock-server -- --port 3000 --data-dir ./my-data
```

### Command Line Options

| Option | Short | Default | Description |
|--------|-------|---------|-------------|
| `--port` | `-p` | 8080 | Port to listen on |
| `--data-dir` | `-d` | ./mock-data | Directory for JSON data files |

### Environment Variables

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | Log level (e.g., `debug`, `info`, `warn`) |

## API Endpoints

### Projects

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/rest/api/3/project` | List all projects |

### Issues

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/rest/api/3/search/jql` | Search issues (query params) |
| POST | `/rest/api/3/search/jql` | Search issues (JSON body) |
| GET | `/rest/api/3/search` | Legacy search endpoint |
| POST | `/rest/api/3/issue` | Create issue |
| PUT | `/rest/api/3/issue/{key}` | Update issue |

### Workflow

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/rest/api/3/issue/{key}/transitions` | Get available transitions |
| POST | `/rest/api/3/issue/{key}/transitions` | Perform status transition |

### Metadata

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/rest/api/3/project/{key}/statuses` | Get project statuses |
| GET | `/rest/api/3/priority` | Get all priorities |
| GET | `/rest/api/3/issuetype/project` | Get issue types by project ID |
| GET | `/rest/api/3/issue/createmeta/{key}/issuetypes` | Get issue types by project key |
| GET | `/rest/api/3/project/{key}/components` | Get project components |
| GET | `/rest/api/3/project/{key}/versions` | Get project versions |
| GET | `/rest/api/3/field` | Get all field definitions |

### Issue Links

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/rest/api/3/issueLink` | Create issue link |

## Configuration with jira-db

Configure jira-db to use the mock server:

```json
{
  "jira_endpoints": [
    {
      "name": "mock",
      "display_name": "Mock Server",
      "endpoint": "http://localhost:8080",
      "username": "mock@example.com",
      "api_key": "mock-api-key"
    }
  ],
  "active_endpoint": "mock"
}
```

Or using the CLI:

```bash
jira-db endpoint add mock \
  --url http://localhost:8080 \
  --username mock@example.com \
  --api-key mock-api-key \
  --display-name "Mock Server"

jira-db endpoint set-active mock
```

## Default Sample Data

On first run, the server creates sample data:

### Project
- **Key**: `TEST`
- **Name**: Test Project
- **ID**: 10000

### Statuses
| Name | Category |
|------|----------|
| To Do | new |
| In Progress | indeterminate |
| Done | done |

### Issue Types
- Epic
- Story
- Task
- Bug

### Priorities
- Highest
- High
- Medium
- Low
- Lowest

### Workflow Transitions
```
To Do ──► In Progress ──► Done
  ▲            │           │
  │            ▼           │
  └────────────────────────┘
```

## JQL Support

The mock server supports basic JQL queries:

```
# Filter by project
project = TEST

# Text search
text ~ "search term"

# Combined
project = TEST AND text ~ "bug"
```

### Pagination

```bash
# Using query parameters
GET /rest/api/3/search/jql?jql=project=TEST&maxResults=50&startAt=0

# Using JSON body
POST /rest/api/3/search/jql
{
  "jql": "project = TEST",
  "maxResults": 50,
  "startAt": 0
}
```

## Data Persistence

Data is stored in JSON files in the data directory:

```
mock-data/
├── projects.json
├── issues.json
├── statuses.json
├── priorities.json
├── issue_types.json
├── components.json
├── versions.json
├── fields.json
├── issue_links.json
└── transitions.json
```

Files are automatically created and updated as data changes.

## Example Usage

### Create an Issue

```bash
curl -X POST http://localhost:8080/rest/api/3/issue \
  -H "Content-Type: application/json" \
  -d '{
    "fields": {
      "project": {"key": "TEST"},
      "summary": "Test issue",
      "description": "Description here",
      "issuetype": {"name": "Task"},
      "priority": {"name": "Medium"}
    }
  }'
```

Response:
```json
{
  "id": "10001",
  "key": "TEST-1",
  "self": "http://localhost:8080/rest/api/3/issue/10001"
}
```

### Transition an Issue

```bash
# Get available transitions
curl http://localhost:8080/rest/api/3/issue/TEST-1/transitions

# Perform transition
curl -X POST http://localhost:8080/rest/api/3/issue/TEST-1/transitions \
  -H "Content-Type: application/json" \
  -d '{"transition": {"id": "21"}}'
```

### Search Issues

```bash
curl "http://localhost:8080/rest/api/3/search/jql?jql=project=TEST&maxResults=10"
```

## Docker

### Build Image

```bash
docker build -t jira-mock-server -f crates/jira-mock-server/Dockerfile .
```

### Run Container

```bash
docker run -d \
  --name jira-mock \
  -p 8080:8080 \
  -v $(pwd)/mock-data:/app/data \
  jira-mock-server
```

## Limitations

- **No Authentication**: All requests are accepted without auth validation
- **Basic JQL**: Only `project = X` and `text ~ "term"` patterns are supported
- **Limited Updates**: Issue updates support only summary, description, and due date
- **Single Workflow**: All projects share the same To Do → In Progress → Done workflow

## Development

### Project Structure

```
crates/jira-mock-server/
├── Cargo.toml
├── README.md
├── Dockerfile
└── src/
    ├── main.rs      # Entry point, CLI, routing
    ├── handlers.rs  # API endpoint handlers
    └── data.rs      # Data models and storage
```

### Adding New Endpoints

1. Add route in `main.rs`
2. Implement handler in `handlers.rs`
3. Add data model if needed in `data.rs`

### Running Tests

```bash
cargo test -p jira-mock-server
```

## License

Part of the jira-db project.
