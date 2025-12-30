# jira-db

JIRAのプロジェクトとイシューをローカルのDuckDBデータベースに同期し、オフラインで高速に検索・分析できるコマンドラインツールです。

## 特徴

- 🚀 JIRAデータをローカルに同期して高速アクセス
- 💾 DuckDBによる効率的なデータ保存
- 🔄 プロジェクト単位での同期制御
- 📊 RAWデータとしてJSON形式で完全なAPIレスポンス（全フィールド・変更履歴含む）を保存
- 🏷️ プロジェクトのメタデータ（ステータス、優先度、イシュータイプ、ラベル等）を自動同期
- 🛠️ 使いやすいCLIインターフェース
- 🔍 高速なフルテキスト検索とフィルタリング

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

ビルドに成功すると、`target/release/jira-db` に実行可能ファイルが生成されます。

## 使い方

### 1. 設定ファイルの初期化

初回起動時に設定ファイルを生成します。

```bash
jira-db init
```

これにより現在のディレクトリに `./settings.json` が作成されます。

**出力例：**
```
[INFO] Initializing jira-db configuration...
[INFO] Created configuration file at: ./settings.json
[INFO]
[INFO] Next steps:
[INFO]   1. Edit the configuration file and set your JIRA credentials:
[INFO]      - endpoint: Your JIRA instance URL
[INFO]      - username: Your JIRA username/email
[INFO]      - api_key: Your JIRA API key
[INFO]   2. Run: jira-db project init
```

### 2. 認証情報の設定

生成された設定ファイルを編集して、JIRA接続情報を入力します。

```bash
# エディタで設定ファイルを開く
vim ./settings.json
# または
code ./settings.json
```

以下の項目を実際の値に変更してください：

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

### 3. プロジェクト一覧の初期化

設定が完了したら、JIRAからプロジェクト一覧を取得します。

```bash
jira-db project init
```

**出力例：**
```
[INFO] Initializing project list from JIRA...
[INFO] Connecting to JIRA...
[INFO] Connected successfully!
[INFO] Found 5 projects
[INFO]
[INFO] Project list initialized successfully!
[INFO] Run 'jira-db project list' to see all projects
[INFO] Use 'jira-db project enable <PROJECT_KEY>' to enable sync for specific projects
```

### 4. プロジェクト一覧の確認

取得したプロジェクトを確認します。

```bash
# シンプルな一覧表示
jira-db project list

# 詳細情報を表示
jira-db project list --verbose
```

**出力例：**
```
Projects:

  [ ] PROJ - My Project
  [ ] TEST - Test Project
  [ ] DEMO - Demo Project

Use --verbose for detailed information
Use 'jira-db project enable <PROJECT_KEY>' to enable syncing for a project
```

### 5. 同期するプロジェクトを有効化

同期したいプロジェクトを選択します。

```bash
# プロジェクトの同期を有効化
jira-db project enable PROJ

# 複数のプロジェクトを有効化する場合
jira-db project enable TEST
jira-db project enable DEMO
```

**出力例：**
```
[INFO] Enabled sync for project: PROJ
```

### 6. データの同期

有効化したプロジェクトのイシューを同期します。

```bash
# すべての有効なプロジェクトを同期
jira-db sync

# 特定のプロジェクトのみ同期
jira-db sync --project PROJ
```

**出力例：**
```
[INFO] Connecting to JIRA...
[INFO] Connected successfully!
[INFO] Syncing 2 projects
[INFO] Syncing project: PROJ
[INFO] Fetching issues for project: PROJ
[INFO] Fetched 150 issues, saving to database...
[INFO] Successfully synced 150 issues for project PROJ
```

### 7. 設定の確認・変更

#### 現在の設定を表示

```bash
jira-db config show
```

#### 設定値の変更

```bash
# エンドポイントの変更
jira-db config set jira.endpoint https://new-domain.atlassian.net

# ユーザー名の変更
jira-db config set jira.username new-email@example.com

# APIキーの変更
jira-db config set jira.api_key new-api-token
```

#### プロジェクトの同期を無効化

```bash
jira-db project disable PROJ
```

## コマンドリファレンス

### `jira-db init [OPTIONS]`
設定ファイルを初期化します。`./settings.json` を作成します。

