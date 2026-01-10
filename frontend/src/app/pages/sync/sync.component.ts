import { Component, OnInit, signal, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Project, SyncResult } from '../../generated/models';
import { API_SERVICE, IApiService } from '../../api.provider';

@Component({
  selector: 'app-sync',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './sync.component.html',
  styleUrl: './sync.component.scss'
})
export class SyncComponent implements OnInit {
  private api = inject<IApiService>(API_SERVICE);

  projects = signal<Project[]>([]);
  loading = signal(true);
  syncing = signal(false);
  selectedProject = signal<string | null>(null);
  forceFullSync = signal(false);
  syncResults = signal<SyncResult[]>([]);
  error = signal<string | null>(null);

  ngOnInit(): void {
    this.loadProjects();
  }

  loadProjects(): void {
    this.loading.set(true);
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

  startSync(force?: boolean): void {
    this.syncing.set(true);
    this.error.set(null);
    this.syncResults.set([]);

    const useForce = force ?? this.forceFullSync();
    const request = {
      ...(this.selectedProject() ? { projectKey: this.selectedProject()! } : {}),
      ...(useForce ? { force: true } : {})
    };

    this.api.syncExecute(request).subscribe({
      next: (response) => {
        this.syncResults.set(response.results);
        this.syncing.set(false);
        // Reset force checkbox after sync
        this.forceFullSync.set(false);
        // Refresh projects to update last_synced
        this.loadProjects();
      },
      error: (err) => {
        this.error.set('Sync failed: ' + err);
        this.syncing.set(false);
      }
    });
  }

  toggleForceSync(): void {
    this.forceFullSync.update(v => !v);
  }

  selectProject(key: string | null): void {
    this.selectedProject.set(key);
  }

  get enabledProjects(): Project[] {
    return this.projects().filter(p => p.enabled);
  }

  formatDuration(seconds: number): string {
    if (seconds < 60) {
      return `${seconds.toFixed(1)}s`;
    }
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    return `${minutes}m ${remainingSeconds.toFixed(0)}s`;
  }

  formatDate(dateStr?: string): string {
    if (!dateStr) return 'Never';
    const date = new Date(dateStr);
    return date.toLocaleString();
  }
}
