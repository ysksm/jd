import { Component, OnInit, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { TauriApiService } from '../../generated/tauri-api.service';
import { Project, SyncResult } from '../../generated/models';

@Component({
  selector: 'app-sync',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './sync.component.html',
  styleUrl: './sync.component.scss'
})
export class SyncComponent implements OnInit {
  projects = signal<Project[]>([]);
  loading = signal(true);
  syncing = signal(false);
  selectedProject = signal<string | null>(null);
  syncResults = signal<SyncResult[]>([]);
  error = signal<string | null>(null);

  constructor(private api: TauriApiService) {}

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

  startSync(): void {
    this.syncing.set(true);
    this.error.set(null);
    this.syncResults.set([]);

    const request = this.selectedProject()
      ? { projectKey: this.selectedProject()! }
      : {};

    this.api.syncExecute(request).subscribe({
      next: (response) => {
        this.syncResults.set(response.results);
        this.syncing.set(false);
        // Refresh projects to update last_synced
        this.loadProjects();
      },
      error: (err) => {
        this.error.set('Sync failed: ' + err);
        this.syncing.set(false);
      }
    });
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
