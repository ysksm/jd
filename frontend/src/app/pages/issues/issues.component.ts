import { Component, OnInit, signal, inject, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Issue, Project, Status, IssueType } from '../../generated/models';
import { API_SERVICE, IApiService } from '../../api.provider';

type ViewMode = 'list' | 'board';
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
  imports: [CommonModule, FormsModule],
  templateUrl: './issues.component.html',
  styleUrl: './issues.component.scss'
})
export class IssuesComponent implements OnInit {
  private api = inject<IApiService>(API_SERVICE);

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

  ngOnInit(): void {
    this.loadProjects();
    this.search();
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

    // Debug logging for Epic grouping
    if (groupByValue === 'epic') {
      console.group('Epic Grouping Debug');
      console.log('Total issues:', issues.length);
      console.log('Unique issueTypes:', [...new Set(issues.map(i => i.issueType))]);
      console.log('Epic keys found:', Array.from(epicKeys));
      console.log('Issues with parentKey:', issues.filter(i => i.parentKey).map(i => ({
        key: i.key,
        issueType: i.issueType,
        parentKey: i.parentKey
      })));
      console.groupEnd();
    }

    issues.forEach(issue => {
      let key: string;
      if (groupByValue === 'assignee') {
        key = issue.assignee || 'Unassigned';
      } else {
        // Epic grouping - find the Epic this issue belongs to
        if (this.isEpicType(issue.issueType)) {
          // Epics themselves are group headers - use their key as the group name
          // but we'll show their summary for better UX
          const epicSummary = `${issue.key}: ${issue.summary}`;
          key = epicSummary;
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
      groupMap.get(key)!.push(issue);
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
}
