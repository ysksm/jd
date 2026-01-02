# jira-db Database Schema Skill

このスキルはjira-dbのDuckDBデータベーススキーマの詳細を提供します。

## スキーマファイルの場所

- `crates/jira-db-core/src/infrastructure/database/schema.rs` - メインスキーマ定義
- `crates/jira-db-core/src/infrastructure/database/repositories/embeddings_repository.rs` - Embeddingsスキーマ

## テーブル一覧

### Core Tables

#### projects
JIRAプロジェクトのメタデータを格納

| Column | Type | Description |
|--------|------|-------------|
| id | VARCHAR | PRIMARY KEY |
| key | VARCHAR | NOT NULL - プロジェクトキー (例: "PROJ") |
| name | VARCHAR | NOT NULL - プロジェクト名 |
| description | TEXT | プロジェクト説明 |
| raw_data | JSON | JIRA APIレスポンスの生データ |
| created_at | TIMESTAMP | 作成日時 (DEFAULT CURRENT_TIMESTAMP) |
| updated_at | TIMESTAMP | 更新日時 (DEFAULT CURRENT_TIMESTAMP) |

#### issues
JIRAイシューの全データを格納

| Column | Type | Description |
|--------|------|-------------|
| id | VARCHAR | PRIMARY KEY |
| project_id | VARCHAR | NOT NULL - 所属プロジェクトID |
| key | VARCHAR | NOT NULL - イシューキー (例: "PROJ-123") |
| summary | TEXT | NOT NULL - サマリー |
| description | TEXT | 説明 |
| status | VARCHAR | ステータス |
| priority | VARCHAR | 優先度 |
| assignee | VARCHAR | 担当者 |
| reporter | VARCHAR | 報告者 |
| issue_type | VARCHAR | イシュータイプ |
| resolution | VARCHAR | 解決状況 |
| labels | VARCHAR | ラベル (カンマ区切り) |
| components | VARCHAR | コンポーネント (カンマ区切り) |
| fix_versions | VARCHAR | 修正バージョン (カンマ区切り) |
| sprint | VARCHAR | スプリント名 |
| parent_key | VARCHAR | 親イシューキー |
| created_date | TIMESTAMP | JIRA作成日時 |
| updated_date | TIMESTAMP | JIRA更新日時 |
| raw_data | JSON | 完全なAPIレスポンス (changelog含む) |
| synced_at | TIMESTAMP | 同期日時 (DEFAULT CURRENT_TIMESTAMP) |

**インデックス:**
- `idx_issues_project` ON (project_id)
- `idx_issues_key` ON (key)
- `idx_issues_status` ON (status)

#### sync_history
同期操作の履歴を追跡

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER | PRIMARY KEY (シーケンス: sync_history_id_seq) |
| project_id | VARCHAR | NOT NULL - プロジェクトID |
| sync_type | VARCHAR | NOT NULL - 同期タイプ |
| started_at | TIMESTAMP | NOT NULL - 開始日時 |
| completed_at | TIMESTAMP | 完了日時 |
| status | VARCHAR | NOT NULL - ステータス |
| items_synced | INTEGER | 同期アイテム数 |
| error_message | TEXT | エラーメッセージ |

**インデックス:**
- `idx_sync_history_project` ON (project_id)

#### issue_change_history
イシューの変更履歴を正規化して格納

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER | PRIMARY KEY (シーケンス: change_history_id_seq) |
| issue_id | VARCHAR | NOT NULL - イシューID |
| issue_key | VARCHAR | NOT NULL - イシューキー |
| history_id | VARCHAR | NOT NULL - JIRA履歴エントリID |
| author_account_id | VARCHAR | 変更者のアカウントID |
| author_display_name | VARCHAR | 変更者の表示名 |
| field | VARCHAR | NOT NULL - 変更されたフィールド |
| field_type | VARCHAR | フィールドタイプ |
| from_value | VARCHAR | 変更前の値 |
| from_string | VARCHAR | 変更前の表示文字列 |
| to_value | VARCHAR | 変更後の値 |
| to_string | VARCHAR | 変更後の表示文字列 |
| changed_at | TIMESTAMP | NOT NULL - 変更日時 |
| created_at | TIMESTAMP | レコード作成日時 (DEFAULT CURRENT_TIMESTAMP) |

**インデックス:**
- `idx_change_history_issue_id` ON (issue_id)
- `idx_change_history_issue_key` ON (issue_key)
- `idx_change_history_field` ON (field)
- `idx_change_history_changed_at` ON (changed_at)

### Metadata Tables

全てのメタデータテーブルは複合主キー `(project_id, name)` を使用

#### statuses
プロジェクトのステータス定義

| Column | Type | Description |
|--------|------|-------------|
| project_id | VARCHAR | NOT NULL - プロジェクトID |
| name | VARCHAR | NOT NULL - ステータス名 |
| description | VARCHAR | 説明 |
| category | VARCHAR | カテゴリ (例: "To Do", "In Progress", "Done") |
| created_at | TIMESTAMP | NOT NULL - 作成日時 |
| updated_at | TIMESTAMP | NOT NULL - 更新日時 |

