import { Component, OnInit, signal, inject, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Issue, Project, Status, IssueType } from '../../generated/models';
import { API_SERVICE, IApiService } from '../../api.provider';

type ViewMode = 'list' | 'board';
type GroupBy = 'none' | 'assignee' | 'epic';

interface BoardGroup {
  name: string;
  issues: Issue[];
}

interface BoardColumn {
  status: string;
  category: string;
  groups: BoardGroup[];
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

  // Board view computed property
  boardColumns = computed<BoardColumn[]>(() => {
    const statusList = this.statuses();
    const issues = this.issues();
    const groupByValue = this.groupBy();

    // Create a map of status to category
    const statusCategoryMap = new Map<string, string>();
    statusList.forEach(s => statusCategoryMap.set(s.name, s.category));

    // Get unique statuses from issues if no metadata is loaded
    const statusNames = statusList.length > 0
      ? statusList.map(s => s.name)
      : [...new Set(issues.map(i => i.status))];

    return statusNames.map(status => {
      const statusIssues = issues.filter(i => i.status === status);
      const category = statusCategoryMap.get(status) || 'default';

      let groups: BoardGroup[];
      if (groupByValue === 'none') {
        groups = [{ name: '', issues: statusIssues }];
      } else if (groupByValue === 'assignee') {
        const assigneeMap = new Map<string, Issue[]>();
        assigneeMap.set('Unassigned', []);
        statusIssues.forEach(issue => {
          const key = issue.assignee || 'Unassigned';
          if (!assigneeMap.has(key)) {
            assigneeMap.set(key, []);
          }
          assigneeMap.get(key)!.push(issue);
        });
        groups = Array.from(assigneeMap.entries())
          .filter(([, issues]) => issues.length > 0)
          .sort((a, b) => {
            if (a[0] === 'Unassigned') return 1;
            if (b[0] === 'Unassigned') return -1;
            return a[0].localeCompare(b[0]);
          })
          .map(([name, issues]) => ({ name, issues }));
      } else {
        // Group by epic (parentKey)
        const epicMap = new Map<string, Issue[]>();
        epicMap.set('No Epic', []);
        statusIssues.forEach(issue => {
          const key = issue.parentKey || 'No Epic';
          if (!epicMap.has(key)) {
            epicMap.set(key, []);
          }
          epicMap.get(key)!.push(issue);
        });
        groups = Array.from(epicMap.entries())
          .filter(([, issues]) => issues.length > 0)
          .sort((a, b) => {
            if (a[0] === 'No Epic') return 1;
            if (b[0] === 'No Epic') return -1;
            return a[0].localeCompare(b[0]);
          })
          .map(([name, issues]) => ({ name, issues }));
      }

      return { status, category, groups };
    });
  });

  setViewMode(mode: ViewMode): void {
    this.viewMode.set(mode);
  }

  setGroupBy(group: GroupBy): void {
    this.groupBy.set(group);
  }

  getColumnIssueCount(column: BoardColumn): number {
    return column.groups.reduce((sum, g) => sum + g.issues.length, 0);
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
