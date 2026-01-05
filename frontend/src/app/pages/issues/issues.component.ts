import { Component, OnInit, OnChanges, SimpleChanges, Input, signal, inject, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Issue, Project, Status, IssueType } from '../../generated/models';
import { API_SERVICE, IApiService } from '../../api.provider';
import { MindmapComponent } from './mindmap/mindmap.component';
import { environment } from '../../../environments/environment';
import { openUrl } from '@tauri-apps/plugin-opener';

type ViewMode = 'list' | 'board' | 'mindmap' | 'calendar';
type GroupBy = 'none' | 'assignee' | 'epic';

interface StatusColumn {
  status: string;
  category: string;
  issues: Issue[];
}

interface Swimlane {
  name: string;
  issueCount: number;
  columns: StatusColumn[];
}

@Component({
  selector: 'app-issues',
  standalone: true,
  imports: [CommonModule, FormsModule, MindmapComponent],
  templateUrl: './issues.component.html',
  styleUrl: './issues.component.scss'
})
export class IssuesComponent implements OnInit, OnChanges {
  private api = inject<IApiService>(API_SERVICE);

  // Input for project context (when used inside project detail)
  @Input() projectKey: string = '';

  issues = signal<Issue[]>([]);
  selectedIssue = signal<Issue | null>(null);
  loading = signal(false);
  error = signal<string | null>(null);
  total = signal(0);

  // View mode
  viewMode = signal<ViewMode>('list');
  groupBy = signal<GroupBy>('none');

  // Collapsed swimlanes
  collapsedSwimlanes = signal<Set<string>>(new Set());

  // Filter options
  projects = signal<Project[]>([]);
  statuses = signal<Status[]>([]);
  issueTypes = signal<IssueType[]>([]);
  assignees = signal<string[]>([]);
  epics = signal<string[]>([]);

  // Search filters
  searchQuery = signal('');
  projectFilter = signal('');
  statusFilter = signal('');
  issueTypeFilter = signal('');
  assigneeFilter = signal('');

  // JIRA endpoint for external links
  jiraEndpoint = signal('');

  // Check if project is fixed from parent
  get hasFixedProject(): boolean {
    return !!this.projectKey;
  }

  ngOnInit(): void {
    this.initializeComponent();
  }

  ngOnChanges(changes: SimpleChanges): void {
    if (changes['projectKey'] && !changes['projectKey'].firstChange) {
      this.initializeComponent();
    }
  }

  private initializeComponent(): void {
    if (this.projectKey) {
      // Fixed project context - load only this project's data
      this.projectFilter.set(this.projectKey);
      this.loadProjectMetadata(this.projectKey);
    } else {
      // Global context - load all projects
      this.loadProjects();
    }
    this.loadJiraEndpoint();
    this.search();
  }

  loadJiraEndpoint(): void {
    this.api.configGet({}).subscribe({
      next: (response) => {
        this.jiraEndpoint.set(response.settings.jira.endpoint);
      },
      error: (err) => {
        console.error('Failed to load JIRA endpoint:', err);
      }
    });
  }

  getJiraUrl(issueKey: string): string {
    const endpoint = this.jiraEndpoint();
    if (!endpoint) {
      return '#';
    }
    // Remove trailing slash if present
    const baseUrl = endpoint.replace(/\/$/, '');
    return `${baseUrl}/browse/${issueKey}`;
  }

  openJiraLink(event: Event, issueKey: string): void {
    event.stopPropagation();
    const url = this.getJiraUrl(issueKey);
    if (url !== '#') {
      if (environment.apiMode === 'tauri') {
        // Use Tauri opener plugin for desktop app
        openUrl(url).catch(err => {
          console.error('Failed to open URL:', err);
        });
      } else {
        // Use window.open for web mode
        window.open(url, '_blank', 'noopener,noreferrer');
      }
    }
  }

  loadProjects(): void {
    this.api.projectsList({}).subscribe({
      next: (response) => {
        const enabledProjects = response.projects.filter(p => p.enabled);
        this.projects.set(enabledProjects);
        // Load metadata after projects are loaded
        this.loadAllMetadata();
      },
      error: (err) => {
        console.error('Failed to load projects:', err);
      }
    });
  }

