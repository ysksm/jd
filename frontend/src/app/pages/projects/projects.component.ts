import { Component, OnInit, signal, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Router } from '@angular/router';
import { Project, JiraEndpoint, EndpointFetchResult } from '../../generated/models';
import { API_SERVICE, IApiService } from '../../api.provider';

@Component({
  selector: 'app-projects',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './projects.component.html',
  styleUrl: './projects.component.scss'
})
export class ProjectsComponent implements OnInit {
  private api = inject<IApiService>(API_SERVICE);
  private router = inject(Router);

  projects = signal<Project[]>([]);
  loading = signal(true);
  syncing = signal(false);
  error = signal<string | null>(null);
  success = signal<string | null>(null);

  // Endpoint selection
  endpoints = signal<JiraEndpoint[]>([]);
  activeEndpoint = signal<string | null>(null);
  showFetchOptions = signal(false);
  selectedFetchEndpoint = signal<string>('active'); // 'active', 'all', or endpoint name
  lastFetchResults = signal<EndpointFetchResult[] | null>(null);

  ngOnInit(): void {
    this.loadSettings();
    this.loadProjects();
  }

  loadSettings(): void {
    this.api.configGet({}).subscribe({
      next: (response) => {
        if (response.settings.jiraEndpoints) {
          this.endpoints.set(response.settings.jiraEndpoints);
        }
        this.activeEndpoint.set(response.settings.activeEndpoint || null);
      },
      error: () => {
        // Ignore settings load error
      }
    });
  }

  loadProjects(): void {
    this.loading.set(true);
    this.error.set(null);

    this.api.projectsList({}).subscribe({
      next: (response) => {
        this.projects.set(response.projects);
        this.loading.set(false);
      },
      error: (err) => {
        this.error.set('Failed to load projects: ' + err);
        this.loading.set(false);
      }
    });
  }

  initializeProjects(): void {
    const selection = this.selectedFetchEndpoint();

    this.syncing.set(true);
    this.error.set(null);
    this.success.set(null);
    this.lastFetchResults.set(null);

    const request: { endpointName?: string; allEndpoints?: boolean } = {};

    if (selection === 'all') {
      request.allEndpoints = true;
    } else if (selection !== 'active') {
      request.endpointName = selection;
    }
    // If 'active', send empty request (uses active endpoint)

    this.api.projectsInitialize(request).subscribe({
      next: (response) => {
        this.projects.set(response.projects);

        if (response.endpointResults && response.endpointResults.length > 0) {
          this.lastFetchResults.set(response.endpointResults);
          const successCount = response.endpointResults.filter(r => r.success).length;
          const totalEndpoints = response.endpointResults.length;
          this.success.set(
            `Fetched from ${successCount}/${totalEndpoints} endpoints. ` +
            `${response.newCount} new projects added.`
          );
        } else {
          this.success.set(`Fetched ${response.newCount} new projects from JIRA`);
        }

        this.syncing.set(false);
        this.showFetchOptions.set(false);
      },
      error: (err) => {
        this.error.set('Failed to fetch projects: ' + err);
        this.syncing.set(false);
      }
    });
  }

  getEndpointDisplayName(endpoint: JiraEndpoint): string {
    return endpoint.displayName || endpoint.name;
  }

  hasMultipleEndpoints(): boolean {
    return this.endpoints().length > 1;
  }

  enableProject(key: string): void {
    this.api.projectsEnable({ key }).subscribe({
      next: (response) => {
        this.updateProject(response.project);
        this.success.set(`Enabled sync for ${key}`);
      },
      error: (err) => {
        this.error.set('Failed to enable project: ' + err);
      }
    });
  }

  disableProject(key: string): void {
    this.api.projectsDisable({ key }).subscribe({
      next: (response) => {
        this.updateProject(response.project);
        this.success.set(`Disabled sync for ${key}`);
      },
      error: (err) => {
        this.error.set('Failed to disable project: ' + err);
      }
    });
  }

  private updateProject(updated: Project): void {
    this.projects.update(projects =>
      projects.map(p => p.key === updated.key ? updated : p)
    );
  }

  get enabledCount(): number {
    return this.projects().filter(p => p.enabled).length;
  }

  get totalCount(): number {
    return this.projects().length;
  }

  goToProject(projectKey: string): void {
    this.router.navigate(['/projects', projectKey]);
  }
}
