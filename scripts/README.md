# Scripts

このディレクトリには、JiraDbアプリケーションのビルドとデプロイ用のスクリプトが含まれています。

## build.sh

JiraDbアプリケーションをビルドするスクリプトです。

### 使い方

```bash
# すべてのアプリケーションをビルド
./scripts/build.sh

# 個別にビルド
./scripts/build.sh cli       # CLI のみ
./scripts/build.sh web       # Web サーバー + Angular フロントエンド
./scripts/build.sh mcp       # MCP サーバーのみ
./scripts/build.sh tauri     # Tauri デスクトップアプリ
./scripts/build.sh frontend  # Angular フロントエンドのみ
```

### ビルドオプション

| オプション | 説明 |
|-----------|------|
| `all` | すべてのアプリケーションをビルド（デフォルト） |
| `cli` | jira-db-cli のみをビルド |
| `web` | jira-db-web + Angular フロントエンドをビルド |
| `mcp` | jira-db-mcp のみをビルド |
| `tauri` | Tauri デスクトップアプリをビルド |
| `frontend` | Angular フロントエンドのみをビルド（設定名を指定可能） |

### 出力先

- Rust バイナリ: `target/release/`
  - `jira-db` (CLI)
  - `jira-db-mcp` (MCP Server)
  - `jira-db-web` (Web Server)
- 静的ファイル: `crates/jira-db-web/static/browser/`

### 前提条件

- Rust（1.85以上）
- Node.js（frontendビルド時）
- macOS の場合、DuckDB がインストールされていること
  ```bash
  brew install duckdb
  ```

---

## deploy.sh

JiraDb Webアプリケーションをデプロイするスクリプトです。

### 使い方

```bash
# デフォルトの ./deploy ディレクトリにデプロイ
./scripts/deploy.sh

# 指定したディレクトリにデプロイ
./scripts/deploy.sh /path/to/deploy
```

### 実行内容

1. Webアプリケーションをビルド（`build.sh web` を内部で呼び出し）
2. 必要なファイルをデプロイディレクトリにコピー
3. デフォルトの `config.toml` を作成（存在しない場合）
4. systemd サービスファイルを生成

### デプロイ後のディレクトリ構造

```
deploy/
├── bin/
│   └── jira-db-web          # 実行バイナリ
├── static/
│   └── browser/             # Angular ビルド成果物
├── data/                    # データディレクトリ（settings.json 用）
├── logs/                    # ログディレクトリ
├── config.toml              # サーバー設定ファイル
├── run.sh                   # 起動スクリプト
├── stop.sh                  # 停止スクリプト
└── jira-db-web.service      # systemd サービスファイル
```

### サーバーの起動

```bash
cd /path/to/deploy
./run.sh
```

### systemd サービスとしてインストール

```bash
# 1. サービスファイルを編集してユーザー/グループを設定
vi /path/to/deploy/jira-db-web.service

# 2. systemd にコピー
sudo cp jira-db-web.service /etc/systemd/system/

# 3. デーモンをリロード
sudo systemctl daemon-reload

# 4. サービスを有効化して起動
sudo systemctl enable --now jira-db-web
```

### 設定ファイル (config.toml)

```toml
[server]
host = "0.0.0.0"    # バインドするアドレス
port = 8080         # ポート番号

[app]
settings_path = "./data/settings.json"  # JiraDb 設定ファイルのパス
static_dir = "./static/browser"         # 静的ファイルのディレクトリ
```
