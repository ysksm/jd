# jira-db アーキテクチャドキュメント

## 概要

jira-dbは、JIRAデータをローカルDuckDBデータベースに同期し、オフライン検索・分析を可能にするツールです。
クリーンアーキテクチャに基づいた設計で、CLI、MCP Server、GUI（Tauri）など複数のインターフェースに対応しています。

## ワークスペース構成

```
jira-db/
├── Cargo.toml                    # ワークスペース定義
├── crates/
│   ├── jira-db-core/             # コアライブラリ（全バイナリで共有）
│   ├── jira-db-cli/              # CLIバイナリ
│   ├── jira-db-mcp/              # MCP Serverバイナリ
│   ├── jira-db-tauri/            # Tauri GUIアプリ
│   ├── jira-db-web/              # Webサーバー（ActixWeb + Angular）
│   └── jira-db-service/          # Web/Tauri共有サービスレイヤー
├── frontend/                     # Angular フロントエンド
├── docs/                         # ドキュメント
├── typespec/                     # API仕様定義
└── .claude/                      # Claude Code設定
    ├── skills/                   # AIスキル定義
    └── commands/                 # AIコマンド定義
```

## レイヤーアーキテクチャ

```
┌─────────────────────────────────────────────────────────────┐
│                  Presentation Layer                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │  jira-db-cli│  │ jira-db-mcp │  │   jira-db-tauri     │  │
│  │    (CLI)    │  │(MCP Server) │  │      (GUI)          │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                     jira-db-core                             │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              Application Layer                       │    │
│  │  ┌─────────────────┐  ┌─────────────────────────┐   │    │
│  │  │    Use Cases    │  │       Services          │   │    │
│  │  │ - SyncProject   │  │   - JiraService         │   │    │
│  │  │ - SearchIssues  │  │                         │   │    │
│  │  │ - Generate...   │  └─────────────────────────┘   │    │
│  │  └─────────────────┘                                │    │
│  └─────────────────────────────────────────────────────┘    │
│                            │                                 │
│                            ▼                                 │
│  ┌─────────────────────────────────────────────────────┐    │
│  │                Domain Layer                          │    │
│  │  ┌───────────────┐    ┌─────────────────────────┐   │    │
│  │  │   Entities    │    │    Repository Traits    │   │    │
│  │  │ - Project     │    │  - IssueRepository      │   │    │
│  │  │ - Issue       │    │  - ProjectRepository    │   │    │
│  │  │ - Metadata    │    │  - MetadataRepository   │   │    │
│  │  └───────────────┘    └─────────────────────────┘   │    │
│  └─────────────────────────────────────────────────────┘    │
│                            │                                 │
│                            ▼                                 │
│  ┌─────────────────────────────────────────────────────┐    │
│  │             Infrastructure Layer                     │    │
│  │  ┌──────────────┐  ┌────────────┐  ┌─────────────┐  │    │
│  │  │   Database   │  │   Config   │  │  External   │  │    │
│  │  │   (DuckDB)   │  │   (JSON)   │  │ (JIRA API)  │  │    │
│  │  └──────────────┘  └────────────┘  └─────────────┘  │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

## jira-db-core 詳細

### Domain Layer (`src/domain/`)

ビジネスロジックの中核。外部依存なし。

#### Entities (`domain/entities/`)
```
entities/
├── mod.rs              # エクスポート定義
├── project.rs          # Project構造体
├── issue.rs            # Issue構造体
├── change_history.rs   # ChangeHistoryItem構造体
└── metadata.rs         # Status, Priority, IssueType等
```

#### Repository Traits (`domain/repositories/`)
```
repositories/
├── mod.rs                        # エクスポート定義
├── project_repository.rs         # ProjectRepository trait
├── issue_repository.rs           # IssueRepository trait + SearchParams
├── metadata_repository.rs        # MetadataRepository trait
├── sync_history_repository.rs    # SyncHistoryRepository trait
└── change_history_repository.rs  # ChangeHistoryRepository trait
```

#### Error (`domain/error.rs`)
```rust
pub enum DomainError {
    NotFound(String),
    Database(String),
    JiraApi(String),
    Config(String),
    Validation(String),
    Io(String),
}