  loadProjectMetadata(projectKey: string): void {
    this.api.metadataGet({ projectKey }).subscribe({
      next: (response) => {
        this.statuses.set(response.metadata.statuses);
        this.issueTypes.set(response.metadata.issueTypes);
      },
      error: (err) => {
        console.error('Failed to load metadata:', err);
      }
    });
  }

  loadAllMetadata(): void {
    // Aggregate metadata from all enabled projects
    const allStatuses: Status[] = [];
    const allIssueTypes: IssueType[] = [];
    const statusSet = new Set<string>();
    const typeSet = new Set<string>();

    const enabledProjects = this.projects().filter(p => p.enabled);
    let pending = enabledProjects.length;

    if (pending === 0) {
      return;
    }

    enabledProjects.forEach(project => {
      this.api.metadataGet({ projectKey: project.key }).subscribe({
        next: (response) => {
          response.metadata.statuses.forEach(s => {
            if (!statusSet.has(s.name)) {
              statusSet.add(s.name);
              allStatuses.push(s);
            }
          });
          response.metadata.issueTypes.forEach(t => {
            if (!typeSet.has(t.name)) {
              typeSet.add(t.name);
              allIssueTypes.push(t);
            }
          });
          pending--;
          if (pending === 0) {
            this.statuses.set(allStatuses);
            this.issueTypes.set(allIssueTypes);
          }
        },
        error: () => {
          pending--;
        }
      });
    });
  }

  extractAssignees(): void {
    const assigneeSet = new Set<string>();
    this.issues().forEach(issue => {
      if (issue.assignee) {
        assigneeSet.add(issue.assignee);
      }
    });
    this.assignees.set(Array.from(assigneeSet).sort());
  }

  extractEpics(): void {
    const epicSet = new Set<string>();
    this.issues().forEach(issue => {
      if (issue.parentKey) {
        epicSet.add(issue.parentKey);
      }
    });
    this.epics.set(Array.from(epicSet).sort());
  }

  // Category order for workflow sorting
  private readonly categoryOrder: Record<string, number> = {
    'new': 0,
    'to do': 0,
    'indeterminate': 1,
    'in progress': 1,
    'done': 2,
  };

  // Get status names for board header (sorted by workflow order)
  statusNames = computed<string[]>(() => {
    const statusList = this.statuses();
    const issues = this.issues();
    if (statusList.length > 0) {
      // Sort by category order (To Do -> In Progress -> Done)
      const sorted = [...statusList].sort((a, b) => {
        const orderA = this.categoryOrder[a.category.toLowerCase()] ?? 1;
        const orderB = this.categoryOrder[b.category.toLowerCase()] ?? 1;
        return orderA - orderB;
      });
      return sorted.map(s => s.name);
    }
    return [...new Set(issues.map(i => i.status))];
  });

  // Build a map of issue keys to their issue data for Epic resolution
  private issueMap = computed<Map<string, Issue>>(() => {
    const map = new Map<string, Issue>();
    this.issues().forEach(issue => map.set(issue.key, issue));
    return map;
  });

  // Helper to check if an issue type is Epic (supports multiple languages)
  private isEpicType(issueType: string): boolean {
    const normalized = issueType.toLowerCase();
    return normalized === 'epic' || normalized === 'エピック';
  }

  // Build a set of Epic keys for quick lookup
  private epicKeys = computed<Set<string>>(() => {
    const epics = new Set<string>();
    this.issues().forEach(issue => {
      if (this.isEpicType(issue.issueType)) {
        epics.add(issue.key);
      }
    });
    return epics;
  });

  // Find the Epic key for an issue (traverses parent chain)
  private findEpicForIssue(issue: Issue): string | null {
    const issueMap = this.issueMap();
    const epicKeys = this.epicKeys();

    // If the issue itself is an Epic, return null (Epics don't belong to other Epics)
    if (this.isEpicType(issue.issueType)) {
      return null;
    }

    // Check if the direct parent is an Epic
    if (issue.parentKey && epicKeys.has(issue.parentKey)) {
      return issue.parentKey;
    }

    // If parent exists but is not an Epic, check parent's parent (for sub-tasks)
    if (issue.parentKey) {
      const parent = issueMap.get(issue.parentKey);
      if (parent && parent.parentKey && epicKeys.has(parent.parentKey)) {
        return parent.parentKey;
      }
    }

    return null;
  }

