import { Component, OnInit, OnDestroy, signal, inject, PLATFORM_ID } from '@angular/core';
import { CommonModule, isPlatformBrowser } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Project, SyncResult, SyncProgress } from '../../generated/models';
import { API_SERVICE, IApiService } from '../../api.provider';

// Tauri event types
type UnlistenFn = () => void;

@Component({
  selector: 'app-sync',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './sync.component.html',
  styleUrl: './sync.component.scss'
})
export class SyncComponent implements OnInit, OnDestroy {
  private api = inject<IApiService>(API_SERVICE);
  private platformId = inject(PLATFORM_ID);
  private unlisten: UnlistenFn | null = null;

  projects = signal<Project[]>([]);
  loading = signal(true);
  syncing = signal(false);
  selectedProject = signal<string | null>(null);
  forceFullSync = signal(false);
  syncResults = signal<SyncResult[]>([]);
  error = signal<string | null>(null);

  // Progress tracking
  currentProgress = signal<SyncProgress | null>(null);
  progressHistory = signal<SyncProgress[]>([]);
  showProgressPanel = signal(false);

  // All sync phases in order
  readonly syncPhases = ['fields', 'columns', 'issues', 'expand', 'views', 'complete'] as const;

  ngOnInit(): void {
    this.loadProjects();
    this.setupProgressListener();
  }

  ngOnDestroy(): void {
    if (this.unlisten) {
      this.unlisten();
    }
  }

  private async setupProgressListener(): Promise<void> {
    // Only setup listener in browser (Tauri) environment
    if (!isPlatformBrowser(this.platformId)) {
      return;
    }

    try {
      // Dynamic import to avoid SSR issues
      const { listen } = await import('@tauri-apps/api/event');
      this.unlisten = await listen<SyncProgress>('sync-progress', (event) => {
        const progress = event.payload;
        this.currentProgress.set(progress);

        // Add to history
        this.progressHistory.update(history => [...history, progress]);
      });
    } catch (e) {
      // Not running in Tauri, ignore
      console.debug('Tauri event listener not available:', e);
    }
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
    this.currentProgress.set(null);
    this.progressHistory.set([]);
    this.showProgressPanel.set(true);

    const useForce = force ?? this.forceFullSync();
    const request = {
      ...(this.selectedProject() ? { projectKey: this.selectedProject()! } : {}),
      ...(useForce ? { force: true } : {})
    };

    this.api.syncExecute(request).subscribe({
      next: (response) => {
        this.syncResults.set(response.results);
        this.syncing.set(false);
        // Keep currentProgress and progressHistory visible after completion
        // Reset force checkbox after sync
        this.forceFullSync.set(false);
        // Refresh projects to update last_synced
        this.loadProjects();
      },
      error: (err) => {
        this.error.set('Sync failed: ' + err);
        this.syncing.set(false);
        // Keep progress panel visible on error too
      }
    });
  }

  dismissProgressPanel(): void {
    this.showProgressPanel.set(false);
    this.currentProgress.set(null);
    this.progressHistory.set([]);
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

  getPhaseLabel(phase: string): string {
    const labels: Record<string, string> = {
      'fields': 'Syncing Fields',
      'columns': 'Adding Columns',
      'issues': 'Fetching Issues',
      'expand': 'Expanding Data',
      'views': 'Creating Views',
      'complete': 'Complete'
    };
    return labels[phase] || phase;
  }

  getPhaseStatus(phase: string): 'pending' | 'in_progress' | 'completed' {
    const currentPhase = this.currentProgress()?.phase;
    if (!currentPhase) {
      // No progress yet - all pending, or all completed if sync finished
      if (!this.syncing() && this.progressHistory().length > 0) {
        // Sync finished - check if this phase was completed
        const completed = this.progressHistory().some(p => p.phase === phase);
        return completed ? 'completed' : 'pending';
      }
      return 'pending';
    }

    const currentIndex = this.syncPhases.indexOf(currentPhase as typeof this.syncPhases[number]);
    const phaseIndex = this.syncPhases.indexOf(phase as typeof this.syncPhases[number]);

    if (phaseIndex < currentIndex) {
      return 'completed';
    } else if (phaseIndex === currentIndex) {
      return 'in_progress';
    } else {
      return 'pending';
    }
  }

  getPhaseIcon(status: 'pending' | 'in_progress' | 'completed'): string {
    switch (status) {
      case 'completed': return '✓';
      case 'in_progress': return '●';
      case 'pending': return '○';
    }
  }
}
