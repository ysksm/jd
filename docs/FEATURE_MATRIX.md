# jira-db 機能一覧表

このドキュメントは jira-db の全機能と CLI / Tauri での対応状況をまとめたものです。

## 機能対応マトリクス

| カテゴリ | 機能 | CLI | Tauri | ステータス |
|---------|------|:---:|:-----:|:--------:|
| **設定管理** | 初期設定 | ✅ | ✅ | 完了 |
| | 設定表示 | ✅ | ✅ | 完了 |
| | 設定更新 | ✅ | ✅ | 完了 |
| **プロジェクト管理** | プロジェクト初期化 | ✅ | ✅ | 完了 |
| | プロジェクト一覧 | ✅ | ✅ | 完了 |
| | 同期有効化 | ✅ | ✅ | 完了 |
| | 同期無効化 | ✅ | ✅ | 完了 |
| **データ同期** | Issue同期 | ✅ | ✅ | 完了 |
| | 増分同期 | ✅ | ✅ | 完了 |
| | 中断再開（チェックポイント） | ✅ | ✅ | 完了 |
| | メタデータ同期 | ✅ | ✅ | 完了 |
| | フィールド定義同期 | ✅ | ✅ | 完了 |
| | フィールド展開 | ✅ | ✅ | 完了 |
| | 同期ステータス確認 | - | ⚠️ | 一部実装 |
| **検索・取得** | 全文検索 | ✅ | ✅ | 完了 |
| | Issue詳細取得 | - | ✅ | 完了 |
| | セマンティック検索 | - | ✅ | 完了 |
| **履歴・スナップショット** | 変更履歴表示 | ✅ | ✅ | 完了 |
| | スナップショット生成 | ✅ | - | 完了 |
| | スナップショット表示 | ✅ | - | 完了 |
| **メタデータ** | メタデータ表示 | ✅ | ✅ | 完了 |
| **埋め込み** | Embedding生成 | ✅ | ✅ | 完了 |
| **レポート** | HTMLレポート生成 | ✅ | ✅ | 完了 |
| **SQL** | SQL実行 | - | ✅ | 完了 |
| | スキーマ取得 | - | ✅ | 完了 |
| | クエリ保存・管理 | - | ✅ | 完了 |
| **テスト** | テストチケット作成 | ✅ | - | 完了 |

**凡例**: ✅ 対応済み | ⚠️ 一部実装 | - 未対応

---

## 操作導線一覧

### 1. 設定管理

#### 初期設定
| 操作 | CLI | Tauri |
|------|-----|-------|
| 対話式セットアップ | `jira-db init --interactive` | 設定画面から入力 |
| 非対話式セットアップ | `jira-db init` | `config_initialize()` API |

#### 設定表示・更新
| 操作 | CLI | Tauri |
|------|-----|-------|
| 設定表示 | `jira-db config show` | `config_get()` API |
| エンドポイント変更 | `jira-db config set jira.endpoint <URL>` | `config_update()` API |
| ユーザー名変更 | `jira-db config set jira.username <USER>` | 設定画面から入力 |
| APIキー変更 | `jira-db config set jira.api_key <KEY>` | 設定画面から入力 |
| DB パス変更 | `jira-db config set database.path <PATH>` | 設定画面から入力 |

---

### 2. プロジェクト管理

| 操作 | CLI | Tauri |
|------|-----|-------|
| JIRAからプロジェクト取得 | `jira-db project init` | `projects_initialize()` API |
| プロジェクト一覧表示 | `jira-db project list` | `projects_list()` API |
| 詳細一覧表示 | `jira-db project list --verbose` | - |
| 同期を有効化 | `jira-db project enable <KEY>` | `projects_enable(key)` API |
| 同期を無効化 | `jira-db project disable <KEY>` | `projects_disable(key)` API |

---

### 3. データ同期

