import {
  Component,
  OnInit,
  OnDestroy,
  signal,
  computed,
  ElementRef,
  ViewChild,
  AfterViewInit,
} from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { TauriApiService } from '../../generated/tauri-api.service';
import { SavedQuery } from '../../generated/models';
import uPlot from 'uplot';

export type ChartType = 'line' | 'bar' | 'area' | 'scatter';
export type AggregationType = 'count' | 'sum' | 'avg' | 'min' | 'max' | 'none';

interface ChartConfig {
  chartType: ChartType;
  xColumn: string;
  yColumn: string;
  groupByColumn: string | null;
  aggregation: AggregationType;
  title: string;
  showLegend: boolean;
  showGrid: boolean;
  rotateLabels: boolean;
}

interface FilterConfig {
  column: string;
  operator: 'eq' | 'ne' | 'gt' | 'lt' | 'gte' | 'lte' | 'contains';
  value: string;
}

@Component({
  selector: 'app-visualization',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './visualization.component.html',
  styleUrl: './visualization.component.scss',
})
export class VisualizationComponent implements OnInit, OnDestroy, AfterViewInit {
  @ViewChild('chartContainer') chartContainer!: ElementRef<HTMLDivElement>;

  // Query state
  savedQueries = signal<SavedQuery[]>([]);
  selectedQuery = signal<SavedQuery | null>(null);
  customQuery = signal('');
  useCustomQuery = signal(false);

  // Data state
  columns = signal<string[]>([]);
  rows = signal<Record<string, unknown>[]>([]);
  filteredRows = signal<Record<string, unknown>[]>([]);
  rowCount = signal(0);
  executionTimeMs = signal(0);

  // Chart configuration
  chartConfig = signal<ChartConfig>({
    chartType: 'bar',
    xColumn: '',
    yColumn: '',
    groupByColumn: null,
    aggregation: 'count',
    title: '',
    showLegend: true,
    showGrid: true,
    rotateLabels: false,
  });

  // Filters
  filters = signal<FilterConfig[]>([]);
  drillDownStack = signal<{ column: string; value: string }[]>([]);

  // UI state
  loading = signal(false);
  error = signal<string | null>(null);
  successMessage = signal<string | null>(null);
  showConfigPanel = signal(true);
  activeTab = signal<'chart' | 'data'>('chart');
  splitView = signal(true);

  // uPlot instance
  private chart: uPlot | null = null;

  // Computed
  hasData = computed(() => this.columns().length > 0 && this.rows().length > 0);
  numericColumns = computed(() =>
    this.columns().filter((col) => this.isNumericColumn(col))
  );
  categoricalColumns = computed(() =>
    this.columns().filter((col) => !this.isNumericColumn(col))
  );

  chartTypes: { value: ChartType; label: string }[] = [
    { value: 'bar', label: 'Bar Chart' },
    { value: 'line', label: 'Line Chart' },
    { value: 'area', label: 'Area Chart' },
    { value: 'scatter', label: 'Scatter Plot' },
  ];

  aggregationTypes: { value: AggregationType; label: string }[] = [
    { value: 'count', label: 'Count' },
    { value: 'sum', label: 'Sum' },
    { value: 'avg', label: 'Average' },
    { value: 'min', label: 'Minimum' },
    { value: 'max', label: 'Maximum' },
    { value: 'none', label: 'No Aggregation' },
  ];

  filterOperators: { value: FilterConfig['operator']; label: string }[] = [
    { value: 'eq', label: '=' },
    { value: 'ne', label: '!=' },
    { value: 'gt', label: '>' },
    { value: 'lt', label: '<' },
    { value: 'gte', label: '>=' },
    { value: 'lte', label: '<=' },
    { value: 'contains', label: 'Contains' },
  ];

  constructor(private api: TauriApiService) {}

  ngOnInit(): void {
    this.loadSavedQueries();
  }

  ngAfterViewInit(): void {
    // Chart will be rendered after data is loaded
  }

  ngOnDestroy(): void {
    this.destroyChart();
  }

  loadSavedQueries(): void {
    this.api.sqlQueryList({}).subscribe({
      next: (response) => {
        this.savedQueries.set(response.queries);
      },
      error: (err) => {
        console.error('Failed to load saved queries:', err);
      },
    });
  }

  selectQuery(query: SavedQuery): void {
    this.selectedQuery.set(query);
    this.useCustomQuery.set(false);
    this.executeQuery(query.query);
  }

  executeCustomQuery(): void {
    const query = this.customQuery();
    if (!query.trim()) {
      this.error.set('Please enter a query');
      return;
    }
    this.selectedQuery.set(null);
    this.useCustomQuery.set(true);
    this.executeQuery(query);
  }