  // Swimlane-based board view
  swimlanes = computed<Swimlane[]>(() => {
    const statusList = this.statuses();
    const issues = this.issues();
    const groupByValue = this.groupBy();

    // Create a map of status to category
    const statusCategoryMap = new Map<string, string>();
    statusList.forEach(s => statusCategoryMap.set(s.name, s.category));

    // Get status names
    const statusNames = this.statusNames();

    // If no grouping, return single swimlane with all issues
    if (groupByValue === 'none') {
      const columns = statusNames.map(status => ({
        status,
        category: statusCategoryMap.get(status) || 'default',
        issues: issues.filter(i => i.status === status)
      }));
      return [{ name: '', issueCount: issues.length, columns }];
    }

    // Group issues by assignee or epic
    const groupMap = new Map<string, Issue[]>();
    const defaultGroup = groupByValue === 'assignee' ? 'Unassigned' : 'No Epic';
    groupMap.set(defaultGroup, []);

    const issueMap = this.issueMap();
    const epicKeys = this.epicKeys();

    issues.forEach(issue => {
      let key: string;
      let shouldAddIssue = true;

      if (groupByValue === 'assignee') {
        key = issue.assignee || 'Unassigned';
      } else {
        // Epic grouping - find the Epic this issue belongs to
        if (this.isEpicType(issue.issueType)) {
          // Epics themselves are group headers - use their key as the group name
          // but we'll show their summary for better UX
          const epicSummary = `${issue.key}: ${issue.summary}`;
          key = epicSummary;
          // Don't add Epic itself to the issue list (it's already the swimlane header)
          shouldAddIssue = false;
        } else {
          const epicKey = this.findEpicForIssue(issue);
          if (epicKey) {
            // Use Epic's key and summary for the group name
            const epic = issueMap.get(epicKey);
            key = epic ? `${epic.key}: ${epic.summary}` : epicKey;
          } else {
            key = 'No Epic';
          }
        }
      }
      if (!groupMap.has(key)) {
        groupMap.set(key, []);
      }
      if (shouldAddIssue) {
        groupMap.get(key)!.push(issue);
      }
    });

    // Convert to swimlanes
    return Array.from(groupMap.entries())
      .filter(([, groupIssues]) => groupIssues.length > 0)
      .sort((a, b) => {
        if (a[0] === defaultGroup) return 1;
        if (b[0] === defaultGroup) return -1;
        return a[0].localeCompare(b[0]);
      })
      .map(([name, groupIssues]) => {
        const columns = statusNames.map(status => ({
          status,
          category: statusCategoryMap.get(status) || 'default',
          issues: groupIssues.filter(i => i.status === status)
        }));
        return { name, issueCount: groupIssues.length, columns };
      });
  });

  setViewMode(mode: ViewMode): void {
    this.viewMode.set(mode);
  }

  setGroupBy(group: GroupBy): void {
    this.groupBy.set(group);
    // Reset collapsed state when grouping changes
    this.collapsedSwimlanes.set(new Set());
  }

  toggleSwimlane(name: string): void {
    const collapsed = this.collapsedSwimlanes();
    const newCollapsed = new Set(collapsed);
    if (newCollapsed.has(name)) {
      newCollapsed.delete(name);
    } else {
      newCollapsed.add(name);
    }
    this.collapsedSwimlanes.set(newCollapsed);
  }

  isSwimlaneCollapsed(name: string): boolean {
    return this.collapsedSwimlanes().has(name);
  }

  expandAllSwimlanes(): void {
    this.collapsedSwimlanes.set(new Set());
  }

  collapseAllSwimlanes(): void {
    const allNames = this.swimlanes().map(s => s.name).filter(n => n);
    this.collapsedSwimlanes.set(new Set(allNames));
  }

