# jira-db

JIRAのプロジェクトとイシューをローカルのDuckDBデータベースに同期し、オフラインで高速に検索・分析できるツールです。CLI、デスクトップアプリ（Tauri）、Webサーバーの3つのインターフェースを提供します。

## クイックスタート

```bash
# 1. リポジトリをクローンしてビルド
git clone https://github.com/ysksm/jira-db.git
cd jira-db
cargo build --release

# 2. 設定ファイルを初期化
./target/release/jira-db init --interactive

# 3. プロジェクト一覧を取得
./target/release/jira-db project init

# 4. 同期するプロジェクトを有効化
./target/release/jira-db project enable <PROJECT_KEY>

# 5. データを同期
./target/release/jira-db sync

# 6. 検索
./target/release/jira-db search "バグ"
```

## 目次

- [特徴](#特徴)
- [前提条件](#前提条件)
- [インストール](#インストール)
- [使い方](#使い方)
  - [CLI](#cli)
  - [デスクトップアプリ（Tauri）](#デスクトップアプリtauri)
  - [MCPサーバー](#mcpサーバー)
- [コマンドリファレンス](#コマンドリファレンス)
- [増分同期](#増分同期)
- [セマンティック検索](#セマンティック検索)
- [データベーススキーマ](#データベーススキーマ)
- [トラブルシューティング](#トラブルシューティング)
- [開発](#開発)
- [ライセンス](#ライセンス)

## 特徴

- 🚀 JIRAデータをローカルに同期して高速アクセス
- 💾 DuckDBによる効率的なデータ保存
- 🔄 プロジェクト単位での同期制御
- ⚡ **増分同期**: 前回同期以降に更新されたイシューのみを取得
- 🔁 **中断再開可能**: 同期が中断しても最後のチェックポイントから再開
- 📊 RAWデータとしてJSON形式で完全なAPIレスポンス（全フィールド・変更履歴含む）を保存
- 🏷️ プロジェクトのメタデータ（ステータス、優先度、イシュータイプ、ラベル等）を自動同期
- 🛠️ 使いやすいCLIインターフェース
- 🖥️ **デスクトップアプリ**: Tauriベースのクロスプラットフォームアプリ
- 🌐 **Webサーバー**: チーム共有用のHTTP API
- 🔍 高速なフルテキスト検索とフィルタリング
- 🤖 **MCPサーバー**: AIアシスタント（Claude Desktop等）との連携
- 🧠 **セマンティック検索**: 複数の埋め込みプロバイダー（OpenAI、Ollama、Cohere）とDuckDB VSSによるベクトル検索

## 前提条件

### 必須

- **Rust 1.85以上** (Rust Edition 2024対応)
  ```bash
  rustc --version  # 1.85以上であることを確認
  ```

- **DuckDB** (システムライブラリ)
  ```bash
  # macOS
  brew install duckdb

  # Linux (Ubuntu/Debian)
  sudo apt-get install libduckdb-dev

  # Linux (Arch)
  sudo pacman -S duckdb
  ```

### JIRA APIキー

JIRA Cloud APIを使用するため、以下が必要です：
- JIRAインスタンスのURL (例: `https://your-domain.atlassian.net`)
- ユーザー名（メールアドレス）
- APIトークン（[Atlassian APIトークン作成ページ](https://id.atlassian.com/manage-profile/security/api-tokens)で作成）

## インストール

### ソースからビルド

```bash
# リポジトリをクローン
git clone https://github.com/ysksm/jira-db.git
cd jira-db

# ビルド（macOSの場合は環境変数設定が必要）
export LIBRARY_PATH="/opt/homebrew/lib:$LIBRARY_PATH"
cargo build --release

# バイナリをパスの通った場所にコピー（オプション）
cp target/release/jira-db /usr/local/bin/
```

ビルドに成功すると、以下の実行可能ファイルが生成されます：
- `target/release/jira-db` - CLI
- `target/release/jira-db-mcp` - MCPサーバー
- `target/release/jira-db-tauri` - デスクトップアプリ

## 使い方

### CLI

#### 1. 設定ファイルの初期化

```bash
# 対話的に設定（推奨）
jira-db init --interactive

# または、設定ファイルを手動編集
jira-db init
vim ./settings.json
```

設定ファイル例（`./settings.json`）:
```json
{
  "jira": {
    "endpoint": "https://your-domain.atlassian.net",
    "username": "your-email@example.com",
    "api_key": "your-api-token-here"
  },
  "projects": [],
  "database": {
    "path": "./data/jira.duckdb"
  }
}
```

#### 2. プロジェクトの設定と同期

```bash
# JIRAからプロジェクト一覧を取得
jira-db project init

# プロジェクト一覧を確認
jira-db project list

# 同期するプロジェクトを有効化
jira-db project enable PROJ

# データを同期
jira-db sync
```

#### 3. 検索とデータ活用

```bash
# イシューを検索
jira-db search "バグ" --project PROJ

# メタデータを確認
jira-db metadata --project PROJ

# 変更履歴を確認
jira-db history PROJ-123

# HTMLレポートを生成
jira-db report --interactive
```

### デスクトップアプリ（Tauri）

Angularベースのモダンなデスクトップアプリを提供します。

```bash
# デスクトップアプリをビルド
cd crates/jira-db-tauri
npm install
npm run tauri build

# 開発モードで起動
npm run tauri dev
```

**機能:**
- プロジェクト管理（一覧、有効化/無効化）
- イシュー検索（フィルタ、詳細表示）
- データ同期（進捗表示）
- 設定管理
- レポート生成

### MCPサーバー

AIアシスタント（Claude Desktop等）からJIRAデータにアクセスできます。

```bash
# stdioモード（Claude Desktop、VS Code等用）
cargo run -p jira-db-mcp -- --database ./data/jira.duckdb

# HTTPモード（Webクライアント用）
cargo run -p jira-db-mcp -- --database ./data/jira.duckdb --http --port 3000
```

**Claude Desktopでの設定例:**

`claude_desktop_config.json`:
```json
{
  "mcpServers": {
    "jira-db": {
      "command": "/path/to/jira-db-mcp",
      "args": ["--database", "/path/to/jira.duckdb"],
      "env": {
        "OPENAI_API_KEY": "sk-..."
      }
    }
  }
}
```

**利用可能なツール:**
| ツール名 | 説明 |
|---------|------|
| `search_issues` | テキスト検索（プロジェクト、ステータス、担当者フィルタ） |
| `get_issue` | イシュー詳細取得 |
| `get_issue_history` | 変更履歴取得 |
| `list_projects` | プロジェクト一覧 |
| `get_project_metadata` | メタデータ取得 |
| `get_schema` | DBスキーマ取得 |
| `execute_sql` | 読み取り専用SQL実行 |
| `semantic_search` | セマンティック検索（要埋め込み生成） |

## コマンドリファレンス

### 基本コマンド

| コマンド | 説明 |
|---------|------|
| `jira-db init [--interactive]` | 設定ファイルを初期化 |
| `jira-db project init` | JIRAからプロジェクト一覧を取得 |
| `jira-db project list [--verbose]` | プロジェクト一覧を表示 |
| `jira-db project enable <KEY>` | プロジェクトの同期を有効化 |
| `jira-db project disable <KEY>` | プロジェクトの同期を無効化 |
| `jira-db sync [--project <KEY>]` | データを同期 |
| `jira-db config show` | 現在の設定を表示 |
| `jira-db config set <KEY> <VALUE>` | 設定値を変更 |

### 検索・分析コマンド

| コマンド | 説明 |
|---------|------|
| `jira-db search <QUERY> [OPTIONS]` | イシューを検索 |
| `jira-db metadata --project <KEY> [--type <TYPE>]` | メタデータを表示 |
| `jira-db history <ISSUE_KEY> [--field <FIELD>]` | 変更履歴を表示 |
| `jira-db embeddings [--provider <PROVIDER>]` | 埋め込みを生成 |
| `jira-db report [--interactive]` | HTMLレポートを生成 |

### 検索オプション

```bash
jira-db search <QUERY> [OPTIONS]

オプション:
  -p, --project <KEY>     プロジェクトで絞り込み
  -s, --status <STATUS>   ステータスで絞り込み
  -a, --assignee <NAME>   担当者で絞り込み
  -l, --limit <NUM>       表示件数（デフォルト: 20）
  -o, --offset <NUM>      オフセット（ページネーション用）
```

**例:**
```bash
# "bug"を含むイシューを検索
jira-db search bug

# 特定プロジェクトのオープンなイシューを検索
jira-db search "" --project PROJ --status "Open"

# 担当者で絞り込み
jira-db search "performance" --assignee "john"
```

## セマンティック検索

自然言語によるセマンティック検索を使用する場合は、埋め込みを生成します。

### プロバイダー一覧

| プロバイダー | 環境変数 | デフォルトモデル | 特徴 |
|-------------|----------|-----------------|------|
| `openai` | `OPENAI_API_KEY` | text-embedding-3-small | バランス良好 |
| `ollama` | 不要 | nomic-embed-text | 無料、ローカル実行 |
| `cohere` | `COHERE_API_KEY` | embed-multilingual-v3.0 | 多言語に強い |

### 使用例

```bash
# Ollama（無料、ローカル）を使用
ollama pull nomic-embed-text
jira-db embeddings --provider ollama

# OpenAIを使用
export OPENAI_API_KEY="sk-..."
jira-db embeddings --project PROJ

# Cohereを使用（日本語に強い）
export COHERE_API_KEY="..."
jira-db embeddings --provider cohere
```

詳細は [docs/EMBEDDINGS.md](./docs/EMBEDDINGS.md) を参照してください。

## 増分同期

jira-dbは効率的なデータ同期のため、増分同期（Incremental Sync）と中断再開（Resumable Sync）をサポートしています。

### 仕組み

- **増分同期**: 2回目以降の同期では、前回同期以降に更新されたイシューのみを取得します
- **中断再開**: 同期中に中断（ネットワークエラー、Ctrl+C等）しても、最後のチェックポイントから再開できます
- **安全マージン**: JQLの分単位精度による取りこぼしを防ぐため、設定可能なマージンを適用

### 設定

`settings.json`で増分同期の動作を設定できます：

```json
{
  "sync": {
    "incremental_sync_enabled": true,
    "incremental_sync_margin_minutes": 5
  }
}
```

| 設定 | デフォルト | 説明 |
|-----|----------|------|
| `incremental_sync_enabled` | `true` | 増分同期を有効にする（`false`で常にフルSync） |
| `incremental_sync_margin_minutes` | `5` | 安全マージン（分）。JQLは分単位精度のため、同じ分内の更新を確実に取得 |

### 動作例

```bash
# 初回同期（フルSync）
jira-db sync
# -> すべてのイシューを取得

# 2回目以降（増分Sync）
jira-db sync
# -> 前回同期以降に更新されたイシューのみ取得

# 同期が中断した場合
jira-db sync
# -> チェックポイントから自動的に再開
```

### 確認方法

同期の状態は`settings.json`で確認できます：

```json
{
  "projects": [
    {
      "id": "10001",
      "key": "PROJ",
      "name": "My Project",
      "sync_enabled": true,
      "last_synced": "2024-12-15T14:30:00Z",
      "sync_checkpoint": null
    }
  ]
}
```

| フィールド | 説明 |
|-----------|------|
| `last_synced` | 最後に**完了**した同期の日時。次回同期時はこの日時以降の更新のみ取得（増分同期の起点） |
| `sync_checkpoint` | 同期**中断時**のみ存在。次回実行時にここから再開 |

**正常な状態**: `last_synced` に日時があり、`sync_checkpoint` は `null`
**中断した状態**: `sync_checkpoint` に再開情報が存在

詳細な技術ドキュメントは [docs/SYNC_ARCHITECTURE.md](./docs/SYNC_ARCHITECTURE.md) を参照してください。

## データベーススキーマ

### 主要テーブル

| テーブル | 説明 |
|---------|------|
| `projects` | JIRAプロジェクトのメタデータ |
| `issues` | JIRAイシューデータ（raw_dataに全フィールド含む） |
| `issue_change_history` | 変更履歴（正規化済み） |
| `sync_history` | 同期履歴 |
| `statuses` | ステータス定義 |
| `priorities` | 優先度定義 |
| `issue_types` | イシュータイプ定義 |
| `labels` | ラベル |
| `components` | コンポーネント定義 |
| `fix_versions` | バージョン定義 |
| `issue_embeddings` | セマンティック検索用埋め込み |

### issuesテーブル

| カラム | 型 | 説明 |
|--------|-----|------|
| id | VARCHAR | イシューID（主キー） |
| project_id | VARCHAR | プロジェクトID |
| key | VARCHAR | イシューキー（例: PROJ-123） |
| summary | TEXT | イシューの概要 |
| description | TEXT | イシューの説明 |
| status | VARCHAR | ステータス |
| priority | VARCHAR | 優先度 |
| assignee | VARCHAR | 担当者 |
| reporter | VARCHAR | 報告者 |
| created_date | TIMESTAMP | イシュー作成日 |
| updated_date | TIMESTAMP | イシュー更新日 |
| raw_data | JSON | 完全なAPIレスポンス（全フィールド・変更履歴含む） |
| synced_at | TIMESTAMP | 同期日時 |

## トラブルシューティング

### ビルドエラー: `ld: library 'duckdb' not found`

DuckDBライブラリのパスを設定してください：

```bash
# .zshrcに追加（恒久的）
echo 'export LIBRARY_PATH="/opt/homebrew/lib:$LIBRARY_PATH"' >> ~/.zshrc
echo 'export LD_LIBRARY_PATH="/opt/homebrew/lib:$LD_LIBRARY_PATH"' >> ~/.zshrc
source ~/.zshrc

# または一時的に設定
export LIBRARY_PATH="/opt/homebrew/lib:$LIBRARY_PATH"
cargo build --release
```

### `No projects found` エラー

先にプロジェクトを初期化してください：
```bash
jira-db project init
```

### 認証エラー

1. [Atlassian APIトークン管理ページ](https://id.atlassian.com/manage-profile/security/api-tokens)で新しいトークンを作成
2. `jira-db config set jira.api_key <新しいトークン>` で更新

### 同期が遅い

- 必要なプロジェクトのみ有効化する
- ログレベルを下げる: `RUST_LOG=warn jira-db sync`
- 特定のプロジェクトのみ同期: `jira-db sync --project PROJ`

## 開発

開発者向けの詳細情報は [CLAUDE.md](./CLAUDE.md) を参照してください。

### プロジェクト構成

```
jira-db/
├── crates/
│   ├── jira-db-core/     # コアライブラリ（ドメイン、ユースケース）
│   ├── jira-db-cli/      # CLIバイナリ
│   ├── jira-db-mcp/      # MCPサーバー
│   └── jira-db-tauri/    # デスクトップアプリ（Tauri + Angular）
├── typespec/             # API定義（TypeSpec）
└── docs/                 # ドキュメント
```

### 開発コマンド

```bash
cargo test          # テスト実行
cargo clippy        # リンター
cargo fmt           # フォーマット
cargo check         # コンパイルチェック
```

## ライセンス

MIT License - 詳細は [LICENSE](./LICENSE) を参照してください。

## 実装済み機能

- ✅ CLI（検索、同期、メタデータ、レポート）
- ✅ MCPサーバー（stdio/HTTP、8種類のツール）
- ✅ セマンティック検索（OpenAI/Ollama/Cohere）
- ✅ デスクトップアプリ（Tauri + Angular）
- ✅ HTMLレポート（静的/インタラクティブ）
- ✅ 増分同期（最終同期日時以降の変更のみ取得）
- ✅ 中断再開可能な同期（チェックポイントから再開）

## 今後の実装予定

- [ ] エクスポート機能（CSV、Excel）
- [ ] Webhookによるリアルタイム同期
- [ ] 複数JIRA環境のサポート

## リンク

- [JIRAドキュメント](https://developer.atlassian.com/cloud/jira/platform/rest/v3/)
- [DuckDB](https://duckdb.org/)
- [Tauri](https://tauri.app/)
- [Angular](https://angular.dev/)
