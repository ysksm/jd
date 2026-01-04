import { Component, OnInit, signal, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Project } from '../../generated/models';
import { API_SERVICE, IApiService } from '../../api.provider';

@Component({
  selector: 'app-projects',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './projects.component.html',
  styleUrl: './projects.component.scss'
})
export class ProjectsComponent implements OnInit {
  private api = inject<IApiService>(API_SERVICE);

  projects = signal<Project[]>([]);
  loading = signal(true);
  syncing = signal(false);
  error = signal<string | null>(null);
  success = signal<string | null>(null);

  ngOnInit(): void {
    this.loadProjects();
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
    this.syncing.set(true);
    this.error.set(null);
    this.success.set(null);

    this.api.projectsInitialize({}).subscribe({
      next: (response) => {
        this.projects.set(response.projects);
        this.success.set(`Fetched ${response.newCount} projects from JIRA`);
        this.syncing.set(false);
      },
      error: (err) => {
        this.error.set('Failed to fetch projects: ' + err);
        this.syncing.set(false);
      }
    });
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
}
