import { Component, OnInit, signal, inject, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ActivatedRoute, Router } from '@angular/router';
import { Project, ProjectMetadata } from '../../generated/models';
import { API_SERVICE, IApiService } from '../../api.provider';
import { IssuesComponent } from '../issues/issues.component';
import { QueryComponent } from '../query/query.component';
import { VisualizationComponent } from '../visualization/visualization.component';

type TabType = 'overview' | 'issues' | 'query' | 'charts';

@Component({
  selector: 'app-project-detail',
  standalone: true,
  imports: [CommonModule, IssuesComponent, QueryComponent, VisualizationComponent],
  templateUrl: './project-detail.component.html',
  styleUrl: './project-detail.component.scss'
})
export class ProjectDetailComponent implements OnInit {
  private route = inject(ActivatedRoute);
  private router = inject(Router);
  private api = inject<IApiService>(API_SERVICE);

  projectKey = signal<string>('');
  project = signal<Project | null>(null);
  metadata = signal<ProjectMetadata | null>(null);
  loading = signal(true);
  error = signal<string | null>(null);

  // Active tab based on URL
  activeTab = signal<TabType>('overview');

  // Tab definitions
  tabs: { id: TabType; label: string; icon: string }[] = [
    { id: 'overview', label: 'Overview', icon: 'info' },
    { id: 'issues', label: 'Issues', icon: 'list' },
    { id: 'query', label: 'SQL Query', icon: 'code' },
    { id: 'charts', label: 'Charts', icon: 'chart' },
  ];

  // Computed stats
  issueStats = computed(() => {
    const meta = this.metadata();
    if (!meta) return null;
    return {
      statusCount: meta.statuses.length,
      priorityCount: meta.priorities.length,
      issueTypeCount: meta.issueTypes.length,
      labelCount: meta.labels.length,
      componentCount: meta.components.length,
      versionCount: meta.fixVersions.length,
    };
  });

  ngOnInit(): void {
    // Get project key from route
    this.route.paramMap.subscribe(params => {
      const key = params.get('projectKey');
      if (key) {
        this.projectKey.set(key);
        this.loadProject();
        this.loadMetadata();
      }
    });

    // Get active tab from child route
    this.route.url.subscribe(() => {
      const childPath = this.route.firstChild?.snapshot.url[0]?.path;
      if (childPath) {
        this.activeTab.set(childPath as TabType);
      }
    });
  }

  loadProject(): void {
    this.loading.set(true);
    this.api.projectsList({}).subscribe({
      next: (response) => {
        const proj = response.projects.find(p => p.key === this.projectKey());
        if (proj) {
          this.project.set(proj);
        } else {
          this.error.set(`Project ${this.projectKey()} not found`);
        }
        this.loading.set(false);
      },
      error: (err) => {
        this.error.set('Failed to load project: ' + err);
        this.loading.set(false);
      }
    });
  }

  loadMetadata(): void {
    this.api.metadataGet({ projectKey: this.projectKey() }).subscribe({
      next: (response) => {
        this.metadata.set(response.metadata);
      },
      error: (err) => {
        console.error('Failed to load metadata:', err);
      }
    });
  }

  setActiveTab(tab: TabType): void {
    this.activeTab.set(tab);
    this.router.navigate(['projects', this.projectKey(), tab]);
  }

  goBack(): void {
    this.router.navigate(['/projects']);
  }

  formatDate(date: string | undefined): string {
    if (!date) return 'Never';
    return new Date(date).toLocaleString();
  }
}