pub type DomainResult<T> = Result<T, DomainError>;
```

### Application Layer (`src/application/`)

ユースケースとサービスインターフェース。

#### Use Cases (`application/use_cases/`)
```
use_cases/
├── mod.rs                    # エクスポート定義
├── sync_project_list.rs      # プロジェクト一覧同期
├── sync_project.rs           # プロジェクトデータ同期
├── search_issues.rs          # イシュー検索
├── get_change_history.rs     # 変更履歴取得
├── get_project_metadata.rs   # メタデータ取得
├── generate_embeddings.rs    # ベクトル埋め込み生成
├── generate_report.rs        # レポート生成
└── create_test_ticket.rs     # テストチケット作成
```

#### Services (`application/services/`)
```rust
pub trait JiraService: Send + Sync {
    async fn fetch_projects(&self) -> DomainResult<Vec<Project>>;
    async fn fetch_issues(&self, project_key: &str, ...) -> DomainResult<Vec<Issue>>;
    async fn fetch_metadata(&self, project: &Project) -> DomainResult<ProjectMetadata>;
}
```

### Infrastructure Layer (`src/infrastructure/`)

外部システムとの連携。

#### Database (`infrastructure/database/`)
```
database/
├── mod.rs                # エクスポート定義
├── connection.rs         # Database, DbConnection型
├── schema.rs             # スキーマ初期化SQL
└── repositories/
    ├── mod.rs
    ├── project_repository.rs
    ├── issue_repository.rs
    ├── metadata_repository.rs
    ├── sync_history_repository.rs
    ├── change_history_repository.rs
    └── embeddings_repository.rs
```

#### External (`infrastructure/external/`)
```
external/
├── mod.rs
├── jira/
│   ├── mod.rs
│   └── client.rs         # JiraApiClient
└── embeddings/
    ├── mod.rs            # EmbeddingProvider trait
    ├── openai.rs         # OpenAI埋め込み
    ├── ollama.rs         # Ollama埋め込み
    └── cohere.rs         # Cohere埋め込み
```

#### Config (`infrastructure/config/`)
```
config/
├── mod.rs
└── settings.rs           # Settings, JiraConfig, DatabaseConfig等
```

### Report Layer (`src/report/`)
```
report/
├── mod.rs
├── static_report.rs      # 静的HTMLレポート
└── interactive/
    ├── mod.rs
    ├── html.rs           # HTMLテンプレート
    ├── css.rs            # スタイル
    └── js.rs             # JavaScript
```

## jira-db-cli 詳細

```
src/
├── main.rs               # エントリーポイント
└── cli/
    ├── mod.rs
    ├── commands.rs       # Clap定義（Cli, Commands enum）
    └── handlers.rs       # コマンドハンドラー実装
```

### コマンド構造
```
jira-db
├── init [--interactive]          # 設定ファイル初期化
├── project
│   ├── init                      # JIRA からプロジェクト一覧取得
│   ├── list [--verbose]          # プロジェクト一覧表示
│   ├── enable <KEY>              # 同期有効化
│   └── disable <KEY>             # 同期無効化
├── sync [--project <KEY>]        # データ同期
├── search <QUERY> [--project] [--status] [--limit]  # 検索
├── metadata --project <KEY> [--type]                # メタデータ表示
├── history <ISSUE_KEY> [--field] [--limit]          # 変更履歴
├── embeddings [--provider] [--project] [--force]    # 埋め込み生成
├── report [--interactive] [--output]                # レポート生成
└── config
    ├── show                      # 設定表示
    └── set <KEY> <VALUE>         # 設定変更
```

## jira-db-mcp 詳細

```
src/
├── main.rs               # エントリーポイント
├── config.rs             # サーバー設定
├── server.rs             # MCPサーバーロジック
├── protocol/             # JSON-RPC 2.0 + MCP型
├── handlers/             # リクエストハンドラー
├── tools/                # ツール定義と実装
└── transport/
    ├── stdio.rs          # 標準入出力トランスポート
    └── http.rs           # HTTP/SSEトランスポート
