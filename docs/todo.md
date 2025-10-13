# jira-db タスク管理

## ステータス凡例
- `[ ]` 未着手
- `[>]` 進行中
- `[x]` 完了
- `[-]` スキップ/保留

---

## Phase 1: 基盤構築

### 1.1 プロジェクト構造
- [x] モジュール構造を作成
  - [x] `src/config/mod.rs`
  - [x] `src/jira/mod.rs`
  - [x] `src/sync/mod.rs`
  - [x] `src/db/mod.rs`
  - [x] `src/cli/mod.rs`
  - [x] `src/error.rs`

### 1.2 依存関係
- [x] jira-api を追加
- [x] 必要なクレートをCargo.tomlに追加
  - [x] duckdb
  - [x] serde, serde_json
  - [x] tokio
  - [x] anyhow, thiserror
  - [x] clap
  - [x] log, env_logger
  - [x] dirs

### 1.3 エラーハンドリング
- [x] `src/error.rs` にカスタムエラー型を定義
- [x] 各モジュールでのエラー変換実装

### 1.4 ロギング
- [x] env_loggerの初期化
- [x] ログレベルの設定
- [x] 各モジュールでのログ出力実装

---

## Phase 2: 設定管理

### 2.1 データ構造
- [x] `src/config/settings.rs` にSettings構造体を定義
  - [x] JiraConfig構造体
  - [x] ProjectConfig構造体
  - [x] DatabaseConfig構造体

### 2.2 ファイルI/O
- [x] 設定ファイルパスの決定（`~/.config/jira-db/settings.json`）
- [x] 設定ファイル読み込み関数
- [x] 設定ファイル書き込み関数
- [x] 初回起動時の設定ファイル生成

### 2.3 バリデーション
- [x] バリデーション実装（settings.rsのvalidateメソッド）
- [x] エンドポイントURLの検証
- [x] APIキーの存在確認
- [x] パス存在チェック

---

## Phase 3: データベース層

### 3.1 接続管理
- [x] `src/db/connection.rs` 実装
- [x] DuckDB接続の確立
- [x] 接続プールの管理（Arc<Mutex>で実装）

### 3.2 スキーマ定義
- [x] `src/db/schema.rs` 実装
- [x] projectsテーブルのCREATE文
- [x] issuesテーブルのCREATE文
- [x] sync_historyテーブルのCREATE文
- [x] インデックスの作成

### 3.3 リポジトリ
- [x] `src/db/repository.rs` 実装
- [x] ProjectRepository
  - [x] insert
  - [x] find_by_key
  - [x] find_all
- [x] IssueRepository
  - [x] batch_insert
  - [x] find_by_project
  - [x] count_by_project
- [x] SyncHistoryRepository
  - [x] insert
  - [x] update_completed
  - [x] update_failed
  - [x] find_latest_by_project

### 3.4 初期化
- [x] データベース初期化関数
- [x] スキーマのマイグレーション処理

---

## Phase 4: JIRA統合

### 4.1 クライアントラッパー
- [x] `src/jira/client.rs` 実装
- [x] jira-apiクライアントの初期化
- [x] 認証設定

### 4.2 データモデル
- [x] `src/jira/models.rs` 実装
- [x] Project構造体
- [x] Issue構造体
- [x] JSONからの変換実装

### 4.3 API呼び出し
- [x] プロジェクト一覧取得
- [x] イシュー一覧取得（ページネーション対応）
- [x] JQL検索のサポート（jira-api経由）

### 4.4 エラーハンドリング
- [x] APIエラーのハンドリング
- [x] リトライロジック（レート制限対応）
- [x] タイムアウト処理

---

## Phase 5: 同期機能

### 5.1 同期マネージャー
- [x] `src/sync/manager.rs` 実装
- [x] SyncManager構造体
- [x] 同期対象プロジェクトの判定

### 5.2 同期ロジック
- [x] フル同期の実装
  - [x] プロジェクトデータの同期
  - [x] イシューデータのバッチ取得
  - [x] データベースへの保存
- [ ] 増分同期の実装（オプション）
  - [ ] 最終同期日時の取得
  - [ ] 差分データの取得

### 5.3 同期履歴
- [x] 同期開始時の履歴記録
- [x] 同期完了時のステータス更新
- [x] エラー時のログ記録

### 5.4 進捗表示
- [x] 同期中の進捗バー表示
- [x] 処理件数の表示（ログ出力）

---

## Phase 6: CLI実装

### 6.1 CLI基盤
- [x] `src/cli/commands.rs` 実装
- [x] clapでのコマンド定義
- [x] サブコマンド構造の設定

### 6.2 initコマンド
- [x] `jira-db init` コマンド実装
- [x] 対話的な設定入力
- [x] 設定ファイルの生成
- [x] 初回接続テスト

### 6.3 syncコマンド
- [x] `jira-db sync` コマンド実装
- [x] プロジェクト一覧の更新
- [x] 同期対象プロジェクトのデータ取得
- [x] `--project` オプション（特定プロジェクトのみ同期）
- [x] `--force` オプション（定義済み）

### 6.4 listコマンド
- [x] `jira-db list` コマンド実装
- [x] プロジェクト一覧の表示
- [x] 同期ステータスの表示
- [x] 最終同期日時の表示（--verbose）

### 6.5 configコマンド
- [x] `jira-db config show` コマンド実装
- [x] `jira-db config set` コマンド実装
- [x] `jira-db config enable-sync <project-key>` 実装
- [x] `jira-db config disable-sync <project-key>` 実装

