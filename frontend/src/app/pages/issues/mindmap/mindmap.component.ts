import {
  Component,
  Input,
  Output,
  EventEmitter,
  OnChanges,
  OnDestroy,
  SimpleChanges,
  ElementRef,
  ViewChild,
  AfterViewInit,
  signal,
  computed,
} from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Issue } from '../../../generated/models';
import * as echarts from 'echarts';

// Tree node structure compatible with ECharts tree series
interface TreeNodeData {
  name: string;
  issueData?: Issue;
  children?: TreeNodeData[];
  collapsed?: boolean;
  itemStyle?: {
    color: string;
    borderColor: string;
  };
}

// Status color mapping
const STATUS_COLORS: Record<string, { bg: string; border: string }> = {
  // To Do / Open
  'open': { bg: '#FEF3C7', border: '#F59E0B' },
  'to do': { bg: '#FEF3C7', border: '#F59E0B' },
  'backlog': { bg: '#F3F4F6', border: '#9CA3AF' },
  'オープン': { bg: '#FEF3C7', border: '#F59E0B' },
  // In Progress
  'in progress': { bg: '#DBEAFE', border: '#3B82F6' },
  '進行中': { bg: '#DBEAFE', border: '#3B82F6' },
  'in review': { bg: '#E0E7FF', border: '#6366F1' },
  // Done
  'done': { bg: '#D1FAE5', border: '#10B981' },
  '完了': { bg: '#D1FAE5', border: '#10B981' },
  'closed': { bg: '#D1FAE5', border: '#10B981' },
  'resolved': { bg: '#D1FAE5', border: '#10B981' },
};

// Issue type color mapping
const ISSUE_TYPE_COLORS: Record<string, { bg: string; border: string }> = {
  'epic': { bg: '#FAE8FF', border: '#A855F7' },
  'エピック': { bg: '#FAE8FF', border: '#A855F7' },
  'story': { bg: '#DCFCE7', border: '#22C55E' },
  'ストーリー': { bg: '#DCFCE7', border: '#22C55E' },
  'task': { bg: '#DBEAFE', border: '#3B82F6' },
  'タスク': { bg: '#DBEAFE', border: '#3B82F6' },
  'bug': { bg: '#FEE2E2', border: '#EF4444' },
  'バグ': { bg: '#FEE2E2', border: '#EF4444' },
  'sub-task': { bg: '#E0E7FF', border: '#6366F1' },
  'subtask': { bg: '#E0E7FF', border: '#6366F1' },
  'サブタスク': { bg: '#E0E7FF', border: '#6366F1' },
};

type ColorMode = 'status' | 'issueType';

@Component({
  selector: 'app-mindmap',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './mindmap.component.html',
  styleUrl: './mindmap.component.scss',
})
export class MindmapComponent implements OnChanges, OnDestroy, AfterViewInit {
  @ViewChild('chartContainer') chartContainer!: ElementRef<HTMLDivElement>;

  @Input() issues: Issue[] = [];
  @Output() issueClick = new EventEmitter<Issue>();

  private echartsInstance: echarts.ECharts | null = null;
  private resizeObserver: ResizeObserver | null = null;

  // UI state
  colorMode = signal<ColorMode>('status');
  zoomLevel = signal(100);
  expandAll = signal(true);

  // Tree data
  treeData = computed(() => this.buildTree(this.issues));

  // Statistics
  stats = computed(() => {
    const issues = this.issues;
    const roots = issues.filter(i => !i.parentKey || !this.issueMap.has(i.parentKey));
    const withChildren = issues.filter(i =>
      issues.some(child => child.parentKey === i.key)
    );
    return {
      total: issues.length,
      roots: roots.length,
      withChildren: withChildren.length,
      maxDepth: this.calculateMaxDepth(this.treeData() as TreeNodeData[]),
    };
  });

  private issueMap = new Map<string, Issue>();