  executeQuery(query: string): void {
    this.loading.set(true);
    this.error.set(null);
    this.successMessage.set(null);

    this.api.sqlExecute({ query, limit: 10000 }).subscribe({
      next: (response) => {
        this.columns.set(response.columns);
        this.rows.set(response.rows);
        this.filteredRows.set(response.rows);
        this.rowCount.set(response.rowCount);
        this.executionTimeMs.set(response.executionTimeMs);
        this.loading.set(false);

        // Auto-configure chart if columns available
        this.autoConfigureChart();

        // Reset filters and drill-down
        this.filters.set([]);
        this.drillDownStack.set([]);

        this.successMessage.set(`Loaded ${response.rowCount} rows in ${response.executionTimeMs.toFixed(2)}ms`);
        setTimeout(() => this.successMessage.set(null), 3000);
      },
      error: (err) => {
        this.error.set('Query execution failed: ' + err);
        this.loading.set(false);
      },
    });
  }

  private autoConfigureChart(): void {
    const cols = this.columns();
    if (cols.length === 0) return;

    const config = this.chartConfig();

    // Find first categorical column for X
    const categoricalCol = cols.find((c) => !this.isNumericColumn(c));
    // Find first numeric column for Y
    const numericCol = cols.find((c) => this.isNumericColumn(c));

    this.chartConfig.set({
      ...config,
      xColumn: categoricalCol || cols[0],
      yColumn: numericCol || '',
      aggregation: numericCol ? 'sum' : 'count',
    });

    // Render chart after configuration
    setTimeout(() => this.renderChart(), 100);
  }

  private isNumericColumn(colName: string): boolean {
    const rows = this.rows();
    if (rows.length === 0) return false;

    // Check first few non-null values
    for (const row of rows.slice(0, 10)) {
      const val = row[colName];
      if (val !== null && val !== undefined) {
        return typeof val === 'number';
      }
    }
    return false;
  }

  updateChartConfig(updates: Partial<ChartConfig>): void {
    this.chartConfig.set({ ...this.chartConfig(), ...updates });
    this.renderChart();
  }

  // Filter Management
  addFilter(): void {
    const cols = this.columns();
    if (cols.length === 0) return;

    const currentFilters = this.filters();
    this.filters.set([
      ...currentFilters,
      { column: cols[0], operator: 'eq', value: '' },
    ]);
  }

  updateFilter(index: number, updates: Partial<FilterConfig>): void {
    const currentFilters = [...this.filters()];
    currentFilters[index] = { ...currentFilters[index], ...updates };
    this.filters.set(currentFilters);
    this.applyFilters();
  }

  removeFilter(index: number): void {
    const currentFilters = [...this.filters()];
    currentFilters.splice(index, 1);
    this.filters.set(currentFilters);
    this.applyFilters();
  }

  applyFilters(): void {
    let data = this.rows();

    // Apply drill-down filters
    for (const drill of this.drillDownStack()) {
      data = data.filter((row) => String(row[drill.column]) === drill.value);
    }

    // Apply user filters
    for (const filter of this.filters()) {
      if (!filter.value && filter.operator !== 'eq' && filter.operator !== 'ne') continue;

      data = data.filter((row) => {
        const val = row[filter.column];
        const filterVal = filter.value;

        switch (filter.operator) {
          case 'eq':
            return String(val) === filterVal;
          case 'ne':
            return String(val) !== filterVal;
          case 'gt':
            return Number(val) > Number(filterVal);
          case 'lt':
            return Number(val) < Number(filterVal);
          case 'gte':
            return Number(val) >= Number(filterVal);
          case 'lte':
            return Number(val) <= Number(filterVal);
          case 'contains':
            return String(val).toLowerCase().includes(filterVal.toLowerCase());
          default:
            return true;
        }
      });
    }

    this.filteredRows.set(data);
    this.renderChart();
  }

  clearFilters(): void {
    this.filters.set([]);
    this.drillDownStack.set([]);
    this.filteredRows.set(this.rows());
    this.renderChart();
  }

  // Drill-down
  drillDown(column: string, value: string): void {
    const stack = this.drillDownStack();
    this.drillDownStack.set([...stack, { column, value }]);
    this.applyFilters();
  }

  drillUp(): void {
    const stack = [...this.drillDownStack()];
    stack.pop();
    this.drillDownStack.set(stack);
    this.applyFilters();
  }