### 6.6 searchコマンド（Phase 7後）
- [x] `jira-db search` コマンド実装
- [x] 基本的なキーワード検索
- [x] フィルタオプション

---

## Phase 7: 検索機能

### 7.1 インデックス
- [x] 検索用リポジトリメソッド実装
- [x] 動的SQLクエリ生成
- [x] LIKE検索による全文検索（DuckDB）

### 7.2 検索クエリ
- [x] 基本的なテキスト検索（summary, description）
- [x] ステータスフィルタ
- [x] プロジェクトフィルタ（project_key）
- [x] 担当者フィルタ（assignee）

### 7.3 結果表示
- [x] 検索結果のテーブル表示（comfy-table）
- [x] ページネーション（limit, offset）
- [x] 結果のソート（created_date DESC）

---

## Phase 8: テスト

### 8.1 ユニットテスト
- [ ] config モジュールのテスト
- [ ] db モジュールのテスト
- [ ] sync モジュールのテスト

### 8.2 統合テスト
- [ ] JIRA APIモックを使用したテスト
- [ ] データベース操作のテスト
- [ ] エンドツーエンドフロー

### 8.3 ドキュメント
- [x] README.md の作成
- [x] CLAUDE.md の更新
- [x] docs/requirements.md の作成
- [x] docs/plan.md の作成
- [x] docs/todo.md の作成

---

## Phase 9: 最適化・改善

### 9.1 パフォーマンス
- [ ] バッチサイズの最適化
- [ ] 並行処理の実装
- [ ] メモリ使用量の最適化

### 9.2 ユーザビリティ
- [ ] エラーメッセージの改善
- [ ] ヘルプテキストの充実
- [ ] デバッグモードの追加

### 9.3 セキュリティ
- [ ] 設定ファイルのパーミッション設定
- [ ] 環境変数サポート
- [ ] ログからの機密情報除外

---

## 優先度

### 高優先度（MVP）
1. Phase 1: 基盤構築
2. Phase 2: 設定管理
3. Phase 3: データベース層
4. Phase 4: JIRA統合
5. Phase 5: 同期機能（フル同期のみ）
6. Phase 6: CLI実装（init, sync, listコマンド）

### 中優先度
7. Phase 7: 検索機能
8. Phase 6: CLI実装（config, searchコマンド）
9. Phase 8: テスト

### 低優先度（将来の拡張）
10. Phase 9: 最適化・改善
11. 増分同期
12. Webhookサポート
13. Webインターフェース

---

## 進捗メモ

### 完了済み（2025-10-13〜2025-10-14）
- [x] Phase 1: 基盤構築（完全完了）
- [x] Phase 2: 設定管理（完全完了）
- [x] Phase 3: データベース層（完全完了）
- [x] Phase 4: JIRA統合（完全完了）
  - [x] リトライロジック実装（指数バックオフ）
  - [x] タイムアウト処理実装
- [x] Phase 5: 同期機能（完全完了）
  - [x] フル同期実装
  - [x] 進捗バー表示実装
- [x] Phase 6: CLI実装（完全完了）
  - [x] init, project, sync, config, searchコマンド
  - [x] 対話的な設定入力（--interactive）
  - [x] 接続テスト機能
- [x] Phase 7: 検索機能（完全完了）
  - [x] テキスト検索、フィルタ、ページネーション
  - [x] テーブル形式での結果表示
- [x] CLI構造の改善（projectサブコマンド導入）
- [x] Rust Edition 2024 に更新
- [x] モジュール構造の現代化（mod.rs → フォルダ名.rs）
- [x] DuckDB 1.4.1 インストール・統合
- [x] 設定ファイルパス変更（`~/.config/jira-db/settings.json` → `./settings.json`）
- [x] README.md 作成完了
- [x] CLAUDE.md 更新完了

### CLI構造の改善内容
旧コマンド体系から新しい体系に変更：
- `jira-db init` → 設定ファイル作成のみに専念
- `jira-db sync` → プロジェクト初期化とデータ同期を分離
- 新規 `jira-db project` サブコマンド導入
  - `project init` - プロジェクト一覧取得
  - `project list` - プロジェクト表示
  - `project enable/disable` - 同期制御
- `config enable-sync/disable-sync` → `project enable/disable` に移動

### 現在の状況
- ✅ **Phase 1〜7 完全実装済み**
- ✅ 本番利用可能な状態
- ✅ 完全なJIRA同期機能（リトライ・タイムアウト対応）
- ✅ 強力な検索機能（フィルタ、ページネーション）
- ✅ 進捗バー表示による視覚的フィードバック
- ✅ 対話的な初期設定（--interactive）
- ✅ 設定管理、データベース、CLI完備
- ✅ ドキュメント整備済み（README、CLAUDE.md、plan.md、todo.md、requirements.md）
- ✅ ユーザーフレンドリーなCLI構造
- ✅ 設定ファイルはカレントディレクトリに保存（プロジェクト単位での管理が可能）
- ✅ モダンなRust 2024コード構造

### 追加実装された機能（Phase 7完了）
- **検索機能**: テキスト検索、複数フィルタ、ページネーション、ソート
- **進捗表示**: 同期中の視覚的な進捗バー
- **対話的init**: --interactiveフラグによる質問形式の初期設定
- **エラーハンドリング**: 指数バックオフ、タイムアウト、自動リトライ

### 次のアクション（オプション）
1. Phase 8: テストの追加（品質保証）
2. Phase 5.2: 増分同期実装（パフォーマンス最適化）
3. Phase 9: 最適化・改善（並行処理、環境変数対応など）