  ngAfterViewInit(): void {
    this.initChart();
    this.setupResizeObserver();
  }

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['issues'] && this.echartsInstance) {
      this.updateChart();
    }
  }

  ngOnDestroy(): void {
    this.destroyChart();
    if (this.resizeObserver) {
      this.resizeObserver.disconnect();
    }
  }

  private setupResizeObserver(): void {
    if (this.chartContainer?.nativeElement) {
      this.resizeObserver = new ResizeObserver(() => {
        if (this.echartsInstance) {
          this.echartsInstance.resize();
        }
      });
      this.resizeObserver.observe(this.chartContainer.nativeElement);
    }
  }

  private initChart(): void {
    if (!this.chartContainer?.nativeElement) return;

    this.echartsInstance = echarts.init(this.chartContainer.nativeElement);
    this.updateChart();

    // Handle click events
    this.echartsInstance.on('click', (params: echarts.ECElementEvent) => {
      const data = params.data as TreeNodeData | undefined;
      if (data?.issueData) {
        this.issueClick.emit(data.issueData);
      }
    });
  }

  private updateChart(): void {
    if (!this.echartsInstance) return;

    const treeData = this.treeData() as TreeNodeData[];
    if (treeData.length === 0) {
      this.echartsInstance.clear();
      return;
    }

    // Create a root node if there are multiple trees
    const rootData: TreeNodeData = treeData.length === 1
      ? treeData[0]
      : {
          name: 'Issues',
          children: treeData,
          itemStyle: { color: '#F3F4F6', borderColor: '#9CA3AF' },
        };

    const option: echarts.EChartsOption = {
      tooltip: {
        trigger: 'item',
        formatter: (params: unknown) => {
          const data = (params as { data: TreeNodeData }).data;
          if (!data.issueData) return String(data.name);
          const issue = data.issueData;
          return `
            <div style="max-width: 300px;">
              <strong>${issue.key}</strong><br/>
              <span style="color: #666;">${issue.issueType}</span> ·
              <span style="color: #666;">${issue.status}</span><br/>
              ${issue.summary}<br/>
              ${issue.assignee ? `<small>Assignee: ${issue.assignee}</small>` : ''}
            </div>
          `;
        },
      },
      series: [
        {
          type: 'tree',
          data: [rootData],
          orient: 'LR', // Left to Right
          layout: 'orthogonal',
          symbol: 'roundRect',
          symbolSize: [120, 50],
          initialTreeDepth: this.expandAll() ? -1 : 2,
          roam: true, // Enable zoom and pan
          zoom: this.zoomLevel() / 100,
          label: {
            position: 'inside',
            verticalAlign: 'middle',
            align: 'center',
            fontSize: 11,
            color: '#333',
            formatter: (params: unknown) => {
              const data = (params as { data: TreeNodeData }).data;
              if (!data.issueData) return String(data.name);
              const issue = data.issueData;
              const summary = issue.summary.length > 25
                ? issue.summary.substring(0, 22) + '...'
                : issue.summary;
              return `{key|${issue.key}}\n{summary|${summary}}`;
            },
            rich: {
              key: {
                fontSize: 11,
                fontWeight: 'bold',
                color: '#4A4AE0',
                lineHeight: 16,
              },
              summary: {
                fontSize: 10,
                color: '#666',
                lineHeight: 14,
              },
            },
          },
          leaves: {
            label: {
              position: 'inside',
              align: 'center',
            },
          },
          expandAndCollapse: true,
          animationDuration: 550,
          animationDurationUpdate: 750,
          lineStyle: {
            width: 2,
            color: '#ccc',
            curveness: 0.5,
          },
          emphasis: {
            focus: 'descendant',
            itemStyle: {
              borderWidth: 3,
            },
          },
        },
      ],
    };

    this.echartsInstance.setOption(option, true);
  }

  private buildTree(issues: Issue[]): TreeNodeData[] {
    // Build issue map
    this.issueMap.clear();
    issues.forEach(issue => this.issueMap.set(issue.key, issue));

    // Build children map
    const childrenMap = new Map<string, Issue[]>();
    issues.forEach(issue => {
      if (issue.parentKey) {
        const children = childrenMap.get(issue.parentKey) || [];
        children.push(issue);
        childrenMap.set(issue.parentKey, children);
      }
    });

    // Find root nodes (no parent or parent not in dataset)
    const roots = issues.filter(
      issue => !issue.parentKey || !this.issueMap.has(issue.parentKey)
    );

    // Build tree recursively
    const buildNode = (issue: Issue): TreeNodeData => {
      const children = childrenMap.get(issue.key) || [];
      const colors = this.getNodeColors(issue);

      return {
        name: issue.key,
        issueData: issue,
        children: children.map(buildNode),
        collapsed: !this.expandAll(),
        itemStyle: {
          color: colors.bg,
          borderColor: colors.border,
        },
      };
    };

    return roots.map(buildNode);
  }

  private getNodeColors(issue: Issue): { bg: string; border: string } {
    if (this.colorMode() === 'status') {
      const statusKey = issue.status.toLowerCase();
      return STATUS_COLORS[statusKey] || { bg: '#F3F4F6', border: '#9CA3AF' };
    } else {
      const typeKey = issue.issueType.toLowerCase();
      return ISSUE_TYPE_COLORS[typeKey] || { bg: '#F3F4F6', border: '#9CA3AF' };
    }
  }

  private calculateMaxDepth(nodes: TreeNodeData[]): number {
    if (nodes.length === 0) return 0;

    const getDepth = (node: TreeNodeData, depth: number): number => {
      if (!node.children || node.children.length === 0) return depth;
      return Math.max(...node.children.map(child => getDepth(child, depth + 1)));
    };

    return Math.max(...nodes.map(node => getDepth(node, 1)));
  }

  private destroyChart(): void {
    if (this.echartsInstance) {
      this.echartsInstance.dispose();
      this.echartsInstance = null;
    }
  }

  // Public methods for UI controls
  setColorMode(mode: ColorMode): void {
    this.colorMode.set(mode);
    this.updateChart();
  }

  zoomIn(): void {
    const current = this.zoomLevel();
    if (current < 200) {
      this.zoomLevel.set(current + 20);
      this.updateChart();
    }
  }

  zoomOut(): void {
    const current = this.zoomLevel();
    if (current > 40) {
      this.zoomLevel.set(current - 20);
      this.updateChart();
    }
  }

  resetZoom(): void {
    this.zoomLevel.set(100);
    this.updateChart();
  }

  toggleExpand(): void {
    this.expandAll.set(!this.expandAll());
    this.updateChart();
  }

  fitToScreen(): void {
    if (this.echartsInstance) {
      this.echartsInstance.resize();
      this.resetZoom();
    }
  }
}
