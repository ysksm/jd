import {
  Component,
  OnInit,
  OnDestroy,
  OnChanges,
  SimpleChanges,
  Input,
  signal,
  computed,
  ElementRef,
  ViewChild,
  AfterViewInit,
  inject,
} from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { SavedQuery } from '../../generated/models';
import { API_SERVICE, IApiService } from '../../api.provider';
import uPlot from 'uplot';
import * as echarts from 'echarts';

export type ChartType = 'line' | 'bar' | 'area' | 'scatter' | 'spline' | 'stepped' | 'stepped-area' | 'bubble' | 'heatmap';
export type AggregationType = 'count' | 'sum' | 'avg' | 'min' | 'max' | 'none';

interface ChartConfig {
  chartType: ChartType;
  xColumn: string;
  yColumn: string;
  zColumn: string; // For bubble size or heatmap value
  groupByColumn: string | null;
  aggregation: AggregationType;
  title: string;
  showLegend: boolean;
  showGrid: boolean;
  rotateLabels: boolean;
  xTickCount: number; // 0 = auto, otherwise specific count
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
export class VisualizationComponent implements OnInit, OnDestroy, AfterViewInit, OnChanges {
  @ViewChild('chartContainer') chartContainer!: ElementRef<HTMLDivElement>;

  // Input for project context (when used inside project detail)
  @Input() projectKey: string = '';