**オプション：**
- `-i, --interactive` - 対話的に設定を入力（エンドポイント、ユーザー名、APIキーなど）

### `jira-db project <SUBCOMMAND>`
プロジェクト管理コマンド。

**サブコマンド：**
- `init` - JIRAからプロジェクト一覧を取得して初期化
- `list [--verbose]` - プロジェクト一覧を表示
  - `-v, --verbose` - 詳細情報を表示（ID、同期ステータス、最終同期日時）
- `enable <PROJECT_KEY>` - プロジェクトの同期を有効化
- `disable <PROJECT_KEY>` - プロジェクトの同期を無効化

### `jira-db sync [OPTIONS]`
有効化されたプロジェクトのJIRAデータを同期します。

**オプション：**
- `-p, --project <PROJECT_KEY>` - 特定のプロジェクトのみ同期
- `-f, --force` - フル同期を強制（将来的に増分同期実装時に使用）

**注意：** プロジェクトが初期化されていない場合はエラーになります。先に `jira-db project init` を実行してください。

### `jira-db config <SUBCOMMAND>`
設定を管理します。

**サブコマンド：**
- `show` - 現在の設定を表示
- `set <KEY> <VALUE>` - 設定値を変更
  - 有効なキー: `jira.endpoint`, `jira.username`, `jira.api_key`

### `jira-db search <QUERY> [OPTIONS]`
同期済みのイシューを検索します。

**引数：**
- `QUERY` - 検索キーワード（summaryとdescriptionから検索）

**オプション：**
- `-p, --project <PROJECT_KEY>` - プロジェクトで絞り込み
- `-s, --status <STATUS>` - ステータスで絞り込み
- `-a, --assignee <NAME>` - 担当者で絞り込み
- `-l, --limit <NUM>` - 表示件数（デフォルト: 20）
- `-o, --offset <NUM>` - オフセット（ページネーション用、デフォルト: 0）

**例：**
```bash
# "bug"を含むイシューを検索
jira-db search bug

# 特定プロジェクトのオープンなイシューを検索
jira-db search "" --project PROJ --status "Open"

# 担当者で絞り込み
jira-db search "performance" --assignee "john"

# 2ページ目を表示（21〜40件目）
jira-db search "api" --limit 20 --offset 20
```

### `jira-db metadata [OPTIONS]`
プロジェクトのメタデータ（ステータス、優先度、イシュータイプなど）を表示します。

**オプション：**
- `-p, --project <PROJECT_KEY>` - プロジェクトキーを指定（必須）
- `-t, --type <TYPE>` - メタデータタイプを指定（オプション）
  - `status` - ステータス一覧
  - `priority` - 優先度一覧
  - `issue-type` - イシュータイプ一覧
  - `label` - ラベル一覧
  - `component` - コンポーネント一覧
  - `version` - バージョン（フィックスバージョン）一覧

**例：**
```bash
# プロジェクト全体のメタデータサマリーを表示
jira-db metadata --project PROJ

# ステータス一覧を表示
jira-db metadata --project PROJ --type status

# 優先度一覧を表示
jira-db metadata --project PROJ --type priority

# イシュータイプ一覧を表示
jira-db metadata --project PROJ --type issue-type

# ラベル一覧を表示
jira-db metadata --project PROJ --type label
```

**注意：** メタデータは `jira-db sync` の実行時にJIRA APIから自動的に取得・保存されます。

### `jira-db history <ISSUE_KEY> [OPTIONS]`
イシューの変更履歴を表示します。

**引数：**
- `ISSUE_KEY` - イシューキー（例: PROJ-123）

**オプション：**
- `-f, --field <FIELD>` - 特定のフィールドで絞り込み（例: status, assignee, priority）
- `-l, --limit <NUM>` - 表示件数（デフォルト: 50）

**例：**
```bash
# イシューの全変更履歴を表示
jira-db history PROJ-123

# ステータス変更のみを表示
jira-db history PROJ-123 --field status

# 担当者変更のみを表示
jira-db history PROJ-123 --field assignee

# 最新10件のみ表示
jira-db history PROJ-123 --limit 10
```

### `jira-db test-ticket [OPTIONS]`
JIRAにテスト用チケットを作成します。動作確認やテスト目的で使用します。

