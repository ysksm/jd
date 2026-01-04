# Embedding プロバイダー技術ドキュメント

## 目次

1. [概要](#概要)
2. [Embeddingとは](#embeddingとは)
3. [プロバイダー比較](#プロバイダー比較)
4. [プロバイダー別設定ガイド](#プロバイダー別設定ガイド)
   - [OpenAI](#openai-デフォルト)
   - [Ollama](#ollama-ローカル実行---推奨)
   - [LM Studio](#lm-studio-ローカル実行)
   - [Cohere](#cohere)
5. [ベクトル検索の仕組み](#ベクトル検索の仕組み)
6. [パフォーマンス最適化](#パフォーマンス最適化)
7. [トラブルシューティング](#トラブルシューティング)
8. [カスタムプロバイダーの実装](#カスタムプロバイダーの実装)
9. [参考リンク](#参考リンク)

---

## 概要

このドキュメントでは、jira-dbで使用可能なEmbeddingプロバイダーと、各プロバイダーの設定方法について説明します。

## Embeddingとは

Embeddingは、テキストを高次元のベクトル（数値の配列）に変換する技術です。類似したテキストは類似したベクトルを持つため、セマンティック（意味的）検索が可能になります。

```
テキスト: "ログインエラーの修正"
         ↓
Embedding: [0.023, -0.145, 0.089, ..., 0.034]  (1536次元)
```

## プロバイダー比較

| プロバイダー | 次元数 | コスト | 日本語 | ローカル | 特徴 | 状態 |
|------------|--------|--------|--------|----------|------|------|
| OpenAI | 1536/3072 | $$ | ◎ | × | バランス良好 | ✅ 対応済 |
| Ollama | 768-1024 | 無料 | ○ | ◎ | プライバシー | ✅ 対応済 |
| LM Studio | 384-768 | 無料 | ○ | ◎ | GUI管理 | ✅ 対応済 |
| Cohere | 1024 | $$ | ◎ | × | 多言語 | ✅ 対応済 |
| Azure OpenAI | 1536/3072 | $$ | ◎ | × | エンタープライズ | 🔜 予定 |
| Voyage AI | 1536 | $$ | ○ | × | 検索特化 | 🔜 予定 |

---

## プロバイダー別設定ガイド

### OpenAI (デフォルト)

OpenAIのtext-embedding APIを使用します。

#### セットアップ

1. [OpenAI](https://platform.openai.com/)でAPIキーを取得
2. 環境変数または設定ファイルでAPIキーを設定

#### 設定ファイル (settings.json)

```json
{
  "embeddings": {
    "provider": "openai",
    "api_key": "sk-...",
    "model": "text-embedding-3-small"
  }
}
```

#### 環境変数での設定

```bash
export OPENAI_API_KEY="sk-..."
```

#### CLI使用例

```bash
# embeddings生成（デフォルトプロバイダー）
jira-db embeddings --project PROJ

# 強制再生成
jira-db embeddings --project PROJ --force

# バッチサイズ指定
jira-db embeddings --project PROJ --batch-size 100

# 高品質モデルを使用
jira-db embeddings --model text-embedding-3-large
```

#### 利用可能なモデル

| モデル名 | 次元数 | 特徴 |
|---------|--------|------|
| text-embedding-3-small | 1536 | コスト効率が高い（デフォルト） |
| text-embedding-3-large | 3072 | より高品質 |
| text-embedding-ada-002 | 1536 | レガシーモデル |

#### 料金目安（2024年時点）

| モデル | 料金 |
|--------|------|
| text-embedding-3-small | $0.02 / 1M tokens |
| text-embedding-3-large | $0.13 / 1M tokens |

---

### Ollama (ローカル実行) - 推奨

ローカルマシンでLLMを実行するためのオープンソースツール。**無料で利用可能**です。

#### セットアップ

1. Ollamaをインストール: https://ollama.ai/
2. Embeddingモデルをダウンロード:
   ```bash
   ollama pull nomic-embed-text
   ```
3. Ollamaサーバーが起動していることを確認（デフォルトでhttp://localhost:11434）

#### 設定ファイル (settings.json)

```json
{
  "embeddings": {
    "provider": "ollama",
    "endpoint": "http://localhost:11434",
    "model": "nomic-embed-text"
  }
}
```

#### CLI使用例

```bash
# Ollamaを使用してembeddings生成
jira-db embeddings --provider ollama

# カスタムモデルとエンドポイント
jira-db embeddings --provider ollama --model mxbai-embed-large --endpoint http://localhost:11434

# プロジェクト指定
jira-db embeddings --provider ollama --project PROJ
```

#### 利用可能なモデル

| モデル | 次元数 | 特徴 |
|--------|--------|------|
| nomic-embed-text | 768 | 高速、デフォルト |
| mxbai-embed-large | 1024 | 高品質 |
| snowflake-arctic-embed | 1024 | 高品質 |

#### 特徴

- ✅ 無料（ローカル実行）
- ✅ データがローカルに留まる（プライバシー保護）
- ⚠️ GPU推奨（CPUでも動作）

---

### LM Studio (ローカル実行)

デスクトップアプリでLLMをローカル実行。OpenAI互換APIを提供します。

#### セットアップ

1. LM Studioをインストール: https://lmstudio.ai/
2. アプリを起動し、Embeddingモデルをダウンロード（例: nomic-embed-text）
3. 「Local Server」タブでローカルサーバーを起動（デフォルト: http://localhost:1234）

#### 設定ファイル (settings.json)

```json
{
  "embeddings": {
    "provider": "openai",
    "endpoint": "http://localhost:1234/v1",
    "model": "text-embedding-nomic-embed-text-v1.5"
  }
}
```

**注意**: LM StudioはOpenAI互換APIを提供するため、`provider`は`"openai"`を指定します。

#### CLI使用例

```bash
# LM Studioを使用してembeddings生成
jira-db embeddings --provider openai --endpoint http://localhost:1234/v1 --model nomic-embed-text-v1.5

# プロジェクト指定
jira-db embeddings --provider openai --endpoint http://localhost:1234/v1 --project PROJ
```

#### 利用可能なモデル

| モデル | 次元数 | 特徴 |
|--------|--------|------|
| nomic-embed-text-v1.5 | 768 | 汎用、高品質 |
| bge-small-en-v1.5 | 384 | 軽量、英語向け |

#### 特徴

- ✅ 無料（ローカル実行）
- ✅ OpenAI互換API
- ✅ GUIでモデル管理が容易
- ✅ APIキー不要

---

### Cohere

Cohere社のEmbed API。**多言語サポートが優れている**ため、日本語の課題に最適です。

#### セットアップ

1. [Cohere](https://cohere.com/)でAPIキーを取得
2. 環境変数または設定ファイルでAPIキーを設定

#### 設定ファイル (settings.json)

```json
{
  "embeddings": {
    "provider": "cohere",
    "api_key": "your-cohere-key",
    "model": "embed-multilingual-v3.0"
  }
}
```

#### 環境変数での設定

```bash
export COHERE_API_KEY="your-cohere-key"
```

#### CLI使用例

```bash
# Cohereを使用してembeddings生成
jira-db embeddings --provider cohere

# 英語最適化モデルを使用
jira-db embeddings --provider cohere --model embed-english-v3.0

# プロジェクト指定
jira-db embeddings --provider cohere --project PROJ
```

#### 利用可能なモデル

| モデル | 次元数 | 特徴 |
|--------|--------|------|
| embed-multilingual-v3.0 | 1024 | 100+言語対応、デフォルト |
| embed-english-v3.0 | 1024 | 英語最適化 |
| embed-multilingual-light-v3.0 | 384 | 高速 |
| embed-english-light-v3.0 | 384 | 高速、英語 |

#### 特徴

- ✅ 多言語対応が優秀（特に日本語）
- ✅ 検索用途に最適化されたモデル
- ⚠️ バッチサイズ: 最大96

---

## ベクトル検索の仕組み

### DuckDB VSS Extension

jira-dbはDuckDBのVSS (Vector Similarity Search) 拡張を使用します。

```sql
-- HNSW インデックスの作成
CREATE INDEX idx_embeddings_hnsw
ON issue_embeddings
USING HNSW (embedding)
WITH (metric = 'cosine');

-- コサイン距離による類似検索
SELECT issue_key, summary,
       array_cosine_distance(embedding, query_embedding) as distance
FROM issue_embeddings
ORDER BY distance ASC
LIMIT 10;
```

### 類似度メトリクス

| メトリクス | 用途 | 特徴 |
|-----------|------|------|
| cosine | テキスト類似度（デフォルト） | ベクトルの向きを比較 |
| l2 | ユークリッド距離 | 絶対的な距離 |
| ip | 内積 | 正規化されたベクトル向け |

---

## パフォーマンス最適化

### バッチ処理

```bash
# 大規模なイシューセットの場合はバッチサイズを調整
jira-db embeddings --batch-size 50
```

### 増分更新

```bash
# 既存のembeddingsをスキップ（デフォルト動作）
jira-db embeddings

# 強制再生成（モデル変更時など）
jira-db embeddings --force
```

---

## トラブルシューティング

### API キーエラー

```
Error: OpenAI API key is required
```

**解決方法:**
```bash
export OPENAI_API_KEY="sk-..."
```

### レート制限

```
Error: Rate limit exceeded
```

**解決方法:**
- バッチサイズを小さくする: `--batch-size 20`
- リトライロジックは自動で処理されます

### 次元数の不一致

```
Error: Embedding dimension mismatch
```

**解決方法:**
- モデルを変更した場合は `--force` で再生成が必要です

### Ollamaに接続できない

```
Error: Failed to connect to Ollama
```

**解決方法:**
1. Ollamaが起動していることを確認: `ollama serve`
2. エンドポイントを確認: デフォルトは `http://localhost:11434`

### LM Studioに接続できない

```
Error: Failed to connect to local server
```

**解決方法:**
1. LM Studioの「Local Server」タブでサーバーが起動していることを確認
2. エンドポイントを確認: デフォルトは `http://localhost:1234/v1`

---

## カスタムプロバイダーの実装

新しいプロバイダーを追加する場合のインターフェース:

```rust
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// 単一テキストのembedding生成
    async fn embed(&self, text: &str) -> DomainResult<Vec<f32>>;

    /// バッチembedding生成
    async fn embed_batch(&self, texts: &[&str]) -> DomainResult<Vec<Vec<f32>>>;

    /// embedding次元数
    fn dimension(&self) -> usize;

    /// プロバイダー名（例: "openai", "ollama", "cohere"）
    fn provider_name(&self) -> &str;

    /// モデル名（例: "text-embedding-3-small", "nomic-embed-text"）
    fn model_name(&self) -> &str;
}
```

---

## 参考リンク

- [OpenAI Embeddings Guide](https://platform.openai.com/docs/guides/embeddings)
- [DuckDB VSS Extension](https://duckdb.org/docs/extensions/vss)
- [Cohere Embed API](https://docs.cohere.com/reference/embed)
- [Voyage AI Documentation](https://docs.voyageai.com/)
- [Ollama](https://ollama.ai/)
- [LM Studio](https://lmstudio.ai/)