```

### MCP ツール一覧
| ツール名 | 説明 | パラメータ |
|---------|------|----------|
| search_issues | 全文検索 | query, project?, status?, limit? |
| get_issue | イシュー詳細取得 | issue_key |
| get_issue_history | 変更履歴取得 | issue_key |
| list_projects | プロジェクト一覧 | - |
| get_project_metadata | メタデータ取得 | project_key |
| get_schema | DBスキーマ取得 | - |
| execute_sql | SQL実行（読み取り専用） | query |
| semantic_search | セマンティック検索 | query, limit? |

## データベーススキーマ

### コアテーブル
```sql
-- プロジェクト
CREATE TABLE projects (
    id VARCHAR PRIMARY KEY,
    key VARCHAR NOT NULL UNIQUE,
    name VARCHAR NOT NULL,
    description TEXT,
    raw_data JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- イシュー（raw_dataにchangelog含む）
CREATE TABLE issues (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL,
    project_key VARCHAR NOT NULL,
    key VARCHAR NOT NULL,
    summary TEXT NOT NULL,
    description TEXT,
    status VARCHAR,
    priority VARCHAR,
    issue_type VARCHAR,
    assignee VARCHAR,
    reporter VARCHAR,
    created_date TIMESTAMP,
    updated_date TIMESTAMP,
    raw_data JSON,
    synced_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- 変更履歴（正規化済み）
CREATE TABLE issue_change_history (
    id INTEGER PRIMARY KEY,
    issue_id VARCHAR NOT NULL,
    issue_key VARCHAR NOT NULL,
    history_id VARCHAR NOT NULL,
    author_account_id VARCHAR,
    author_display_name VARCHAR,
    field VARCHAR NOT NULL,
    field_type VARCHAR,
    from_value TEXT,
    from_string TEXT,
    to_value TEXT,
    to_string TEXT,
    changed_at TIMESTAMP NOT NULL
);
```

### メタデータテーブル
```sql
-- すべてのメタデータテーブルは (project_id, name) を複合主キーとして使用
CREATE TABLE statuses (
    project_id VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    description TEXT,
    category VARCHAR,
    PRIMARY KEY (project_id, name)
);

-- priorities, issue_types, labels, components, fix_versions も同様の構造
```

### ベクトル検索テーブル
```sql
-- DuckDB VSS拡張を使用
CREATE TABLE issue_embeddings (
    issue_id VARCHAR PRIMARY KEY,
    issue_key VARCHAR NOT NULL,
    embedding FLOAT[],
    model VARCHAR NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- HNSWインデックス
CREATE INDEX embedding_idx ON issue_embeddings USING HNSW (embedding);
```

## 設定ファイル

`./settings.json`（カレントディレクトリ）:
```json
{
  "jira": {
    "endpoint": "https://your-domain.atlassian.net",
    "username": "user@example.com",
    "api_key": "your-api-key"
  },
  "projects": [
    {
      "id": "10000",
      "key": "PROJ",
      "name": "Project Name",
      "sync_enabled": true,
      "last_synced": "2025-01-01T00:00:00Z"
    }
  ],
  "database": {
    "path": "./data/jira.duckdb"
  },
  "embeddings": {
    "provider": "ollama",
    "model": "nomic-embed-text",
    "endpoint": "http://localhost:11434",
    "auto_generate": false
  }
}
```

## 依存関係

### 主要クレート
| クレート | 用途 |
|---------|------|
| duckdb | 埋め込みアナリティカルDB |
| reqwest | HTTP クライアント |
| tokio | 非同期ランタイム |
| serde / serde_json | シリアライゼーション |
| clap | CLI引数パース |
| thiserror | カスタムエラー |
| chrono | 日時処理 |
| indicatif | プログレスバー |
| base64 | 認証ヘッダーエンコード |

## JIRA API 統合

### エンドポイント
- 検索: `POST /rest/api/3/search/jql`（v3、非推奨でない）
- プロジェクト: `GET /rest/api/3/project`
- メタデータ: `GET /rest/api/3/project/{key}/statuses` 等

### 認証
HTTP Basic認証: `Authorization: Basic base64(username:api_token)`

### ページネーション
- maxResults: 100
- startAt: ページオフセット
- 全件取得まで繰り返し

## 拡張ポイント

### 新しいエンティティを追加
1. `domain/entities/` にエンティティ定義
2. `domain/repositories/` にリポジトリtrait
3. `infrastructure/database/repositories/` に実装
4. 必要に応じてスキーマ更新

### 新しいユースケースを追加
1. `application/use_cases/` にユースケース定義
2. 依存リポジトリをコンストラクタで受け取る
3. `execute()` メソッドを実装
4. `lib.rs` でエクスポート

### 新しいCLIコマンドを追加
1. `jira-db-cli/src/cli/commands.rs` にコマンド定義
2. `handlers.rs` にハンドラー実装
3. `main.rs` のマッチに追加

### 新しいMCPツールを追加
1. `jira-db-mcp/src/tools/` にツール定義
2. Tool traitを実装
3. ハンドラーに登録
