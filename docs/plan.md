# jira-db 技術実装計画

## アーキテクチャ概要

```
┌─────────────────────────────────────────────────────────┐
│                      CLI Interface                       │
└─────────────────────────────────────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────┐
│                   Configuration Manager                  │
│  - settings.json の読み書き                              │
│  - 初回起動時の設定ファイル生成                           │
└─────────────────────────────────────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────┐
│                     JIRA API Client                      │
│  (jira-api ライブラリを使用)                             │
│  - プロジェクト一覧取得                                   │
│  - イシュー取得                                          │
└─────────────────────────────────────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────┐
│                    Data Sync Manager                     │
│  - 同期ロジック                                          │
│  - 差分検出                                              │
│  - バッチ処理                                            │
└─────────────────────────────────────────────────────────┘
                            │
┌─────────────────────────────────────────────────────────┐
│                   Database Manager                       │
│  (DuckDB)                                                │
│  - RAWデータテーブル                                      │
│  - インデックス管理                                       │
└─────────────────────────────────────────────────────────┘
```

## 技術スタック

### コア技術
- **言語**: Rust (edition 2024)
- **データベース**: DuckDB
- **HTTP Client**: jira-api (内部でreqwestを使用)

### 必要なクレート
```toml
[dependencies]
jira-api = { git = "https://github.com/ysksm/jira-api" }
duckdb = "1.0"           # DuckDBバインディング
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"       # settings.json用
tokio = { version = "1", features = ["full"] }  # 非同期ランタイム
anyhow = "1.0"           # エラーハンドリング
thiserror = "1.0"        # カスタムエラー定義
clap = { version = "4", features = ["derive"] }  # CLI引数パース
log = "0.4"              # ログ
env_logger = "0.11"      # ログ出力
dirs = "5.0"             # 設定ファイルパス取得
```

## モジュール構成

```
src/
├── main.rs                    # エントリーポイント
├── config/
│   ├── mod.rs
│   ├── settings.rs            # Settings構造体と読み書き
│   └── validator.rs           # 設定値のバリデーション
├── jira/
│   ├── mod.rs
│   ├── client.rs              # JIRA APIクライアントのラッパー
│   └── models.rs              # JIRAデータモデル
├── sync/
│   ├── mod.rs
│   ├── manager.rs             # 同期マネージャー
│   └── strategy.rs            # 同期戦略（増分/フル）
├── db/
│   ├── mod.rs
│   ├── connection.rs          # DuckDB接続管理
│   ├── schema.rs              # テーブルスキーマ定義
│   ├── repository.rs          # データアクセス層
│   └── index.rs               # 検索インデックス生成
├── cli/
│   ├── mod.rs
│   └── commands.rs            # CLIコマンド定義
└── error.rs                   # エラー型定義
```

## データモデル

### settings.json
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
      "sync_enabled": false,
      "last_synced": null
    }
  ],
  "database": {
    "path": "./data/jira.duckdb"
  }
}
```

### DuckDBスキーマ

#### projectsテーブル
```sql
CREATE TABLE IF NOT EXISTS projects (
    id VARCHAR PRIMARY KEY,
    key VARCHAR NOT NULL,
    name VARCHAR NOT NULL,
    description TEXT,
    raw_data JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

#### issuesテーブル
```sql
CREATE TABLE IF NOT EXISTS issues (
    id VARCHAR PRIMARY KEY,
    project_id VARCHAR NOT NULL,
    key VARCHAR NOT NULL,
    summary TEXT NOT NULL,
    description TEXT,
    status VARCHAR,
    priority VARCHAR,
    assignee VARCHAR,
    reporter VARCHAR,
    created_date TIMESTAMP,
    updated_date TIMESTAMP,
    raw_data JSON,
    synced_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE INDEX IF NOT EXISTS idx_issues_project ON issues(project_id);
CREATE INDEX IF NOT EXISTS idx_issues_key ON issues(key);
CREATE INDEX IF NOT EXISTS idx_issues_status ON issues(status);
```

#### sync_historyテーブル
```sql
CREATE TABLE IF NOT EXISTS sync_history (
    id INTEGER PRIMARY KEY,
    project_id VARCHAR NOT NULL,
    sync_type VARCHAR NOT NULL,  -- 'full' or 'incremental'
    started_at TIMESTAMP NOT NULL,
    completed_at TIMESTAMP,
    status VARCHAR NOT NULL,     -- 'running', 'completed', 'failed'
    items_synced INTEGER,
    error_message TEXT,
    FOREIGN KEY (project_id) REFERENCES projects(id)
);
```

## 実装フェーズ

### Phase 1: 基盤構築
1. プロジェクト構造のセットアップ
2. 依存関係の追加
3. エラーハンドリングの実装
4. ロギング設定

### Phase 2: 設定管理
1. Settings構造体の定義
2. 設定ファイルの読み書き実装
3. 初回起動時の設定生成
4. 設定バリデーション

### Phase 3: データベース層
1. DuckDB接続管理
2. スキーマ定義と初期化
3. リポジトリパターン実装
4. マイグレーション機能

### Phase 4: JIRA統合
1. jira-apiクライアントのラッパー実装
2. プロジェクト一覧取得
3. イシュー取得（ページネーション対応）
4. エラーハンドリングとリトライ

### Phase 5: 同期機能
1. 同期マネージャー実装
2. フル同期ロジック
3. 増分同期ロジック（オプション）
4. 同期履歴の記録

### Phase 6: CLI実装
1. CLIコマンド定義
2. init コマンド（初期設定）
3. sync コマンド（同期実行）
4. list コマンド（プロジェクト一覧表示）

### Phase 7: 検索機能
1. 検索インデックス生成
2. 基本的な検索クエリ実装
3. フィルタリング機能

## エラーハンドリング戦略

### カスタムエラー型
```rust
#[derive(Debug, thiserror::Error)]
pub enum JiraDbError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("JIRA API error: {0}")]
    JiraApi(#[from] jira_api::Error),

    #[error("Database error: {0}")]
    Database(#[from] duckdb::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}
```

## パフォーマンス考慮事項

1. **バッチ処理**: イシューは100件ずつバッチで取得・保存
2. **並行処理**: 複数プロジェクトの同期は並行実行可能に
3. **インデックス**: 検索頻度の高いカラムにインデックスを作成
4. **キャッシング**: APIレスポンスの適切なキャッシュ

## セキュリティ考慮事項

1. **APIキー保護**: settings.jsonのパーミッション制限（600）
2. **環境変数**: 環境変数からの設定読み込みサポート
3. **ログ**: APIキーをログに出力しない

## テスト戦略

1. **ユニットテスト**: 各モジュールの単体テスト
2. **統合テスト**: JIRA API連携のモックテスト
3. **E2Eテスト**: 実際のワークフロー確認（手動）

## 将来の拡張性

- 複数JIRA環境のサポート
- Webhookによるリアルタイム同期
- GraphQL APIのサポート
- Webインターフェース
- データエクスポート機能（CSV, Excel）