| 操作 | CLI | Tauri |
|------|-----|-------|
| 全プロジェクト同期 | `jira-db sync` | `sync_execute()` API |
| 特定プロジェクト同期 | `jira-db sync --project <KEY>` | `sync_execute(project_key)` API |
| 強制再同期 | `jira-db sync --force` | `sync_execute(null, true)` API |
| フィールド定義同期 | `jira-db fields sync` | `fields_sync()` API |
| フィールド一覧表示 | `jira-db fields list` | `fields_list()` API |
| カスタムフィールドのみ | `jira-db fields list --custom` | - |
| フィールド展開 | `jira-db fields expand` | `fields_expand()` API |
| フィールドフル同期 | `jira-db fields full` | `fields_full()` API |

---

### 4. Issue検索・取得

| 操作 | CLI | Tauri |
|------|-----|-------|
| 全文検索 | `jira-db search "<QUERY>"` | `issues_search(query)` API |
| プロジェクト絞り込み | `jira-db search "<QUERY>" --project <KEY>` | `issues_search(query, project)` API |
| ステータス絞り込み | `jira-db search "<QUERY>" --status <STATUS>` | `issues_search(query, null, status)` API |
| 担当者絞り込み | `jira-db search "<QUERY>" --assignee <USER>` | `issues_search(query, null, null, assignee)` API |
| 件数制限 | `jira-db search "<QUERY>" --limit 20` | `issues_search(..., limit)` API |
| オフセット指定 | `jira-db search "<QUERY>" --offset 10` | `issues_search(..., offset)` API |
| Issue詳細取得 | - | `issues_get(key)` API |
| セマンティック検索 | - | `embeddings_search(query)` API |

---

### 5. 変更履歴・スナップショット

| 操作 | CLI | Tauri |
|------|-----|-------|
| 変更履歴表示 | `jira-db history <ISSUE_KEY>` | `issues_history(key)` API |
| フィールド絞り込み | `jira-db history <KEY> --field status` | `issues_history(key, field)` API |
| 件数制限 | `jira-db history <KEY> --limit 50` | `issues_history(key, null, limit)` API |
| スナップショット生成 | `jira-db snapshots generate --project <KEY>` | - |
| スナップショット表示 | `jira-db snapshots show <ISSUE_KEY>` | - |
| 特定バージョン表示 | `jira-db snapshots show <KEY> --version <VER>` | - |

---

### 6. メタデータ

| 操作 | CLI | Tauri |
|------|-----|-------|
| 全メタデータ表示 | `jira-db metadata --project <KEY>` | `metadata_get(project_key)` API |
| ステータスのみ | `jira-db metadata --project <KEY> --type status` | `metadata_get(project_key, "status")` API |
| 優先度のみ | `jira-db metadata --project <KEY> --type priority` | `metadata_get(project_key, "priority")` API |
| Issue種別のみ | `jira-db metadata --project <KEY> --type issue-type` | `metadata_get(project_key, "issue-type")` API |
| ラベルのみ | `jira-db metadata --project <KEY> --type label` | `metadata_get(project_key, "label")` API |
| コンポーネントのみ | `jira-db metadata --project <KEY> --type component` | `metadata_get(project_key, "component")` API |
| バージョンのみ | `jira-db metadata --project <KEY> --type version` | `metadata_get(project_key, "version")` API |

---

### 7. Embedding（ベクトル埋め込み）

| 操作 | CLI | Tauri |
|------|-----|-------|
| Embedding生成（デフォルト） | `jira-db embeddings` | `embeddings_generate()` API |
| プロジェクト指定 | `jira-db embeddings --project <KEY>` | `embeddings_generate(null, project_key)` API |
| 強制再生成 | `jira-db embeddings --force` | `embeddings_generate(null, null, true)` API |
| バッチサイズ指定 | `jira-db embeddings --batch-size 50` | `embeddings_generate(null, null, null, batch_size)` API |
| OpenAI使用 | `jira-db embeddings --provider openai` | `embeddings_generate("openai")` API |
| Ollama使用（ローカル） | `jira-db embeddings --provider ollama` | `embeddings_generate("ollama")` API |
| Cohere使用 | `jira-db embeddings --provider cohere` | `embeddings_generate("cohere")` API |
| モデル指定 | `jira-db embeddings --model <NAME>` | - |
| エンドポイント指定 | `jira-db embeddings --endpoint <URL>` | - |

