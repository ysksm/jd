# JIRA Mock Server デプロイメント

JIRA Mock Serverのデプロイメントに必要なファイルです。

## ファイル構成

```
cicd/deploy/
├── README.md           # このファイル
├── Dockerfile          # Dockerイメージビルド用
├── docker-compose.yml  # ローカル開発用
└── deploy.sh           # 手動デプロイスクリプト
```

## ローカル開発

### Docker Composeで起動

```bash
cd cicd/deploy
docker-compose up -d
```

サーバーは http://localhost:8080 で起動します。

### ログ確認

```bash
docker-compose logs -f
```

### 停止

```bash
docker-compose down
```

## 手動デプロイ

### 基本的な使い方

```bash
# ステージング環境へデプロイ
./deploy.sh --host staging.example.com --user deploy --env staging

# プロダクション環境へデプロイ
./deploy.sh --host prod.example.com --user deploy --env production

# ドライラン（実行せずにコマンドを表示）
./deploy.sh --dry-run --host example.com --user deploy
```

### オプション

| オプション | 短縮形 | デフォルト | 説明 |
|-----------|--------|-----------|------|
| `--env` | `-e` | staging | 環境（staging/production） |
| `--host` | `-h` | - | デプロイ先ホスト（必須） |
| `--user` | `-u` | - | SSHユーザー（必須） |
| `--port` | `-p` | 8080 | アプリケーションポート |
| `--data-dir` | `-d` | /opt/jira-mock-server/data | データディレクトリ |
| `--image` | `-i` | jira-mock-server:latest | Dockerイメージ |
| `--build` | - | false | ローカルでイメージをビルド |
| `--dry-run` | - | false | コマンドを表示のみ |

### ローカルビルドしてデプロイ

```bash
./deploy.sh --build --host example.com --user deploy
```

## Docker イメージのビルド

### プロジェクトルートからビルド

```bash
docker build -t jira-mock-server -f cicd/deploy/Dockerfile .
```

### 実行

```bash
docker run -d \
  --name jira-mock \
  -p 8080:8080 \
  -v $(pwd)/mock-data:/app/data \
  jira-mock-server
```

## GitHub Actions CI/CD

`.github/workflows/mock-server.yml` でCI/CDパイプラインが定義されています：

### ジョブ

| ジョブ | 説明 | トリガー |
|-------|------|----------|
| build | ビルド、テスト、Clippy | 常に実行 |
| docker | Dockerイメージビルド＆プッシュ | buildの成功後 |
| deploy-staging | ステージング環境へデプロイ | mainブランチへのプッシュ |
| deploy-production | プロダクション環境へデプロイ | 手動トリガーまたはタグ |

### 必要なシークレット

| シークレット | 説明 |
|-------------|------|
| `STAGING_HOST` | ステージングサーバーのホスト |
| `STAGING_USER` | ステージングSSHユーザー |
| `STAGING_SSH_KEY` | ステージングSSH秘密鍵 |
| `PRODUCTION_HOST` | プロダクションサーバーのホスト |
| `PRODUCTION_USER` | プロダクションSSHユーザー |
| `PRODUCTION_SSH_KEY` | プロダクションSSH秘密鍵 |

### 必要な変数

| 変数 | 説明 |
|------|------|
| `STAGING_URL` | ステージング環境のURL |
| `PRODUCTION_URL` | プロダクション環境のURL |

## 環境変数

| 変数 | 説明 | デフォルト |
|------|------|-----------|
| `RUST_LOG` | ログレベル | info（staging）/ warn（production） |
| `DATA_DIR` | データディレクトリ | /app/data |

## ヘルスチェック

デプロイ後、以下のエンドポイントでヘルスチェックが実行されます：

```bash
curl http://localhost:8080/rest/api/3/project
```

## トラブルシューティング

### コンテナログの確認

```bash
docker logs jira-mock-server --tail 100
```

### コンテナの状態確認

```bash
docker ps -a | grep jira-mock
```

### コンテナの再起動

```bash
docker restart jira-mock-server
```
