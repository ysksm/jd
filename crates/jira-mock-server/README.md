# JIRA Mock Server

テストと開発用のモックJIRA APIサーバーです。JSONファイルによる永続化機能を持つ、軽量なインメモリJIRA REST API v3互換サーバーを提供します。

## 機能

- **JIRA REST API v3互換**: プロジェクト、課題、メタデータの主要エンドポイントを実装
- **インメモリストレージ**: JSONファイル永続化機能付きの高速スレッドセーフデータストア
- **サンプルデータ**: ステータス、課題タイプ、優先度を含むサンプルプロジェクトを自動生成
- **変更履歴サポート**: タイムスタンプ付きのステータス遷移を追跡
- **JQLサポート**: プロジェクトとテキスト検索フィルタの基本的なJQL解析
- **CORS有効**: Webクライアントテストに対応

## クイックスタート

### ビルドと実行

```bash
# ビルド
cargo build -p jira-mock-server --release

# デフォルト設定で実行
cargo run -p jira-mock-server

# カスタムオプションで実行
cargo run -p jira-mock-server -- --port 3000 --data-dir ./my-data
```

### コマンドラインオプション

| オプション | 短縮形 | デフォルト | 説明 |
|-----------|--------|-----------|------|
| `--port` | `-p` | 8080 | リッスンポート |
| `--data-dir` | `-d` | ./mock-data | JSONデータファイルのディレクトリ |

### 環境変数

| 変数 | 説明 |
|------|------|
| `RUST_LOG` | ログレベル（例: `debug`, `info`, `warn`） |

## APIエンドポイント

### プロジェクト

| メソッド | エンドポイント | 説明 |
|---------|---------------|------|
| GET | `/rest/api/3/project` | 全プロジェクト一覧 |

### 課題

| メソッド | エンドポイント | 説明 |
|---------|---------------|------|
| GET | `/rest/api/3/search/jql` | 課題検索（クエリパラメータ） |
| POST | `/rest/api/3/search/jql` | 課題検索（JSONボディ） |
| GET | `/rest/api/3/search` | レガシー検索エンドポイント |
| POST | `/rest/api/3/issue` | 課題作成 |
| PUT | `/rest/api/3/issue/{key}` | 課題更新 |

### ワークフロー

| メソッド | エンドポイント | 説明 |
|---------|---------------|------|
| GET | `/rest/api/3/issue/{key}/transitions` | 利用可能なトランジション取得 |
| POST | `/rest/api/3/issue/{key}/transitions` | ステータス遷移実行 |

### メタデータ

| メソッド | エンドポイント | 説明 |
|---------|---------------|------|
| GET | `/rest/api/3/project/{key}/statuses` | プロジェクトステータス取得 |
| GET | `/rest/api/3/priority` | 全優先度取得 |
| GET | `/rest/api/3/issuetype/project` | プロジェクトIDで課題タイプ取得 |
| GET | `/rest/api/3/issue/createmeta/{key}/issuetypes` | プロジェクトキーで課題タイプ取得 |
| GET | `/rest/api/3/project/{key}/components` | プロジェクトコンポーネント取得 |
| GET | `/rest/api/3/project/{key}/versions` | プロジェクトバージョン取得 |
| GET | `/rest/api/3/field` | 全フィールド定義取得 |

### 課題リンク

| メソッド | エンドポイント | 説明 |
|---------|---------------|------|
| POST | `/rest/api/3/issueLink` | 課題リンク作成 |

## jira-dbとの設定

jira-dbをモックサーバーで使用するための設定:

```json
{
  "jira_endpoints": [
    {
      "name": "mock",
      "display_name": "モックサーバー",
      "endpoint": "http://localhost:8080",
      "username": "mock@example.com",
      "api_key": "mock-api-key"
    }
  ],
  "active_endpoint": "mock"
}
```

またはCLIを使用:

```bash
jira-db endpoint add mock \
  --url http://localhost:8080 \
  --username mock@example.com \
  --api-key mock-api-key \
  --display-name "モックサーバー"

jira-db endpoint set-active mock
```

## デフォルトサンプルデータ

初回起動時にサンプルデータが作成されます:

### プロジェクト
- **キー**: `TEST`
- **名前**: Test Project
- **ID**: 10000

### ステータス
| 名前 | カテゴリ |
|------|----------|
| To Do | new |
| In Progress | indeterminate |
| Done | done |

