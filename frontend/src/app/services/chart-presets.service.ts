/**
 * Chart Presets Service
 *
 * SQLクエリテンプレートとチャート設定のプリセットを提供します。
 * Visualizationコンポーネントで使用され、一般的なメトリクスの可視化を簡単に行えます。
 */

export type ChartType = 'line' | 'bar' | 'area' | 'stacked-area';

export interface ChartPreset {
  id: string;
  name: string;
  nameEn: string;
  description: string;
  chartType: ChartType;
  sqlTemplate: string;
  xColumn: string;
  yColumns: string[];
  groupByColumn?: string;
  colors?: string[];
}

// 残件数推移（バーンダウン）
const burndownSql = `-- チケット残件数推移（バーンダウンチャート）
-- resolution が NULL/空 = 未解決、それ以外 = 解決済み

WITH date_bounds AS (
  -- 最初の課題作成日と最後の更新日を取得
  SELECT
    MIN(valid_from)::DATE AS start_date,
    GREATEST(MAX(valid_from), MAX(valid_to), CURRENT_DATE)::DATE AS end_date
  FROM issue_snapshots
  WHERE valid_from IS NOT NULL
),
date_range AS (
  -- 連続した日付範囲を生成
  SELECT UNNEST(generate_series(
    (SELECT start_date FROM date_bounds),
    (SELECT end_date FROM date_bounds),
    INTERVAL '1 day'
  ))::DATE AS date
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
ORDER BY date`;

// 対応ペース（ベロシティ）
const velocitySql = `-- 対応ペース（日別解決件数）
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
ORDER BY "日付"`;

// 累積フロー図（CFD）
const cfdSql = `-- 累積フロー図（ステータス別件数推移）
-- 日付ごとに各ステータスの件数をカウント

WITH date_bounds AS (
  SELECT
    MIN(valid_from)::DATE AS start_date,
    GREATEST(MAX(valid_from), MAX(valid_to), CURRENT_DATE)::DATE AS end_date
  FROM issue_snapshots
  WHERE valid_from IS NOT NULL
),
date_range AS (
  SELECT UNNEST(generate_series(
    (SELECT start_date FROM date_bounds),
    (SELECT end_date FROM date_bounds),
    INTERVAL '1 day'
  ))::DATE AS date
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
ORDER BY date, status`;

// 週別ベロシティ
const weeklyVelocitySql = `-- 週別対応ペース
-- 週ごとの解決チケット数をカウント

WITH first_resolution AS (
  SELECT
    issue_key,
    MIN(valid_from) AS resolved_at
  FROM issue_snapshots
  WHERE resolution IS NOT NULL AND resolution != ''
  GROUP BY issue_key
)
SELECT
  DATE_TRUNC('week', resolved_at)::DATE AS "週",
  COUNT(*) AS "解決件数"
FROM first_resolution
GROUP BY DATE_TRUNC('week', resolved_at)
ORDER BY "週"`;

// 課題タイプ別残件数
const issueTypeBreakdownSql = `-- 課題タイプ別残件数推移
-- 日付ごとに各課題タイプの未解決件数をカウント

WITH date_bounds AS (
  SELECT
    MIN(valid_from)::DATE AS start_date,
    GREATEST(MAX(valid_from), MAX(valid_to), CURRENT_DATE)::DATE AS end_date
  FROM issue_snapshots
  WHERE valid_from IS NOT NULL
),
date_range AS (
  SELECT UNNEST(generate_series(
    (SELECT start_date FROM date_bounds),
    (SELECT end_date FROM date_bounds),
    INTERVAL '1 day'
  ))::DATE AS date
),
daily_state AS (
  SELECT
    d.date,
    s.issue_key,
    s.issue_type,
    s.resolution
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
  issue_type AS "課題タイプ",
  SUM(CASE WHEN resolution IS NULL OR resolution = '' THEN 1 ELSE 0 END) AS "残件数"
FROM daily_state
GROUP BY date, issue_type
ORDER BY date, issue_type`;

export const CHART_PRESETS: ChartPreset[] = [
  {
    id: 'burndown',
    name: '残件数推移',
    nameEn: 'Burndown Chart',
    description: '日付ごとの未解決チケット数の推移を表示します',
    chartType: 'line',
    sqlTemplate: burndownSql,
    xColumn: '日付',
    yColumns: ['残件数', '解決済み'],
    colors: ['#ef4444', '#22c55e'],
  },
  {
    id: 'velocity',
    name: '対応ペース（日別）',
    nameEn: 'Daily Velocity',
    description: '日ごとの解決チケット数を表示します',
    chartType: 'bar',
    sqlTemplate: velocitySql,
    xColumn: '日付',
    yColumns: ['解決件数'],
    colors: ['#3b82f6'],
  },
  {
    id: 'weekly-velocity',
    name: '対応ペース（週別）',
    nameEn: 'Weekly Velocity',
    description: '週ごとの解決チケット数を表示します',
    chartType: 'bar',
    sqlTemplate: weeklyVelocitySql,
    xColumn: '週',
    yColumns: ['解決件数'],
    colors: ['#8b5cf6'],
  },
  {
    id: 'cfd',
    name: '累積フロー図',
    nameEn: 'Cumulative Flow Diagram',
    description: 'ステータス別の件数推移を積み上げエリアチャートで表示します',
    chartType: 'stacked-area',
    sqlTemplate: cfdSql,
    xColumn: '日付',
    yColumns: ['件数'],
    groupByColumn: 'ステータス',
  },
  {
    id: 'issue-type-breakdown',
    name: '課題タイプ別残件',
    nameEn: 'Issue Type Breakdown',
    description: '課題タイプ別の残件数推移を表示します',
    chartType: 'stacked-area',
    sqlTemplate: issueTypeBreakdownSql,
    xColumn: '日付',
    yColumns: ['残件数'],
    groupByColumn: '課題タイプ',
  },
];

/**
 * プリセットIDからプリセットを取得
 */
export function getPresetById(id: string): ChartPreset | undefined {
  return CHART_PRESETS.find(p => p.id === id);
}

/**
 * プリセットのSQLテンプレートにプロジェクトIDを適用
 * （将来的にプロジェクトフィルターを追加する場合に使用）
 */
export function applyPresetParams(
  preset: ChartPreset,
  params: { projectId?: string; startDate?: string; endDate?: string }
): string {
  let sql = preset.sqlTemplate;

  // プロジェクトIDフィルターを追加（将来の拡張用）
  if (params.projectId) {
    sql = sql.replace(
      /FROM issue_snapshots/g,
      `FROM issue_snapshots\n  WHERE project_id = '${params.projectId}'`
    );
  }

  return sql;
}
