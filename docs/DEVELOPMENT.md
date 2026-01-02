# jira-db 開発ガイドライン

## 開発環境セットアップ

### 必須要件
- Rust 1.85以上（Edition 2024対応）
- DuckDB 1.4.1以上

### macOS セットアップ
```bash
# Rust インストール
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# DuckDB インストール
brew install duckdb

# ライブラリパス設定（.zshrc or .bashrc に追加）
export LIBRARY_PATH="/opt/homebrew/lib:$LIBRARY_PATH"
export LD_LIBRARY_PATH="/opt/homebrew/lib:$LD_LIBRARY_PATH"
```

### Linux セットアップ
```bash
# Rust インストール
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# DuckDB インストール（ソースからビルドまたはパッケージ）
# https://duckdb.org/docs/installation/
```

## ビルド

```bash
# 開発ビルド
cargo build

# リリースビルド
cargo build --release

# 特定クレートのみ
cargo build -p jira-db-core
cargo build -p jira-db-cli
cargo build -p jira-db-mcp
```

## テスト

```bash
# 全テスト実行
cargo test

# 出力付きテスト
cargo test -- --nocapture

# 特定テスト
cargo test test_search_issues

# 特定クレートのテスト
cargo test -p jira-db-core
```

## コード品質

### リンター
```bash
# Clippy実行
cargo clippy

# 警告をエラーとして扱う
cargo clippy -- -D warnings
```

### フォーマット
```bash
# コードフォーマット
cargo fmt

# フォーマットチェックのみ
cargo fmt --check
```

### 型チェック
```bash
cargo check
```

## ワークフロー

### 新機能開発
1. ブランチ作成: `git checkout -b feature/xxx`
2. ドメイン層から実装（entities → repositories）
3. インフラ層を実装（DB repositories）
4. アプリケーション層を実装（use cases）
5. プレゼンテーション層を実装（CLI/MCP）
6. テスト作成
7. PR作成

### バグ修正
1. 問題の再現確認
2. テストケース作成（失敗するテスト）
3. 修正実装
4. テスト通過確認
5. PR作成

## コーディング規約

### ファイル構成
- 1ファイル1責務
- モジュールは `mod.rs` で公開エクスポートを定義
- プライベート関数は公開関数の後に配置

### 命名規則
| 種類 | 規則 | 例 |
|------|------|-----|
| 構造体 | PascalCase | `IssueRepository` |
| トレイト | PascalCase | `Repository` |
| 関数 | snake_case | `find_by_key` |
| 定数 | SCREAMING_SNAKE_CASE | `MAX_RETRIES` |
| 変数 | snake_case | `issue_count` |

### エラーハンドリング
```rust
// DomainError を使用
fn some_function() -> DomainResult<SomeType> {
    something.map_err(|e| DomainError::Database(e.to_string()))?;
    Ok(result)
}

// context を追加する場合
fn load_config() -> DomainResult<Config> {
    let content = fs::read_to_string(&path)
        .map_err(|e| DomainError::Io(format!("Failed to read {}: {}", path.display(), e)))?;
    // ...
}
```

### ドキュメント
```rust
/// 簡潔な説明を1行で
///
/// 詳細な説明が必要な場合は空行を挟んで記述。
///
/// # Arguments
///
/// * `param1` - パラメータの説明
///
/// # Returns
///
/// 戻り値の説明
///
/// # Errors
///
/// エラーになる条件
///
/// # Examples
///
/// ```
/// let result = function(param);
/// ```
pub fn function(param1: Type) -> Result<Type> {
    // ...
}
```

### 非同期コード
```rust
// async/await を使用
pub async fn fetch_issues(&self) -> DomainResult<Vec<Issue>> {
    let response = self.client
        .get(&url)
        .send()
        .await
        .map_err(|e| DomainError::JiraApi(e.to_string()))?;
    // ...
}

// 同期コードをasyncでラップ
let result = tokio::task::spawn_blocking(|| {
    // 同期処理
}).await.map_err(|e| DomainError::Database(e.to_string()))?;
```

## アーキテクチャルール

### 依存関係の方向
```
Presentation → Application → Domain ← Infrastructure
                    ↓
              Infrastructure
```

- Domain層は他の層に依存しない
- Application層はDomain層のみに依存
- Infrastructure層はDomain層のインターフェースを実装
- Presentation層はApplication層を通じて処理

### 禁止事項
- Domain層での外部クレート直接使用（serde以外）
- Infrastructure層の直接使用（Presentation層から）
- 循環依存

### Repository実装規則
```rust
// ✅ Good: Domain trait を実装
impl IssueRepository for DuckDbIssueRepository {
    fn find_by_key(&self, key: &str) -> DomainResult<Option<Issue>> {
        // ...
    }
}

// ❌ Bad: 具象型を直接使用
fn get_issue(repo: DuckDbIssueRepository) { ... }

// ✅ Good: trait object を使用
fn get_issue(repo: Arc<dyn IssueRepository>) { ... }
```

## デバッグ

### ログレベル
```bash
# デバッグログ有効化
RUST_LOG=debug cargo run -p jira-db-cli -- sync

# トレースログ
RUST_LOG=trace cargo run -p jira-db-cli -- sync

# 特定モジュールのみ
RUST_LOG=jira_db_core::infrastructure=debug cargo run
```

### DuckDB直接操作
```bash
# DuckDB CLIで直接確認
duckdb ./data/jira.duckdb

# SQL実行
D SELECT * FROM issues LIMIT 5;
D .schema issues
```

## よくあるエラーと対処

### DuckDBリンクエラー
```
error: linking with `cc` failed
```
対処: LIBRARY_PATH環境変数を設定

### JIRA API 410エラー
```
410 Gone - deprecated endpoint
```
対処: `/rest/api/3/search` ではなく `/rest/api/3/search/jql` を使用

### JIRA API 認証エラー
```
401 Unauthorized
```
対処:
- APIトークンが正しいか確認（パスワードではない）
- エンドポイントURLが正しいか確認

## パフォーマンス最適化

### バッチサイズ
- JIRA API: 100件/リクエスト
- DB Insert: 1000件/トランザクション
- Embeddings: 50件/バッチ

### 並行処理
```rust
// 複数プロジェクトの並行同期
let handles: Vec<_> = projects
    .into_iter()
    .map(|p| tokio::spawn(sync_project(p)))
    .collect();

for handle in handles {
    handle.await??;
}
```

## リリース

### バージョン更新
1. `Cargo.toml` のversion更新
2. CHANGELOG更新
3. タグ作成: `git tag v0.1.0`
4. プッシュ: `git push origin v0.1.0`

### クロスコンパイル
```bash
# Windows向け
cargo build --release --target x86_64-pc-windows-gnu

# Linux向け（macOSから）
cargo build --release --target x86_64-unknown-linux-gnu
```