**オプション：**
- `-p, --project <PROJECT_KEY>` - プロジェクトキー（必須）
- `-s, --summary <SUMMARY>` - チケットのタイトル（デフォルト: "[jira-db] 動作確認用テストチケット"）
- `-d, --description <DESCRIPTION>` - チケットの説明
- `-t, --issue-type <TYPE>` - イシュータイプ（デフォルト: "Task"）
- `-n, --count <NUM>` - 作成するチケット数（1-10、デフォルト: 1）

**例：**
```bash
# デフォルト設定でテストチケットを作成
jira-db test-ticket --project PROJ

# カスタムタイトルでテストチケットを作成
jira-db test-ticket --project PROJ --summary "APIテスト用チケット"

# 複数のテストチケットを作成
jira-db test-ticket --project PROJ --count 3

# Bugタイプのテストチケットを作成
jira-db test-ticket --project PROJ --issue-type Bug
```

### `jira-db report [OPTIONS]`
同期済みデータからHTMLレポートを生成します。

**オプション：**
- `-p, --project <PROJECT_KEY>` - プロジェクトキー（"all"で全プロジェクト）
- `-i, --interactive` - インタラクティブレポートを生成（JavaScript付き）
- `-o, --output <PATH>` - 出力ファイルパス（デフォルト: reports/report_YYYYMMDD_HHMMSS.html）

**例：**
```bash
# 全プロジェクトの静的レポートを生成
jira-db report

# 特定プロジェクトのレポートを生成
jira-db report --project PROJ

# インタラクティブレポートを生成
jira-db report --interactive

# 出力先を指定
jira-db report --project PROJ --output ./my-report.html
```

## データの保存場所

- **設定ファイル**: `./settings.json`（カレントディレクトリ）
- **データベース**: デフォルトは `./data/jira.duckdb`（設定ファイルで変更可能）

## ワークフロー図

```
┌─────────────────────┐
│  jira-db init       │  設定ファイル作成
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│  settings.json編集  │  JIRA認証情報入力
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│ jira-db project init│  プロジェクト一覧取得
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│jira-db project list │  プロジェクト確認
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│jira-db project      │  同期対象を選択
│  enable <KEY>       │
└──────────┬──────────┘
           │
           ▼
┌─────────────────────┐
│  jira-db sync       │  データ同期
└─────────────────────┘
```

## データベーススキーマ

### projectsテーブル
JIRAプロジェクトのメタデータを保存。

| カラム | 型 | 説明 |
|--------|-----|------|
| id | VARCHAR | プロジェクトID（主キー） |
| key | VARCHAR | プロジェクトキー（例: PROJ） |
| name | VARCHAR | プロジェクト名 |
| description | TEXT | プロジェクトの説明 |
| raw_data | JSON | 完全なAPIレスポンス |
| created_at | TIMESTAMP | レコード作成日時 |
| updated_at | TIMESTAMP | レコード更新日時 |

### issuesテーブル
JIRAイシューデータを保存。

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
| issue_type | VARCHAR | イシュータイプ（Task, Bug, Story等） |
| resolution | VARCHAR | 解決状況 |
| labels | VARCHAR | ラベル（カンマ区切り） |
| components | VARCHAR | コンポーネント（カンマ区切り） |
| fix_versions | VARCHAR | フィックスバージョン（カンマ区切り） |
| sprint | VARCHAR | スプリント名 |
| parent_key | VARCHAR | 親イシューキー（サブタスクの場合） |
| created_date | TIMESTAMP | イシュー作成日 |
| updated_date | TIMESTAMP | イシュー更新日 |
| raw_data | JSON | 完全なAPIレスポンス（全フィールド・変更履歴含む） |
| synced_at | TIMESTAMP | 同期日時 |

**注意：** raw_dataカラムには、JIRA REST API v3から取得した全フィールド（`*navigable`）と変更履歴（`changelog`）を含む完全なJSONデータが保存されます。

### sync_historyテーブル
同期履歴を記録。

| カラム | 型 | 説明 |
|--------|-----|------|
| id | INTEGER | 履歴ID（主キー） |
| project_id | VARCHAR | プロジェクトID |
| sync_type | VARCHAR | 同期タイプ（full/incremental） |
| started_at | TIMESTAMP | 開始日時 |
| completed_at | TIMESTAMP | 完了日時 |
| status | VARCHAR | ステータス（running/completed/failed） |
| items_synced | INTEGER | 同期したアイテム数 |
| error_message | TEXT | エラーメッセージ |

