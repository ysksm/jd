import { Component, OnInit, OnChanges, SimpleChanges, Input, signal, computed, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { SavedQuery, SqlTable, SqlColumn } from '../../generated/models';
import { API_SERVICE, IApiService } from '../../api.provider';

@Component({
  selector: 'app-query',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './query.component.html',
  styleUrl: './query.component.scss'
})
export class QueryComponent implements OnInit, OnChanges {
  private api = inject<IApiService>(API_SERVICE);

  // Input for project context (when used inside project detail)
  @Input() projectKey: string = '';

  // All projects mode (query across all synced projects)
  allProjects = signal(false);

  // Query editor state
  queryText = signal('SELECT * FROM issues LIMIT 10');
  queryName = signal('');
  queryDescription = signal('');
  editingQueryId = signal<string | null>(null);

  // Results state
  columns = signal<string[]>([]);
  rows = signal<Record<string, unknown>[]>([]);
  rowCount = signal(0);
  executionTimeMs = signal(0);

  // Schema state
  tables = signal<SqlTable[]>([]);
  selectedTable = signal<SqlTable | null>(null);
  tableColumns = signal<SqlColumn[]>([]);

  // Saved queries
  savedQueries = signal<SavedQuery[]>([]);

  // UI state
  loading = signal(false);
  schemaLoading = signal(false);
  error = signal<string | null>(null);
  successMessage = signal<string | null>(null);
  showSaveModal = signal(false);
  activeTab = signal<'results' | 'schema'>('results');

  // Schema panel visibility
  showSchemaPanel = signal(true);

  // Computed
  hasResults = computed(() => this.columns().length > 0);

  // バーンダウンチャート用SQLテンプレート（全チケット対象）
  burndownSqlTemplate = `-- チケット残件数推移（バーンダウンチャート）
-- issue_snapshotsテーブルを直接使用
-- resolution が NULL/空 = 未解決、それ以外 = 解決済み

WITH date_range AS (
  -- 日付の範囲を取得
  SELECT DISTINCT DATE_TRUNC('day', valid_from)::DATE AS date
  FROM issue_snapshots
  WHERE valid_from IS NOT NULL
),
daily_state AS (
  -- 各日付における各チケットの最新状態を取得
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

  // ベロシティチャート用SQLテンプレート（日別解決件数）
  velocitySqlTemplate = `-- 対応ペース（日別解決件数）
-- 各日に解決されたチケット数をカウント

WITH first_resolution AS (
  -- 各チケットが最初に解決された日を特定
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

  // 累積フロー図用SQLテンプレート（ステータス別件数推移）
  cfdSqlTemplate = `-- 累積フロー図（ステータス別件数推移）
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
ORDER BY date, status`;

  ngOnInit(): void {
    this.initializeComponent();
  }

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['projectKey'] && !changes['projectKey'].firstChange) {
      this.initializeComponent();
    }
  }

  private initializeComponent(): void {
    this.loadSchema();
    this.loadSavedQueries();
  }

  loadSchema(): void {
    this.schemaLoading.set(true);
    const request = this.allProjects()
      ? { allProjects: true }
      : { projectKey: this.projectKey || undefined };
    this.api.sqlGetSchema(request).subscribe({
      next: (response) => {
        this.tables.set(response.tables);
        this.schemaLoading.set(false);
      },
      error: (err) => {
        console.error('Failed to load schema:', err);
        this.schemaLoading.set(false);
      }
    });
  }

  loadTableColumns(table: SqlTable): void {
    if (table.columns) {
      this.selectedTable.set(table);
      this.tableColumns.set(table.columns);
      return;
    }

    const request = this.allProjects()
      ? { allProjects: true, table: table.name }
      : { projectKey: this.projectKey || undefined, table: table.name };
    this.api.sqlGetSchema(request).subscribe({
      next: (response) => {
        if (response.tables.length > 0 && response.tables[0].columns) {
          const updatedTable = { ...table, columns: response.tables[0].columns };
          this.selectedTable.set(updatedTable);
          this.tableColumns.set(response.tables[0].columns);
        }
      },
      error: (err) => {
        console.error('Failed to load table columns:', err);
      }
    });
  }

  toggleAllProjects(enabled: boolean): void {
    this.allProjects.set(enabled);
    // Reload schema when toggling
    this.selectedTable.set(null);
    this.tableColumns.set([]);
    this.loadSchema();
  }

  loadSavedQueries(): void {
    this.api.sqlListQueries({}).subscribe({
      next: (response) => {
        this.savedQueries.set(response.queries);
      },
      error: (err) => {
        console.error('Failed to load saved queries:', err);
      }
    });
  }

  executeQuery(): void {
    this.loading.set(true);
    this.error.set(null);
    this.successMessage.set(null);

    const request = this.allProjects()
      ? { allProjects: true, query: this.queryText(), limit: 500 }
      : { projectKey: this.projectKey || undefined, query: this.queryText(), limit: 500 };

    this.api.sqlExecute(request).subscribe({
      next: (response) => {
        this.columns.set(response.columns);
        this.rows.set(response.rows as Record<string, unknown>[]);
        this.rowCount.set(response.rowCount);
        this.executionTimeMs.set(response.executionTimeMs);
        this.loading.set(false);
        this.activeTab.set('results');
      },
      error: (err) => {
        this.error.set('Query execution failed: ' + err);
        this.loading.set(false);
      }
    });
  }

  openSaveModal(): void {
    if (!this.editingQueryId()) {
      this.queryName.set('');
      this.queryDescription.set('');
    }
    this.showSaveModal.set(true);
  }

  closeSaveModal(): void {
    this.showSaveModal.set(false);
    this.editingQueryId.set(null);
    this.queryName.set('');
    this.queryDescription.set('');
  }

  saveQuery(): void {
    if (!this.queryName().trim()) {
      this.error.set('Query name is required');
      return;
    }

    this.api.sqlSaveQuery({
      id: this.editingQueryId() || undefined,
      name: this.queryName(),
      query: this.queryText(),
      description: this.queryDescription() || undefined
    }).subscribe({
      next: () => {
        this.successMessage.set('Query saved successfully');
        this.loadSavedQueries();
        this.closeSaveModal();
        setTimeout(() => this.successMessage.set(null), 3000);
      },
      error: (err) => {
        this.error.set('Failed to save query: ' + err);
      }
    });
  }

  loadQuery(query: SavedQuery): void {
    this.queryText.set(query.query);
    this.editingQueryId.set(query.id);
    this.queryName.set(query.name);
    this.queryDescription.set(query.description || '');
  }

  deleteQuery(query: SavedQuery, event: Event): void {
    event.stopPropagation();
    if (confirm(`Delete query "${query.name}"?`)) {
      this.api.sqlDeleteQuery({ id: query.id }).subscribe({
        next: () => {
          this.loadSavedQueries();
          if (this.editingQueryId() === query.id) {
            this.editingQueryId.set(null);
          }
        },
        error: (err) => {
          this.error.set('Failed to delete query: ' + err);
        }
      });
    }
  }

  insertTableName(tableName: string): void {
    const currentQuery = this.queryText();
    if (currentQuery.includes('FROM')) {
      // Replace table name after FROM
      const newQuery = currentQuery.replace(/FROM\s+\w+/i, `FROM ${tableName}`);
      this.queryText.set(newQuery);
    } else {
      this.queryText.set(`SELECT * FROM ${tableName} LIMIT 10`);
    }
  }

  insertColumnName(columnName: string): void {
    const currentQuery = this.queryText();
    // Simple insertion at cursor position - for now just append to SELECT
    if (currentQuery.toUpperCase().startsWith('SELECT *')) {
      const newQuery = currentQuery.replace(/SELECT \*/i, `SELECT ${columnName}`);
      this.queryText.set(newQuery);
    } else if (currentQuery.toUpperCase().startsWith('SELECT')) {
      const newQuery = currentQuery.replace(/SELECT /i, `SELECT ${columnName}, `);
      this.queryText.set(newQuery);
    }
  }

  clearResults(): void {
    this.columns.set([]);
    this.rows.set([]);
    this.rowCount.set(0);
    this.executionTimeMs.set(0);
  }

  newQuery(): void {
    this.queryText.set('SELECT * FROM issues LIMIT 10');
    this.editingQueryId.set(null);
    this.queryName.set('');
    this.queryDescription.set('');
    this.clearResults();
  }

  onKeypress(event: KeyboardEvent): void {
    // Execute on Ctrl+Enter or Cmd+Enter
    if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
      event.preventDefault();
      this.executeQuery();
    }
  }

  formatValue(value: unknown): string {
    if (value === null || value === undefined) {
      return 'NULL';
    }
    if (typeof value === 'object') {
      return JSON.stringify(value);
    }
    return String(value);
  }

  toggleSchemaPanel(): void {
    this.showSchemaPanel.set(!this.showSchemaPanel());
  }

  // バーンダウンチャート用SQLをエディタに挿入
  insertBurndownSql(): void {
    this.queryText.set(this.burndownSqlTemplate);
    this.editingQueryId.set(null);
    this.queryName.set('');
    this.queryDescription.set('');
  }

  // ベロシティチャート用SQLをエディタに挿入
  insertVelocitySql(): void {
    this.queryText.set(this.velocitySqlTemplate);
    this.editingQueryId.set(null);
    this.queryName.set('');
    this.queryDescription.set('');
  }

  // 累積フロー図用SQLをエディタに挿入
  insertCfdSql(): void {
    this.queryText.set(this.cfdSqlTemplate);
    this.editingQueryId.set(null);
    this.queryName.set('');
    this.queryDescription.set('');
  }
}
