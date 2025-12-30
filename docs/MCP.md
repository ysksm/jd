# MCP (Model Context Protocol) 技術ドキュメント

## 概要

MCP (Model Context Protocol) は、AIアシスタントが外部ツールやデータソースにアクセスするための標準プロトコルです。Anthropicが開発し、Claude Desktopなどで使用されています。

## MCPの基本概念

### アーキテクチャ

```
┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
│   AIクライアント   │────▶│    MCPサーバー    │────▶│   データソース    │
│  (Claude等)      │◀────│  (jira-db-mcp)  │◀────│   (DuckDB)      │
└─────────────────┘     └─────────────────┘     └─────────────────┘
         │                       │
         │   JSON-RPC 2.0       │
         │                       │
         ▼                       ▼
    ┌─────────┐           ┌─────────┐
    │  stdio  │           │  HTTP   │
    └─────────┘           └─────────┘
```

### 通信プロトコル

MCPはJSON-RPC 2.0をベースにしています：

```json
// リクエスト
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "search_issues",
    "arguments": {"query": "login bug"}
  }
}

// レスポンス
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [{"type": "text", "text": "..."}]
  }
}
```

### トランスポート

MCPは複数のトランスポートをサポートします：

1. **stdio**: 標準入出力を使用（ローカル実行用）
2. **HTTP/SSE**: HTTPリクエストとServer-Sent Events（リモート実行用）

## jira-db-mcp の使用方法

### インストール

```bash
# プロジェクトのビルド
cargo build --release -p jira-db-mcp

# バイナリは target/release/jira-db-mcp に生成されます
```

### 設定ファイルの作成

```bash
# デフォルトの設定ファイルを生成
jira-db-mcp --init
```

設定ファイル (`~/.config/jira-db-mcp/config.json`):

```json
{
  "database_path": "./data/jira.duckdb",
  "http": {
    "enabled": false,
    "port": 3000,
    "host": "127.0.0.1"
  },
  "embedding": {
    "provider": "openai",
    "api_key": null,
    "model": "text-embedding-3-small"
  }
}
```

### 起動方法

#### stdio モード（デフォルト）

```bash
# データベースパスを指定して起動
jira-db-mcp --database ./data/jira.duckdb
```

#### HTTP モード

```bash
# HTTPサーバーとして起動
jira-db-mcp --http --port 3000 --host 127.0.0.1
```

### Claude Desktop との連携

`claude_desktop_config.json` に以下を追加:

```json
{
  "mcpServers": {
    "jira-db": {
      "command": "/path/to/jira-db-mcp",
      "args": ["--database", "/path/to/jira.duckdb"],
      "env": {
        "OPENAI_API_KEY": "your-api-key-for-semantic-search"
      }
    }
  }
}
```

## 利用可能なツール

### 1. search_issues
JIRAイシューをテキストクエリ、プロジェクト、ステータス、担当者で検索します。

```json
{
  "name": "search_issues",
  "arguments": {
    "query": "login error",
    "project": "PROJ",
    "status": "Open",
    "assignee": "john.doe",
    "limit": 20,
    "offset": 0
  }
}
```

### 2. get_issue
特定のイシューの詳細情報を取得します。

```json
{
  "name": "get_issue",
  "arguments": {
    "issue_key": "PROJ-123",
    "include_raw": false
  }
}
```

### 3. get_issue_history
イシューの変更履歴を取得します。

```json
{
  "name": "get_issue_history",
  "arguments": {
    "issue_key": "PROJ-123",
    "field": "status",
    "limit": 100
  }
}
```

### 4. list_projects
データベース内のすべてのプロジェクトを一覧表示します。

```json
{
  "name": "list_projects",
  "arguments": {}
}
```

### 5. get_project_metadata
プロジェクトのメタデータ（ステータス、優先度、イシュータイプなど）を取得します。

```json
{
  "name": "get_project_metadata",
  "arguments": {
    "project_key": "PROJ",
    "metadata_type": "status"
  }
}
```

### 6. get_schema
データベーススキーマ情報を取得します。

```json
{
  "name": "get_schema",
  "arguments": {
    "table": "issues"
  }
}
```

### 7. execute_sql
読み取り専用のSQLクエリを実行します（SELECT文のみ）。

```json
{
  "name": "execute_sql",
  "arguments": {
    "query": "SELECT key, summary FROM issues WHERE status = 'Open' LIMIT 10",
    "limit": 100
  }
}
```

### 8. semantic_search
自然言語によるセマンティック検索を実行します。

```json
{
  "name": "semantic_search",
  "arguments": {
    "query": "ユーザーがログインできない問題",
    "project": "PROJ",
    "limit": 10
  }
}
```

> **注意**: セマンティック検索を使用するには:
> 1. `jira-db embeddings` コマンドで埋め込みを生成する必要があります
> 2. `OPENAI_API_KEY` 環境変数を設定する必要があります

## MCP プロトコルの詳細

### 初期化シーケンス

```
クライアント → サーバー: initialize
サーバー → クライアント: InitializeResult (capabilities)
クライアント → サーバー: notifications/initialized
```

### 主要なメソッド

| メソッド | 説明 |
|---------|------|
| `initialize` | 接続の初期化 |
| `notifications/initialized` | 初期化完了の通知 |
| `tools/list` | 利用可能なツールの一覧 |
| `tools/call` | ツールの実行 |
| `ping` | 接続確認 |

### エラーコード

| コード | 名前 | 説明 |
|--------|------|------|
| -32700 | Parse Error | 無効なJSON |
| -32600 | Invalid Request | 無効なリクエスト |
| -32601 | Method Not Found | メソッドが存在しない |
| -32602 | Invalid Params | 無効なパラメータ |
| -32603 | Internal Error | 内部エラー |

## セキュリティ考慮事項

1. **読み取り専用**: すべてのツールは読み取り専用です
2. **SQL制限**: `execute_sql`はSELECT文のみ許可し、危険なキーワード（INSERT、DELETE等）をブロック
3. **ローカル接続**: stdioモードはローカル実行のみ
4. **HTTP認証**: HTTPモードは127.0.0.1にのみバインド（デフォルト）

## 参考リンク

- [MCP仕様書](https://modelcontextprotocol.io/specification)
- [JSON-RPC 2.0仕様](https://www.jsonrpc.org/specification)
- [Claude Desktop MCP設定](https://docs.anthropic.com/claude/docs/desktop)