### issue_change_historyテーブル
イシューの変更履歴を保存。同期時にchangelogから正規化されたデータ。

| カラム | 型 | 説明 |
|--------|-----|------|
| id | INTEGER | 変更履歴ID（主キー） |
| issue_id | VARCHAR | イシューID |
| issue_key | VARCHAR | イシューキー |
| history_id | VARCHAR | JIRAの履歴エントリID |
| author_account_id | VARCHAR | 変更者のアカウントID |
| author_display_name | VARCHAR | 変更者の表示名 |
| field | VARCHAR | 変更されたフィールド（status, assignee等） |
| field_type | VARCHAR | フィールドタイプ |
| from_value | VARCHAR | 変更前の値（ID） |
| from_string | VARCHAR | 変更前の値（表示名） |
| to_value | VARCHAR | 変更後の値（ID） |
| to_string | VARCHAR | 変更後の値（表示名） |
| changed_at | TIMESTAMP | 変更日時 |
| created_at | TIMESTAMP | レコード作成日時 |

### メタデータテーブル

#### statusesテーブル
プロジェクトのステータス定義を保存。

| カラム | 型 | 説明 |
|--------|-----|------|
| project_id | VARCHAR | プロジェクトID（複合主キー） |
| name | VARCHAR | ステータス名（複合主キー） |
| description | VARCHAR | 説明 |
| category | VARCHAR | ステータスカテゴリ |
| created_at | TIMESTAMP | レコード作成日時 |
| updated_at | TIMESTAMP | レコード更新日時 |

#### prioritiesテーブル
プロジェクトの優先度定義を保存。

| カラム | 型 | 説明 |
|--------|-----|------|
| project_id | VARCHAR | プロジェクトID（複合主キー） |
| name | VARCHAR | 優先度名（複合主キー） |
| description | VARCHAR | 説明 |
| icon_url | VARCHAR | アイコンURL |
| created_at | TIMESTAMP | レコード作成日時 |
| updated_at | TIMESTAMP | レコード更新日時 |

#### issue_typesテーブル
プロジェクトのイシュータイプ定義を保存。

| カラム | 型 | 説明 |
|--------|-----|------|
| project_id | VARCHAR | プロジェクトID（複合主キー） |
| name | VARCHAR | イシュータイプ名（複合主キー） |
| description | VARCHAR | 説明 |
| icon_url | VARCHAR | アイコンURL |
| subtask | BOOLEAN | サブタスクかどうか |
| created_at | TIMESTAMP | レコード作成日時 |
| updated_at | TIMESTAMP | レコード更新日時 |

#### labelsテーブル
プロジェクトのラベルを保存。

| カラム | 型 | 説明 |
|--------|-----|------|
| project_id | VARCHAR | プロジェクトID（複合主キー） |
| name | VARCHAR | ラベル名（複合主キー） |
| created_at | TIMESTAMP | レコード作成日時 |
| updated_at | TIMESTAMP | レコード更新日時 |

#### componentsテーブル
プロジェクトのコンポーネント定義を保存。

| カラム | 型 | 説明 |
|--------|-----|------|
| project_id | VARCHAR | プロジェクトID（複合主キー） |
| name | VARCHAR | コンポーネント名（複合主キー） |
| description | VARCHAR | 説明 |
| lead | VARCHAR | リード担当者 |
| created_at | TIMESTAMP | レコード作成日時 |
| updated_at | TIMESTAMP | レコード更新日時 |

#### fix_versionsテーブル
プロジェクトのバージョン（フィックスバージョン）定義を保存。

| カラム | 型 | 説明 |
|--------|-----|------|
| project_id | VARCHAR | プロジェクトID（複合主キー） |
| name | VARCHAR | バージョン名（複合主キー） |
| description | VARCHAR | 説明 |
| released | BOOLEAN | リリース済みかどうか |
| release_date | TIMESTAMP | リリース日 |
| created_at | TIMESTAMP | レコード作成日時 |
| updated_at | TIMESTAMP | レコード更新日時 |