---

### 8. レポート

| 操作 | CLI | Tauri |
|------|-----|-------|
| 静的レポート生成 | `jira-db report` | `reports_generate()` API |
| インタラクティブレポート | `jira-db report --interactive` | `reports_generate(null, true)` API |
| プロジェクト指定 | `jira-db report --project <KEY>` | `reports_generate(project_key)` API |
| 出力先指定 | `jira-db report --output <PATH>` | `reports_generate(null, null, output_path)` API |

---

### 9. SQL操作（Tauriのみ）

| 操作 | CLI | Tauri |
|------|-----|-------|
| SQL実行 | - | `sql_execute(query)` API |
| 結果件数制限 | - | `sql_execute(query, limit)` API |
| スキーマ取得（全テーブル） | - | `sql_get_schema()` API |
| 特定テーブルのスキーマ | - | `sql_get_schema(table_name)` API |
| クエリ保存 | - | `sql_query_save(name, query, description)` API |
| クエリ一覧 | - | `sql_query_list()` API |
| クエリ削除 | - | `sql_query_delete(id)` API |

---

### 10. テスト（CLIのみ）

| 操作 | CLI | Tauri |
|------|-----|-------|
| テストチケット作成 | `jira-db test-ticket --project <KEY>` | - |
| サマリー指定 | `jira-db test-ticket --project <KEY> --summary "..."` | - |
| 説明指定 | `jira-db test-ticket --project <KEY> --description "..."` | - |
| Issue種別指定 | `jira-db test-ticket --project <KEY> --type Task` | - |
| 複数作成 | `jira-db test-ticket --project <KEY> --count 5` | - |

---

## Embeddingプロバイダー比較

| プロバイダー | 環境変数 | デフォルトモデル | 次元数 | 特徴 |
|------------|---------|----------------|-------|------|
| OpenAI | `OPENAI_API_KEY` | text-embedding-3-small | 1536 | 高品質、有料 |
| Ollama | - (ローカル) | nomic-embed-text | 768 | 無料、ローカル実行 |
| Cohere | `COHERE_API_KEY` | embed-multilingual-v3.0 | 1024 | 多言語対応（日本語◎） |

---

## 典型的なワークフロー

### 初回セットアップ

```bash
# 1. 設定ファイル作成
jira-db init --interactive

# 2. JIRAからプロジェクト一覧取得
jira-db project init

# 3. 同期対象プロジェクトを有効化
jira-db project enable PROJ1
jira-db project enable PROJ2

# 4. データ同期
jira-db sync

# 5. フィールド定義とデータ展開
jira-db fields full
```

### 日常的な利用

```bash
# Issue検索
jira-db search "バグ" --project PROJ1 --status "In Progress"

# 変更履歴確認
jira-db history PROJ1-123

# メタデータ確認
jira-db metadata --project PROJ1 --type status

# レポート生成
jira-db report --interactive --project PROJ1
```

### セマンティック検索の準備

```bash
# Ollama（無料・ローカル）の場合
jira-db embeddings --provider ollama

# OpenAIの場合
export OPENAI_API_KEY=your-api-key
jira-db embeddings --provider openai

# Cohereの場合（日本語に強い）
export COHERE_API_KEY=your-api-key
jira-db embeddings --provider cohere
```

---

## 機能カバレッジ統計

| 項目 | 数 |
|------|---|
| 総機能数 | 26 |
| CLI対応 | 20 |
| Tauri対応 | 22 |
| 両方対応 | 16 |
| CLIのみ | 4 |
| Tauriのみ | 6 |
| 一部実装 | 1 |