  // Query state
  savedQueries = signal<SavedQuery[]>([]);
  selectedQuery = signal<SavedQuery | null>(null);

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
    zColumn: '',
    groupByColumn: null,
    aggregation: 'count',
    title: '',
    showLegend: true,
    showGrid: true,
    rotateLabels: false,
    xTickCount: 0, // 0 = auto
  });

  // X-axis tick count options
  xTickOptions: { value: number; label: string }[] = [
    { value: 0, label: 'Auto' },
    { value: 5, label: '5' },
    { value: 10, label: '10' },
    { value: 15, label: '15' },
    { value: 20, label: '20' },
    { value: 30, label: '30' },
    { value: 50, label: '50' },
    { value: -1, label: 'All' },
  ];

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

  // Chart instances
  private uplotChart: uPlot | null = null;
  private echartsInstance: echarts.ECharts | null = null;

  // Computed
  hasData = computed(() => this.columns().length > 0 && this.rows().length > 0);
  numericColumns = computed(() =>
    this.columns().filter((col) => this.isNumericColumn(col))
  );
  categoricalColumns = computed(() =>
    this.columns().filter((col) => !this.isNumericColumn(col))
  );
  isEChartsType = computed(() => {
    const type = this.chartConfig().chartType;
    return type === 'bubble' || type === 'heatmap';
  });
  needsZColumn = computed(() => {
    const type = this.chartConfig().chartType;
    return type === 'bubble' || type === 'heatmap';
  });

  chartTypes: { value: ChartType; label: string; library: 'uplot' | 'echarts' }[] = [
    { value: 'bar', label: 'Bar Chart', library: 'uplot' },
    { value: 'line', label: 'Line Chart', library: 'uplot' },
    { value: 'spline', label: 'Spline (Smooth)', library: 'uplot' },
    { value: 'stepped', label: 'Stepped Line', library: 'uplot' },
    { value: 'area', label: 'Area Chart', library: 'uplot' },
    { value: 'stepped-area', label: 'Stepped Area', library: 'uplot' },
    { value: 'scatter', label: 'Scatter Plot', library: 'uplot' },
    { value: 'bubble', label: 'Bubble Chart', library: 'echarts' },
    { value: 'heatmap', label: 'Heatmap', library: 'echarts' },
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

  private api = inject<IApiService>(API_SERVICE);

  ngOnInit(): void {
    this.initializeComponent();
  }

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['projectKey'] && !changes['projectKey'].firstChange) {
      this.initializeComponent();
    }
  }

  ngAfterViewInit(): void {
    // Chart will be rendered after data is loaded
  }

  ngOnDestroy(): void {
    this.destroyChart();
  }

  private initializeComponent(): void {
    this.loadSavedQueries();
  }

  loadSavedQueries(): void {
    this.api.sqlListQueries({}).subscribe({
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
    this.executeQuery(query.query);
  }

  executeQuery(query: string): void {
    this.loading.set(true);
    this.error.set(null);
    this.successMessage.set(null);

    this.api.sqlExecute({ projectKey: this.projectKey || undefined, query, limit: 10000 }).subscribe({
      next: (response) => {
        this.columns.set(response.columns);
        this.rows.set(response.rows as Record<string, unknown>[]);
        this.filteredRows.set(response.rows as Record<string, unknown>[]);
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
    // Find numeric columns for Y and Z
    const numericCols = cols.filter((c) => this.isNumericColumn(c));
    const numericCol = numericCols[0];
    const zCol = numericCols[1] || '';

    this.chartConfig.set({
      ...config,
      xColumn: categoricalCol || cols[0],
      yColumn: numericCol || '',
      zColumn: zCol,
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

  // Chart Rendering - Main dispatcher
  renderChart(): void {
    this.destroyChart();

    const container = this.chartContainer?.nativeElement;
    if (!container) return;

    const config = this.chartConfig();
    const data = this.filteredRows();

    if (!config.xColumn || data.length === 0) return;

    if (this.isEChartsType()) {
      this.renderEChartsChart(container, data, config);
    } else {
      this.renderUPlotChart(container, data, config);
    }
  }

  // uPlot Chart Rendering
  private renderUPlotChart(
    container: HTMLDivElement,
    data: Record<string, unknown>[],
    config: ChartConfig
  ): void {
    const chartData = this.prepareChartData(data, config);
    if (!chartData) return;

    const { xData, yData, labels } = chartData;

    const width = container.clientWidth || 800;
    const height = 400;

    const seriesConfig = this.getSeriesConfig(config.chartType, config.yColumn);

    const opts: uPlot.Options = {
      title: config.title || undefined,
      width,
      height,
      scales: {
        x: {
          time: false,
        },
      },
      series: [
        {
          label: config.xColumn,
          value: (_u: uPlot, v: number) => labels[v] ?? String(v),
        },
        seriesConfig,
      ],
      axes: [
        {
          grid: { show: config.showGrid },
          values: (_u: uPlot, vals: number[]) => vals.map((v: number) => labels[v] || String(v)),
          rotate: config.rotateLabels ? -45 : 0,
          gap: config.rotateLabels ? 8 : 5,
          splits: this.getXAxisSplits(xData.length, config.xTickCount),
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
    };

    const uplotData: uPlot.AlignedData = [xData, yData];
    this.uplotChart = new uPlot(opts, uplotData, container);
  }

  // ECharts Rendering
  private renderEChartsChart(
    container: HTMLDivElement,
    data: Record<string, unknown>[],
    config: ChartConfig
  ): void {
    this.echartsInstance = echarts.init(container);

    if (config.chartType === 'bubble') {
      this.renderBubbleChart(data, config);
    } else if (config.chartType === 'heatmap') {
      this.renderHeatmapChart(data, config);
    }
  }

  private renderBubbleChart(data: Record<string, unknown>[], config: ChartConfig): void {
    if (!this.echartsInstance) return;

    // Prepare bubble data: [x, y, size, label]
    const bubbleData: [number | string, number, number, string][] = [];
    const xLabels = new Set<string>();

    for (const row of data) {
      const xVal = String(row[config.xColumn] ?? 'null');
      const yVal = Number(row[config.yColumn]) || 0;
      const zVal = config.zColumn ? Number(row[config.zColumn]) || 1 : 1;
      xLabels.add(xVal);
      bubbleData.push([xVal, yVal, zVal, xVal]);
    }

    const xLabelArray = Array.from(xLabels).sort();

    // Calculate max size for scaling
    const maxZ = Math.max(...bubbleData.map(d => d[2]));
    const minZ = Math.min(...bubbleData.map(d => d[2]));

    const option: echarts.EChartsOption = {
      title: {
        text: config.title || undefined,
        left: 'center',
      },
      tooltip: {
        trigger: 'item',
        formatter: (params: unknown) => {
          const p = params as { data: [string, number, number, string] };
          return `${p.data[3]}<br/>Y: ${p.data[1]}<br/>Size: ${p.data[2]}`;
        },
      },
      legend: {
        show: config.showLegend,
        top: 'bottom',
      },
      grid: {
        left: '10%',
        right: '10%',
        bottom: config.showLegend ? '15%' : '10%',
        top: config.title ? '15%' : '10%',
      },
      xAxis: {
        type: 'category',
        data: xLabelArray,
        axisLabel: {
          rotate: config.rotateLabels ? -45 : 0,
        },
        splitLine: {
          show: config.showGrid,
        },
      },
      yAxis: {
        type: 'value',
        splitLine: {
          show: config.showGrid,
        },
      },
      series: [
        {
          name: config.yColumn || 'Value',
          type: 'scatter',
          symbolSize: (val: number[]) => {
            // Scale bubble size between 10 and 60
            const normalized = maxZ === minZ ? 0.5 : (val[2] - minZ) / (maxZ - minZ);
            return 10 + normalized * 50;
          },
          data: bubbleData.map(d => [d[0], d[1], d[2]]),
          itemStyle: {
            color: 'rgba(74, 74, 224, 0.7)',
          },
          emphasis: {
            itemStyle: {
              color: 'rgba(74, 74, 224, 1)',
            },
          },
        },
      ],
    };

    this.echartsInstance.setOption(option);
  }

  private renderHeatmapChart(data: Record<string, unknown>[], config: ChartConfig): void {
    if (!this.echartsInstance) return;

    // Group data for heatmap: x-axis categories, y-axis categories, values
    const xLabels = new Set<string>();
    const yLabels = new Set<string>();
    const valueMap = new Map<string, number>();

    for (const row of data) {
      const xVal = String(row[config.xColumn] ?? 'null');
      const yVal = String(row[config.yColumn] ?? 'null');
      const zVal = config.zColumn ? Number(row[config.zColumn]) || 0 : 1;

      xLabels.add(xVal);
      yLabels.add(yVal);

      const key = `${xVal}|${yVal}`;
      valueMap.set(key, (valueMap.get(key) || 0) + zVal);
    }

    const xLabelArray = Array.from(xLabels).sort();
    const yLabelArray = Array.from(yLabels).sort();

    // Create heatmap data: [xIndex, yIndex, value]
    const heatmapData: [number, number, number][] = [];
    let maxVal = 0;
    let minVal = Infinity;

    for (let xi = 0; xi < xLabelArray.length; xi++) {
      for (let yi = 0; yi < yLabelArray.length; yi++) {
        const key = `${xLabelArray[xi]}|${yLabelArray[yi]}`;
        const val = valueMap.get(key) || 0;
        heatmapData.push([xi, yi, val]);
        if (val > maxVal) maxVal = val;
        if (val < minVal) minVal = val;
      }
    }

    const option: echarts.EChartsOption = {
      title: {
        text: config.title || undefined,
        left: 'center',
      },
      tooltip: {
        position: 'top',
        formatter: (params: unknown) => {
          const p = params as { data: [number, number, number] };
          return `${xLabelArray[p.data[0]]} / ${yLabelArray[p.data[1]]}<br/>Value: ${p.data[2]}`;
        },
      },
      grid: {
        left: '15%',
        right: '15%',
        bottom: config.rotateLabels ? '20%' : '10%',
        top: config.title ? '15%' : '10%',
      },
      xAxis: {
        type: 'category',
        data: xLabelArray,
        axisLabel: {
          rotate: config.rotateLabels ? -45 : 0,
        },
        splitArea: {
          show: true,
        },
      },
      yAxis: {
        type: 'category',
        data: yLabelArray,
        splitArea: {
          show: true,
        },
      },
      visualMap: {
        min: minVal,
        max: maxVal,
        calculable: true,
        orient: 'horizontal',
        left: 'center',
        bottom: '0%',
        inRange: {
          color: ['#f0f9ff', '#4a4ae0', '#1e1e8f'],
        },
      },
      series: [
        {
          name: config.zColumn || 'Value',
          type: 'heatmap',
          data: heatmapData,
          label: {
            show: heatmapData.length <= 100, // Only show labels for small datasets
          },
          emphasis: {
            itemStyle: {
              shadowBlur: 10,
              shadowColor: 'rgba(0, 0, 0, 0.5)',
            },
          },
        },
      ],
    };

    this.echartsInstance.setOption(option);
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
      case 'spline':
        return {
          ...baseConfig,
          paths: uPlot.paths.spline!(),
        };
      case 'stepped':
        return {
          ...baseConfig,
          paths: uPlot.paths.stepped!({ align: 1 }),
        };
      case 'stepped-area':
        return {
          ...baseConfig,
          fill: 'rgba(74, 74, 224, 0.2)',
          paths: uPlot.paths.stepped!({ align: 1 }),
        };
      case 'line':
      default:
        return baseConfig;
    }
  }

  private getXAxisSplits(dataLength: number, tickCount: number): number[] | undefined {
    if (tickCount === 0) {
      return undefined;
    }

    if (tickCount === -1 || tickCount >= dataLength) {
      return Array.from({ length: dataLength }, (_, i) => i);
    }

    const splits: number[] = [];
    const step = (dataLength - 1) / (tickCount - 1);
    for (let i = 0; i < tickCount; i++) {
      splits.push(Math.round(i * step));
    }
    return splits;
  }

  private prepareChartData(
    data: Record<string, unknown>[],
    config: ChartConfig
  ): { xData: number[]; yData: number[]; labels: Record<number, string> } | null {
    if (!config.xColumn) return null;

    const grouped = new Map<string, number[]>();

    for (const row of data) {
      const xVal = String(row[config.xColumn] ?? 'null');
      const yVal = config.yColumn ? Number(row[config.yColumn]) || 0 : 1;

      if (!grouped.has(xVal)) {
        grouped.set(xVal, []);
      }
      grouped.get(xVal)!.push(yVal);
    }

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

    aggregated.sort((a, b) => a.label.localeCompare(b.label));

    const xData = aggregated.map((_, i) => i);
    const yData = aggregated.map((d) => d.value);
    const labels: Record<number, string> = {};
    aggregated.forEach((d, i) => {
      labels[i] = d.label;
    });

    return { xData, yData, labels };
  }

  private destroyChart(): void {
    if (this.uplotChart) {
      this.uplotChart.destroy();
      this.uplotChart = null;
    }
    if (this.echartsInstance) {
      this.echartsInstance.dispose();
      this.echartsInstance = null;
    }
  }

  onChartClick(event: MouseEvent): void {
    // Implement drill-down on chart click
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
    }
  }
}
