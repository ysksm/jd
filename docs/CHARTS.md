# Charts & Visualization

jira-dbのチャート・可視化機能についてのドキュメントです。

## 概要

フロントエンド（Tauri/Web）でJIRAデータを可視化するための機能を提供します。
- SQLクエリ結果のグラフ表示
- プリセットチャート（バーンダウン、ベロシティ、CFD）
- カスタムSQLによる柔軟な可視化

## 利用可能なチャートプリセット

### 1. 残件数推移（Burndown Chart）

日付ごとの未解決チケット数の推移を表示します。

| 項目 | 説明 |
|------|------|
| X軸 | 日付 |
| Y軸 | 件数 |
| メトリクス | 総件数、残件数、解決済み |
| 判定基準 | `resolution` が NULL/空 = 未解決 |

### 2. 対応ペース（Velocity Chart）

日別または週別の解決チケット数を表示します。

| 項目 | 説明 |
|------|------|
| X軸 | 日付/週 |
| Y軸 | 解決件数 |
| チャートタイプ | 棒グラフ |

### 3. 累積フロー図（Cumulative Flow Diagram）

ステータス別の件数推移を積み上げエリアチャートで表示します。

| 項目 | 説明 |
|------|------|
| X軸 | 日付 |
| Y軸 | 件数 |
| 内訳 | ステータス別 |

### 4. 課題タイプ別残件

課題タイプ（Bug, Story等）別の残件数推移を表示します。

## 使用方法

### SQL Queryページ

1. Projects → プロジェクト選択 → **SQL Query** タブ
2. プリセットボタンをクリック：
   - **残件数推移** - バーンダウンチャート用SQL
   - **対応ペース** - ベロシティチャート用SQL
   - **累積フロー** - CFD用SQL
3. **Execute** ボタンでクエリ実行
4. 結果をChartsタブで可視化

### Chartsページ

1. Projects → プロジェクト選択 → **Charts** タブ
2. **Chart Presets** セクションからプリセットを選択
3. 自動的にSQLが実行され、グラフが表示される

## SQLクエリの詳細

### 残件数推移SQL

```sql
-- チケット残件数推移（バーンダウンチャート）
-- issue_snapshotsテーブルを直接使用
-- resolution が NULL/空 = 未解決、それ以外 = 解決済み

WITH date_range AS (
  SELECT DISTINCT DATE_TRUNC('day', valid_from)::DATE AS date
  FROM issue_snapshots
  WHERE valid_from IS NOT NULL
),
daily_state AS (
  SELECT
    d.date,
    s.issue_key,
    s.resolution,
    s.status
  FROM date_range d
  JOIN issue_snapshots s ON
    s.valid_from <= d.date + INTERVAL '1 day'
    AND (s.valid_to IS NULL OR s.valid_to > d.date)
  QUALIFY ROW_NUMBER() OVER (
    PARTITION BY d.date, s.issue_key
    ORDER BY s.valid_from DESC
  ) = 1
)
SELECT
  date AS "日付",
  COUNT(*) AS "総件数",
  SUM(CASE WHEN resolution IS NULL OR resolution = '' THEN 1 ELSE 0 END) AS "残件数",
  SUM(CASE WHEN resolution IS NOT NULL AND resolution != '' THEN 1 ELSE 0 END) AS "解決済み"
FROM daily_state
GROUP BY date
ORDER BY date
```

### 対応ペースSQL

```sql
-- 対応ペース（日別解決件数）
-- 各日に解決されたチケット数をカウント

WITH first_resolution AS (
  SELECT
    issue_key,
    MIN(valid_from) AS resolved_at
  FROM issue_snapshots
  WHERE resolution IS NOT NULL AND resolution != ''
  GROUP BY issue_key
)
SELECT
  DATE_TRUNC('day', resolved_at)::DATE AS "日付",
  COUNT(*) AS "解決件数"
FROM first_resolution
GROUP BY DATE_TRUNC('day', resolved_at)
ORDER BY "日付"
```

### 累積フロー図SQL

```sql
-- 累積フロー図（ステータス別件数推移）
-- 日付ごとに各ステータスの件数をカウント

WITH date_range AS (
  SELECT DISTINCT DATE_TRUNC('day', valid_from)::DATE AS date
  FROM issue_snapshots
  WHERE valid_from IS NOT NULL
),
daily_state AS (
  SELECT
    d.date,
    s.issue_key,
    s.status
  FROM date_range d
  JOIN issue_snapshots s ON
    s.valid_from <= d.date + INTERVAL '1 day'
    AND (s.valid_to IS NULL OR s.valid_to > d.date)
  QUALIFY ROW_NUMBER() OVER (
    PARTITION BY d.date, s.issue_key
    ORDER BY s.valid_from DESC
  ) = 1
)
SELECT
  date AS "日付",
  status AS "ステータス",
  COUNT(*) AS "件数"
FROM daily_state
GROUP BY date, status
ORDER BY date, status
```

## データソース

### issue_snapshots テーブル

チャート機能は `issue_snapshots` テーブルを使用します。このテーブルはチケットの状態変更履歴をスナップショットとして保存しています。

| カラム | 型 | 説明 |
|--------|------|------|
| issue_id | VARCHAR | チケットID |
| issue_key | VARCHAR | チケットキー (例: PROJ-123) |
| project_id | VARCHAR | プロジェクトID |
| valid_from | TIMESTAMPTZ | スナップショット開始日時 |
| valid_to | TIMESTAMPTZ | スナップショット終了日時 (NULL = 現在有効) |
| status | VARCHAR | ステータス |
| resolution | VARCHAR | 解決状況 (NULL = 未解決) |
| priority | VARCHAR | 優先度 |
| issue_type | VARCHAR | 課題タイプ |
| assignee | VARCHAR | 担当者 |

## チャートライブラリ

- **uPlot**: 線グラフ、棒グラフ、エリアチャート、散布図
- **ECharts**: バブルチャート、ヒートマップ、積み上げエリアチャート

## 技術的な注意事項

### SQLセキュリティ

SQL実行は以下の制約があります：

1. **読み取り専用**: SELECT文とWITH...SELECT（CTE）のみ許可
2. **禁止キーワード**: INSERT, UPDATE, DELETE, DROP, CREATE, ALTER, TRUNCATE, EXEC, EXECUTE
3. **コメント対応**: SQLコメント（`-- ...`）は無視されます

### パフォーマンス

- デフォルトで最大10,000行を取得
- 大量データの場合は日付範囲でフィルタリングを推奨

## 関連ファイル

### フロントエンド

| ファイル | 説明 |
|----------|------|
| `frontend/src/app/pages/query/query.component.ts` | SQLクエリページ、プリセットSQL |
| `frontend/src/app/pages/visualization/visualization.component.ts` | チャート表示 |
| `frontend/src/app/services/chart-presets.service.ts` | チャートプリセット定義 |

### バックエンド

| ファイル | 説明 |
|----------|------|
| `crates/jira-db-core/src/application/use_cases/execute_sql.rs` | SQL実行ユースケース |
| `crates/jira-db-core/src/infrastructure/database/schema.rs` | issue_snapshotsテーブル定義 |