  search(): void {
    this.loading.set(true);
    this.error.set(null);

    this.api.issuesSearch({
      query: this.searchQuery() || undefined,
      project: this.projectFilter() || undefined,
      status: this.statusFilter() || undefined,
      issueType: this.issueTypeFilter() || undefined,
      assignee: this.assigneeFilter() || undefined,
      limit: 200
    }).subscribe({
      next: (response) => {
        this.issues.set(response.issues);
        this.total.set(response.total);
        this.loading.set(false);
        this.extractAssignees();
        this.extractEpics();
      },
      error: (err) => {
        this.error.set('Search failed: ' + err);
        this.loading.set(false);
      }
    });
  }

  selectIssue(issue: Issue): void {
    this.selectedIssue.set(issue);
  }

  closeDetail(): void {
    this.selectedIssue.set(null);
  }

  clearFilters(): void {
    this.searchQuery.set('');
    this.projectFilter.set('');
    this.statusFilter.set('');
    this.issueTypeFilter.set('');
    this.assigneeFilter.set('');
    this.search();
  }

  onProjectChange(): void {
    // Reset dependent filters when project changes
    this.statusFilter.set('');
    this.issueTypeFilter.set('');

    // Load metadata for the selected project
    const project = this.projectFilter();
    if (project) {
      this.loadProjectMetadata(project);
    } else {
      this.loadAllMetadata();
    }

    // Execute search with new project filter
    this.search();
  }

  onSearchKeypress(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      this.search();
    }
  }

  // Due date helper methods
  isOverdue(issue: Issue): boolean {
    if (!issue.dueDate) return false;
    const dueDate = new Date(issue.dueDate);
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    return dueDate < today;
  }

  isDueSoon(issue: Issue): boolean {
    if (!issue.dueDate) return false;
    const dueDate = new Date(issue.dueDate);
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const threeDaysFromNow = new Date(today);
    threeDaysFromNow.setDate(threeDaysFromNow.getDate() + 3);
    return dueDate >= today && dueDate <= threeDaysFromNow;
  }

  formatDueDate(dueDate: string): string {
    const date = new Date(dueDate);
    const month = date.getMonth() + 1;
    const day = date.getDate();
    return `${month}/${day}`;
  }

  // Calendar view computed properties
  calendarMonth = signal(new Date());

  calendarDays = computed(() => {
    const month = this.calendarMonth();
    const year = month.getFullYear();
    const monthIndex = month.getMonth();

    // Get first and last day of month
    const firstDay = new Date(year, monthIndex, 1);
    const lastDay = new Date(year, monthIndex + 1, 0);

    // Get the start of the calendar (previous month days to fill the week)
    const startDate = new Date(firstDay);
    startDate.setDate(startDate.getDate() - firstDay.getDay());

    // Get the end of the calendar (next month days to fill the week)
    const endDate = new Date(lastDay);
    endDate.setDate(endDate.getDate() + (6 - lastDay.getDay()));

    const days: { date: Date; isCurrentMonth: boolean; issues: Issue[] }[] = [];
    const current = new Date(startDate);

    while (current <= endDate) {
      const dateStr = this.formatDateToISO(current);
      const dayIssues = this.issues().filter(issue =>
        issue.dueDate === dateStr
      );

      days.push({
        date: new Date(current),
        isCurrentMonth: current.getMonth() === monthIndex,
        issues: dayIssues
      });

      current.setDate(current.getDate() + 1);
    }

    return days;
  });

  formatDateToISO(date: Date): string {
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    return `${year}-${month}-${day}`;
  }

  previousMonth(): void {
    const current = this.calendarMonth();
    this.calendarMonth.set(new Date(current.getFullYear(), current.getMonth() - 1, 1));
  }

  nextMonth(): void {
    const current = this.calendarMonth();
    this.calendarMonth.set(new Date(current.getFullYear(), current.getMonth() + 1, 1));
  }

  goToToday(): void {
    this.calendarMonth.set(new Date());
  }

  isToday(date: Date): boolean {
    const today = new Date();
    return date.getFullYear() === today.getFullYear() &&
           date.getMonth() === today.getMonth() &&
           date.getDate() === today.getDate();
  }
}
