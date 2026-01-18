import { Component, OnInit, OnChanges, SimpleChanges, Input, signal, computed, inject, effect } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { SavedQuery, SqlTable, SqlColumn } from '../../generated/models';
import { API_SERVICE, IApiService } from '../../api.provider';

// Interface for a query tab
interface QueryTab {
  id: string;
  name: string;
  queryText: string;
  queryName: string;
  queryDescription: string;
  editingQueryId: string | null;
  columns: string[];
  rows: Record<string, unknown>[];
  rowCount: number;
  executionTimeMs: number;
  loading: boolean;
  error: string | null;
  resultsTab: 'results' | 'schema';
}

// Counter for generating unique tab IDs
let tabIdCounter = 0;

function createNewTab(name?: string): QueryTab {
  tabIdCounter++;
  return {
    id: `tab-${tabIdCounter}`,
    name: name || `Query ${tabIdCounter}`,
    queryText: 'SELECT * FROM issues LIMIT 10',
    queryName: '',
    queryDescription: '',
    editingQueryId: null,
    columns: [],
    rows: [],
    rowCount: 0,
    executionTimeMs: 0,
    loading: false,
    error: null,
    resultsTab: 'results'
  };
}

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

  // Multi-tab state
  tabs = signal<QueryTab[]>([createNewTab()]);
  activeTabId = signal<string>('tab-1');

  // Current tab computed property
  currentTab = computed(() => {
    const tab = this.tabs().find(t => t.id === this.activeTabId());
    return tab || this.tabs()[0];
  });

  // Query editor state (delegated to current tab)
  queryText = computed(() => this.currentTab().queryText);
  queryName = computed(() => this.currentTab().queryName);
  queryDescription = computed(() => this.currentTab().queryDescription);
  editingQueryId = computed(() => this.currentTab().editingQueryId);

  // Results state (delegated to current tab)
  columns = computed(() => this.currentTab().columns);
  rows = computed(() => this.currentTab().rows);
  rowCount = computed(() => this.currentTab().rowCount);
  executionTimeMs = computed(() => this.currentTab().executionTimeMs);

  // Loading and error state (delegated to current tab)
  loading = computed(() => this.currentTab().loading);
  error = computed(() => this.currentTab().error);
  activeResultsTab = computed(() => this.currentTab().resultsTab);

  // Schema state (shared across all tabs)
  tables = signal<SqlTable[]>([]);
  selectedTable = signal<SqlTable | null>(null);
  tableColumns = signal<SqlColumn[]>([]);

  // Saved queries
  savedQueries = signal<SavedQuery[]>([]);

  // UI state
  schemaLoading = signal(false);
  successMessage = signal<string | null>(null);
  showSaveModal = signal(false);

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

  // Tab management methods
  addTab(): void {
    const newTab = createNewTab();
    this.tabs.update(tabs => [...tabs, newTab]);
    this.activeTabId.set(newTab.id);
  }

  closeTab(tabId: string, event: Event): void {
    event.stopPropagation();
    const currentTabs = this.tabs();
    if (currentTabs.length <= 1) {
      // Don't close the last tab, just reset it
      this.newQuery();
      return;
    }

    const tabIndex = currentTabs.findIndex(t => t.id === tabId);
    const newTabs = currentTabs.filter(t => t.id !== tabId);
    this.tabs.set(newTabs);

    // If we closed the active tab, switch to another one
    if (this.activeTabId() === tabId) {
      // Try to switch to the next tab, or the previous one if closing the last
      const newIndex = Math.min(tabIndex, newTabs.length - 1);
      this.activeTabId.set(newTabs[newIndex].id);
    }
  }

  switchTab(tabId: string): void {
    this.activeTabId.set(tabId);
  }

  private updateCurrentTab(updates: Partial<QueryTab>): void {
    this.tabs.update(tabs =>
      tabs.map(tab =>
        tab.id === this.activeTabId()
          ? { ...tab, ...updates }
          : tab
      )
    );
  }

  setQueryText(value: string): void {
    this.updateCurrentTab({ queryText: value });
  }

  setQueryName(value: string): void {
    this.updateCurrentTab({ queryName: value });
  }

  setQueryDescription(value: string): void {
    this.updateCurrentTab({ queryDescription: value });
  }

  setResultsTab(value: 'results' | 'schema'): void {
    this.updateCurrentTab({ resultsTab: value });
  }

  setError(value: string | null): void {
    this.updateCurrentTab({ error: value });
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
    this.updateCurrentTab({ loading: true, error: null });
    this.successMessage.set(null);

    const request = this.allProjects()
      ? { allProjects: true, query: this.queryText(), limit: 500 }
      : { projectKey: this.projectKey || undefined, query: this.queryText(), limit: 500 };

    this.api.sqlExecute(request).subscribe({
      next: (response) => {
        this.updateCurrentTab({
          columns: response.columns,
          rows: response.rows as Record<string, unknown>[],
          rowCount: response.rowCount,
          executionTimeMs: response.executionTimeMs,
          loading: false,
          resultsTab: 'results'
        });
      },
      error: (err) => {
        this.updateCurrentTab({
          error: 'Query execution failed: ' + err,
          loading: false
        });
      }
    });
  }

  openSaveModal(): void {
    if (!this.editingQueryId()) {
      this.updateCurrentTab({ queryName: '', queryDescription: '' });
    }
    this.showSaveModal.set(true);
  }

  closeSaveModal(): void {
    this.showSaveModal.set(false);
    this.updateCurrentTab({
      editingQueryId: null,
      queryName: '',
      queryDescription: ''
    });
  }

  saveQuery(): void {
    if (!this.queryName().trim()) {
      this.updateCurrentTab({ error: 'Query name is required' });
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
        this.updateCurrentTab({ error: 'Failed to save query: ' + err });
      }
    });
  }

  loadQuery(query: SavedQuery): void {
    this.updateCurrentTab({
      queryText: query.query,
      editingQueryId: query.id,
      queryName: query.name,
      queryDescription: query.description || ''
    });
    // Update tab name to match the loaded query
    this.tabs.update(tabs =>
      tabs.map(tab =>
        tab.id === this.activeTabId()
          ? { ...tab, name: query.name }
          : tab
      )
    );
  }

  deleteQuery(query: SavedQuery, event: Event): void {
    event.stopPropagation();
    if (confirm(`Delete query "${query.name}"?`)) {
      this.api.sqlDeleteQuery({ id: query.id }).subscribe({
        next: () => {
          this.loadSavedQueries();
          if (this.editingQueryId() === query.id) {
            this.updateCurrentTab({ editingQueryId: null });
          }
        },
        error: (err) => {
          this.updateCurrentTab({ error: 'Failed to delete query: ' + err });
        }
      });
    }
  }

  insertTableName(tableName: string): void {
    const currentQuery = this.queryText();
    if (currentQuery.includes('FROM')) {
      // Replace table name after FROM
      const newQuery = currentQuery.replace(/FROM\s+\w+/i, `FROM ${tableName}`);
      this.updateCurrentTab({ queryText: newQuery });
    } else {
      this.updateCurrentTab({ queryText: `SELECT * FROM ${tableName} LIMIT 10` });
    }
  }

  insertColumnName(columnName: string): void {
    const currentQuery = this.queryText();
    // Simple insertion at cursor position - for now just append to SELECT
    if (currentQuery.toUpperCase().startsWith('SELECT *')) {
      const newQuery = currentQuery.replace(/SELECT \*/i, `SELECT ${columnName}`);
      this.updateCurrentTab({ queryText: newQuery });
    } else if (currentQuery.toUpperCase().startsWith('SELECT')) {
      const newQuery = currentQuery.replace(/SELECT /i, `SELECT ${columnName}, `);
      this.updateCurrentTab({ queryText: newQuery });
    }
  }

  clearResults(): void {
    this.updateCurrentTab({
      columns: [],
      rows: [],
      rowCount: 0,
      executionTimeMs: 0
    });
  }

  newQuery(): void {
    this.updateCurrentTab({
      queryText: 'SELECT * FROM issues LIMIT 10',
      editingQueryId: null,
      queryName: '',
      queryDescription: '',
      columns: [],
      rows: [],
      rowCount: 0,
      executionTimeMs: 0,
      error: null,
      name: `Query ${this.tabs().length + 1}`
    });
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
    this.updateCurrentTab({
      queryText: this.burndownSqlTemplate,
      editingQueryId: null,
      queryName: '',
      queryDescription: ''
    });
  }

  // ベロシティチャート用SQLをエディタに挿入
  insertVelocitySql(): void {
    this.updateCurrentTab({
      queryText: this.velocitySqlTemplate,
      editingQueryId: null,
      queryName: '',
      queryDescription: ''
    });
  }

  // 累積フロー図用SQLをエディタに挿入
  insertCfdSql(): void {
    this.updateCurrentTab({
      queryText: this.cfdSqlTemplate,
      editingQueryId: null,
      queryName: '',
      queryDescription: ''
    });
  }
}