#### priorities
優先度定義

| Column | Type | Description |
|--------|------|-------------|
| project_id | VARCHAR | NOT NULL - プロジェクトID |
| name | VARCHAR | NOT NULL - 優先度名 |
| description | VARCHAR | 説明 |
| icon_url | VARCHAR | アイコンURL |
| created_at | TIMESTAMP | NOT NULL - 作成日時 |
| updated_at | TIMESTAMP | NOT NULL - 更新日時 |

#### issue_types
イシュータイプ定義

| Column | Type | Description |
|--------|------|-------------|
| project_id | VARCHAR | NOT NULL - プロジェクトID |
| name | VARCHAR | NOT NULL - タイプ名 |
| description | VARCHAR | 説明 |
| icon_url | VARCHAR | アイコンURL |
| subtask | BOOLEAN | サブタスクフラグ (DEFAULT false) |
| created_at | TIMESTAMP | NOT NULL - 作成日時 |
| updated_at | TIMESTAMP | NOT NULL - 更新日時 |

#### labels
ラベル定義

| Column | Type | Description |
|--------|------|-------------|
| project_id | VARCHAR | NOT NULL - プロジェクトID |
| name | VARCHAR | NOT NULL - ラベル名 |
| created_at | TIMESTAMP | NOT NULL - 作成日時 |
| updated_at | TIMESTAMP | NOT NULL - 更新日時 |

#### components
コンポーネント定義

| Column | Type | Description |
|--------|------|-------------|
| project_id | VARCHAR | NOT NULL - プロジェクトID |
| name | VARCHAR | NOT NULL - コンポーネント名 |
| description | VARCHAR | 説明 |
| lead | VARCHAR | リード担当者 |
| created_at | TIMESTAMP | NOT NULL - 作成日時 |
| updated_at | TIMESTAMP | NOT NULL - 更新日時 |

#### fix_versions
バージョン/リリース定義

| Column | Type | Description |
|--------|------|-------------|
| project_id | VARCHAR | NOT NULL - プロジェクトID |
| name | VARCHAR | NOT NULL - バージョン名 |
| description | VARCHAR | 説明 |
| released | BOOLEAN | リリース済みフラグ (DEFAULT false) |
| release_date | TIMESTAMP | リリース日 |
| created_at | TIMESTAMP | NOT NULL - 作成日時 |
| updated_at | TIMESTAMP | NOT NULL - 更新日時 |

### Embeddings Table

#### issue_embeddings
セマンティック検索用のベクトルEmbeddings

| Column | Type | Description |
|--------|------|-------------|
| issue_id | VARCHAR | PRIMARY KEY |
| issue_key | VARCHAR | NOT NULL - イシューキー |
| embedding | FLOAT[1536] | NOT NULL - ベクトルEmbedding |
| embedded_text | TEXT | NOT NULL - Embedding対象のテキスト |
| created_at | TIMESTAMP | 作成日時 (DEFAULT CURRENT_TIMESTAMP) |

**インデックス:**
- `idx_embeddings_hnsw` USING HNSW (embedding) WITH (metric = 'cosine')

**必要な拡張:**
- DuckDB VSS extension (`INSTALL vss; LOAD vss;`)

## リレーションシップ

```
projects (1) ──────── (N) issues
    │                      │
    │                      └── (N) issue_change_history
    │
    ├── (N) statuses
    ├── (N) priorities
    ├── (N) issue_types
    ├── (N) labels
    ├── (N) components
    └── (N) fix_versions

issues (1) ──────── (1) issue_embeddings
```

## SQL例

### イシュー検索
```sql
SELECT key, summary, status, priority, assignee
FROM issues
WHERE project_id = 'PROJECT_ID'
  AND status = 'Open'
ORDER BY created_date DESC
LIMIT 20;
```

### 変更履歴の取得
```sql
SELECT field, from_string, to_string, author_display_name, changed_at
FROM issue_change_history
WHERE issue_key = 'PROJ-123'
ORDER BY changed_at DESC;
```

### メタデータの取得
```sql
SELECT name, category FROM statuses WHERE project_id = 'PROJECT_ID';
SELECT name, description FROM priorities WHERE project_id = 'PROJECT_ID';
SELECT name, subtask FROM issue_types WHERE project_id = 'PROJECT_ID';
```

### セマンティック検索
```sql
SELECT e.issue_key, i.summary,
       array_cosine_distance(e.embedding, ?::FLOAT[1536]) as distance
FROM issue_embeddings e
JOIN issues i ON e.issue_id = i.id
ORDER BY distance ASC
LIMIT 10;
```

## マイグレーション

スキーマのマイグレーションは `Schema::run_migrations()` で管理:
- カラムの追加: `add_column_if_not_exists()` を使用
- 既存: `issues.sprint` カラムの追加

## 注意事項

- `raw_data` JSONには完全なJIRA APIレスポンス（changelog含む）が格納される
- メタデータテーブルは `ON CONFLICT DO UPDATE` でupsert
- Embeddingsは1536次元（OpenAI text-embedding-3-small用）
- HNSWインデックスはcosine距離を使用