### 課題タイプ
- Epic（エピック）
- Story（ストーリー）
- Task（タスク）
- Bug（バグ）

### 優先度
- Highest（最高）
- High（高）
- Medium（中）
- Low（低）
- Lowest（最低）

### ワークフロートランジション
```
To Do ──► In Progress ──► Done
  ▲            │           │
  │            ▼           │
  └────────────────────────┘
```

## JQLサポート

モックサーバーは基本的なJQLクエリをサポートしています:

```
# プロジェクトでフィルタ
project = TEST

# テキスト検索
text ~ "検索語"

# 組み合わせ
project = TEST AND text ~ "バグ"
```

### ページネーション

```bash
# クエリパラメータを使用
GET /rest/api/3/search/jql?jql=project=TEST&maxResults=50&startAt=0

# JSONボディを使用
POST /rest/api/3/search/jql
{
  "jql": "project = TEST",
  "maxResults": 50,
  "startAt": 0
}
```

## データ永続化

データはデータディレクトリ内のJSONファイルに保存されます:

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

ファイルはデータ変更時に自動的に作成・更新されます。

## 使用例

### 課題の作成

```bash
curl -X POST http://localhost:8080/rest/api/3/issue \
  -H "Content-Type: application/json" \
  -d '{
    "fields": {
      "project": {"key": "TEST"},
      "summary": "テスト課題",
      "description": "説明文",
      "issuetype": {"name": "Task"},
      "priority": {"name": "Medium"}
    }
  }'
```

レスポンス:
```json
{
  "id": "10001",
  "key": "TEST-1",
  "self": "http://localhost:8080/rest/api/3/issue/10001"
}
```

### 課題のトランジション

```bash
# 利用可能なトランジションを取得
curl http://localhost:8080/rest/api/3/issue/TEST-1/transitions

# トランジションを実行
curl -X POST http://localhost:8080/rest/api/3/issue/TEST-1/transitions \
  -H "Content-Type: application/json" \
  -d '{"transition": {"id": "21"}}'
```

### 課題の検索

```bash
curl "http://localhost:8080/rest/api/3/search/jql?jql=project=TEST&maxResults=10"
```

## Docker

### イメージのビルド

```bash
docker build -t jira-mock-server -f crates/jira-mock-server/Dockerfile .
```

### コンテナの実行

```bash
docker run -d \
  --name jira-mock \
  -p 8080:8080 \
  -v $(pwd)/mock-data:/app/data \
  jira-mock-server
```

### Docker Compose

```bash
cd crates/jira-mock-server
docker-compose up -d
```

## CI/CDデプロイ

### GitHub Actions

`.github/workflows/mock-server.yml`にCI/CDワークフローが定義されています:

- **build**: ビルド、テスト、Clippy、フォーマットチェック
- **docker**: DockerイメージのビルドとGHCRへのプッシュ
- **deploy-staging**: mainブランチへのプッシュでステージングに自動デプロイ
- **deploy-production**: 手動トリガーまたはタグでプロダクションにデプロイ

### 手動デプロイ

```bash
# ステージング環境へデプロイ
./scripts/deploy.sh --host staging.example.com --user deploy --env staging

# プロダクション環境へデプロイ
./scripts/deploy.sh --host prod.example.com --user deploy --env production

# ドライラン（実行せずにコマンドを表示）
./scripts/deploy.sh --dry-run --host example.com --user deploy
```

## 制限事項

- **認証なし**: 全てのリクエストは認証検証なしで受け入れられます
- **基本的なJQL**: `project = X` と `text ~ "term"` パターンのみサポート
- **限定的な更新**: 課題更新はsummary、description、due dateのみ対応
- **単一ワークフロー**: 全プロジェクトで同じ To Do → In Progress → Done ワークフローを共有

## 開発

### プロジェクト構造

```
crates/jira-mock-server/
├── Cargo.toml
├── README.md
├── Dockerfile
├── docker-compose.yml
├── scripts/
│   └── deploy.sh
└── src/
    ├── main.rs      # エントリーポイント、CLI、ルーティング
    ├── handlers.rs  # APIエンドポイントハンドラー
    └── data.rs      # データモデルとストレージ
```

### 新しいエンドポイントの追加

1. `main.rs`にルートを追加
2. `handlers.rs`にハンドラーを実装
3. 必要に応じて`data.rs`にデータモデルを追加

### テストの実行

```bash
cargo test -p jira-mock-server
```

## ライセンス

jira-dbプロジェクトの一部です。