  // Chart Rendering
  renderChart(): void {
    this.destroyChart();

    const container = this.chartContainer?.nativeElement;
    if (!container) return;

    const data = this.filteredRows();
    const config = this.chartConfig();

    if (!config.xColumn || data.length === 0) return;

    // Prepare data based on chart type and aggregation
    const chartData = this.prepareChartData(data, config);
    if (!chartData) return;

    const { xData, yData, labels } = chartData;

    // Create uPlot configuration
    const width = container.clientWidth || 800;
    const height = 400;

    const seriesConfig = this.getSeriesConfig(config.chartType, config.yColumn);

    const opts: uPlot.Options = {
      title: config.title || undefined,
      width,
      height,
      scales: {
        x: {
          time: false, // Disable time-based X-axis
        },
      },
      series: [
        {
          label: config.xColumn, // X-axis label
          value: (_u: uPlot, v: number) => labels[v] ?? String(v), // Format X values in legend
        },
        seriesConfig,
      ],
      axes: [
        {
          grid: { show: config.showGrid },
          values: (_u: uPlot, vals: number[]) => vals.map((v: number) => labels[v] || String(v)),
          rotate: config.rotateLabels ? -45 : 0, // Rotate labels diagonally
          gap: config.rotateLabels ? 8 : 5, // More gap when rotated
        },
        {
          grid: { show: config.showGrid },
        },
      ],
      legend: {
        show: config.showLegend,
      },
      cursor: {
        drag: { x: true, y: true },
      },
      hooks: {
        setCursor: [
          (u: uPlot) => {
            const idx = u.cursor.idx;
            if (idx !== null && idx !== undefined) {
              // Could be used for tooltips
            }
          },
        ],
      },
    };

    const uplotData: uPlot.AlignedData = [xData, yData];

    this.chart = new uPlot(opts, uplotData, container);
  }

  private getSeriesConfig(chartType: ChartType, yColumn: string): uPlot.Series {
    const baseConfig: uPlot.Series = {
      label: yColumn || 'Count',
      stroke: '#4a4ae0',
      width: 2,
    };

    switch (chartType) {
      case 'bar':
        return {
          ...baseConfig,
          fill: 'rgba(74, 74, 224, 0.8)',
          paths: uPlot.paths.bars!({ size: [0.6, 100] }),
        };
      case 'scatter':
        return {
          ...baseConfig,
          paths: () => null,
          points: {
            show: true,
            size: 10,
            fill: '#4a4ae0',
          },
        };
      case 'area':
        return {
          ...baseConfig,
          fill: 'rgba(74, 74, 224, 0.2)',
        };
      case 'line':
      default:
        return baseConfig;
    }
  }

  private prepareChartData(
    data: Record<string, unknown>[],
    config: ChartConfig
  ): { xData: number[]; yData: number[]; labels: Record<number, string> } | null {
    if (!config.xColumn) return null;

    // Group and aggregate data
    const grouped = new Map<string, number[]>();

    for (const row of data) {
      const xVal = String(row[config.xColumn] ?? 'null');
      const yVal = config.yColumn ? Number(row[config.yColumn]) || 0 : 1;

      if (!grouped.has(xVal)) {
        grouped.set(xVal, []);
      }
      grouped.get(xVal)!.push(yVal);
    }

    // Apply aggregation
    const aggregated: { label: string; value: number }[] = [];
    for (const [label, values] of grouped) {
      let value: number;
      switch (config.aggregation) {
        case 'count':
          value = values.length;
          break;
        case 'sum':
          value = values.reduce((a, b) => a + b, 0);
          break;
        case 'avg':
          value = values.reduce((a, b) => a + b, 0) / values.length;
          break;
        case 'min':
          value = Math.min(...values);
          break;
        case 'max':
          value = Math.max(...values);
          break;
        case 'none':
        default:
          value = values[0] ?? 0;
      }
      aggregated.push({ label, value });
    }

    // Sort by label
    aggregated.sort((a, b) => a.label.localeCompare(b.label));

    // Create arrays for uPlot
    const xData = aggregated.map((_, i) => i);
    const yData = aggregated.map((d) => d.value);
    const labels: Record<number, string> = {};
    aggregated.forEach((d, i) => {
      labels[i] = d.label;
    });

    return { xData, yData, labels };
  }

  private destroyChart(): void {
    if (this.chart) {
      this.chart.destroy();
      this.chart = null;
    }
  }

  // Handle chart click for drill-down
  onChartClick(event: MouseEvent): void {
    // Implement drill-down on chart click
    // This would require more sophisticated hit-testing
  }

  // Utility
  formatValue(value: unknown): string {
    if (value === null || value === undefined) {
      return 'NULL';
    }
    if (typeof value === 'object') {
      return JSON.stringify(value);
    }
    return String(value);
  }

  onRowClick(row: Record<string, unknown>): void {
    const config = this.chartConfig();
    if (config.xColumn) {
      const value = String(row[config.xColumn]);
      this.drillDown(config.xColumn, value);
    }
  }

  toggleSplitView(): void {
    this.splitView.set(!this.splitView());
    setTimeout(() => this.renderChart(), 100);
  }

  toggleConfigPanel(): void {
    this.showConfigPanel.set(!this.showConfigPanel());
    setTimeout(() => this.renderChart(), 100);
  }

  refreshChart(): void {
    if (this.selectedQuery()) {
      this.executeQuery(this.selectedQuery()!.query);
    } else if (this.useCustomQuery() && this.customQuery()) {
      this.executeQuery(this.customQuery());
    }
  }
}
