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

  // バーンダウンチャート用SQLテンプレート（バグの件数）
  burndownSqlTemplate = `-- バグ件数のバーンダウンチャート用SQL
-- issue_snapshots_expanded_readable テーブルを使用
-- 日付ごとの残バグ件数を計算

WITH daily_snapshots AS (
  SELECT
    DATE_TRUNC('day', valid_from) AS date,
    issue_key,
    status,
    issue_type,
    -- 各日付で最新のスナップショットを取得
    ROW_NUMBER() OVER (
      PARTITION BY issue_key, DATE_TRUNC('day', valid_from)
      ORDER BY valid_from DESC
    ) AS rn
  FROM issue_snapshots_expanded_readable
  WHERE valid_from IS NOT NULL
    AND issue_type = 'Bug'  -- バグのみを対象
),
daily_status AS (
  SELECT
    date,
    COUNT(*) AS total_bugs,
    SUM(CASE
      WHEN status IN ('Done', 'Closed', '完了', 'Resolved')
      THEN 1 ELSE 0
    END) AS closed_bugs
  FROM daily_snapshots
  WHERE rn = 1
  GROUP BY date
),
burndown AS (
  SELECT
    date,
    total_bugs,
    closed_bugs,
    SUM(closed_bugs) OVER (ORDER BY date) AS cumulative_closed,
    total_bugs - SUM(closed_bugs) OVER (ORDER BY date) AS remaining_bugs
  FROM daily_status
)
SELECT
  date AS "日付",
  total_bugs AS "総バグ数",
  cumulative_closed AS "累計クローズ数",
  remaining_bugs AS "残バグ数"
FROM burndown
ORDER BY date`;

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
    this.api.sqlGetSchema({ projectKey: this.projectKey || undefined }).subscribe({
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

    this.api.sqlGetSchema({ projectKey: this.projectKey || undefined, table: table.name }).subscribe({
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

    this.api.sqlExecute({ projectKey: this.projectKey || undefined, query: this.queryText(), limit: 100 }).subscribe({
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
}