**注意：** メタデータテーブルは同期時にJIRA REST API v3から直接取得されます。issuesテーブルのデータから抽出されるのではなく、プロジェクトに定義されているすべてのメタデータが保存されます。

## トラブルシューティング

### ビルドエラー: `ld: library 'duckdb' not found`

DuckDBライブラリのパスが設定されていない可能性があります。

**恒久的な解決方法（推奨）:**
```bash
# .zshrcに環境変数を追加
cat >> ~/.zshrc << 'EOF'

# DuckDB library path for jira-db
export LIBRARY_PATH="/opt/homebrew/lib:$LIBRARY_PATH"
export LD_LIBRARY_PATH="/opt/homebrew/lib:$LD_LIBRARY_PATH"
EOF

# ターミナルを再起動するか、設定を読み込む
source ~/.zshrc

# ビルド実行
cargo build --release
```

**一時的な解決方法（現在のセッションのみ）:**
```bash
export LIBRARY_PATH="/opt/homebrew/lib:$LIBRARY_PATH"
export LD_LIBRARY_PATH="/opt/homebrew/lib:$LD_LIBRARY_PATH"
cargo build --release
```

**注意**: bashを使用している場合は、`~/.zshrc`の代わりに`~/.bash_profile`を使用してください。

### `No projects found` エラー

`jira-db sync` を実行する前にプロジェクトを初期化する必要があります。

**解決方法:**
```bash
jira-db project init
```

### 認証エラー

APIキーが正しくない、または期限切れの可能性があります。

**解決方法:**
1. [Atlassian APIトークン管理ページ](https://id.atlassian.com/manage-profile/security/api-tokens)で新しいトークンを作成
2. `jira-db config set jira.api_key <新しいトークン>` で更新

### 同期が遅い

大量のイシューがある場合、初回同期には時間がかかります。

**Tips:**
- 必要なプロジェクトのみ有効化する
- ログレベルを下げる: `RUST_LOG=warn jira-db sync`
- 特定のプロジェクトのみ同期: `jira-db sync --project PROJ`

## 環境変数

### ログレベルの設定

```bash
# デバッグログを表示
RUST_LOG=debug jira-db sync

# エラーのみ表示
RUST_LOG=error jira-db sync

# デフォルトはinfo
RUST_LOG=info jira-db sync
```

## 開発

開発者向けの情報は [CLAUDE.md](./CLAUDE.md) を参照してください。

### テストの実行

```bash
cargo test
```

### コードフォーマット

```bash
cargo fmt
```

### リンター

```bash
cargo clippy
```

## ライセンス

MIT License - 詳細は [LICENSE](./LICENSE) を参照してください。

## 貢献

Issue報告やPull Requestを歓迎します！

## リンク

- [JIRAドキュメント](https://developer.atlassian.com/cloud/jira/platform/rest/v3/)
- [DuckDB](https://duckdb.org/)
- [Rust](https://www.rust-lang.org/)

## 実装済み機能

- ✅ **検索機能**（フルテキスト検索、フィルタ、ページネーション）
- ✅ **対話的な初期設定**（--interactiveフラグ）
- ✅ **進捗バー表示**（同期中の視覚的フィードバック）
- ✅ **エラーハンドリング**（自動リトライ、タイムアウト処理）
- ✅ **メタデータ管理**（ステータス、優先度、イシュータイプ、ラベル、コンポーネント、バージョンの同期・表示）
- ✅ **完全なイシューデータ取得**（全フィールド、変更履歴を含む完全なJSON保存）
- ✅ **変更履歴管理**（`history`コマンドでイシューの変更履歴を表示）
- ✅ **テストチケット作成**（`test-ticket`コマンドでJIRAにテストチケットを作成）
- ✅ **HTMLレポート生成**（`report`コマンドで静的・インタラクティブレポートを生成）

## 今後の実装予定

- [ ] 増分同期（最終同期日時以降の変更のみ取得）
- [ ] エクスポート機能（CSV、Excel）
- [ ] Webhookによるリアルタイム同期
- [ ] 複数JIRA環境のサポート
- [ ] ユニットテスト・統合テスト

## サポート

問題が発生した場合は、[GitHubのIssue](https://github.com/ysksm/jira-db/issues)で報告してください。
