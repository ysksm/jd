import { Component, OnInit, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { TauriApiService } from '../../generated/tauri-api.service';
import { SyncStatusResponse, Project } from '../../generated/models';

@Component({
  selector: 'app-dashboard',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './dashboard.component.html',
  styleUrl: './dashboard.component.scss'
})
export class DashboardComponent implements OnInit {
  syncStatus = signal<SyncStatusResponse | null>(null);
  projects = signal<Project[]>([]);
  error = signal<string | null>(null);

  constructor(private api: TauriApiService) {}

  ngOnInit(): void {
    this.loadData();
  }

  loadData(): void {
    this.api.syncStatus({}).subscribe({
      next: (status) => this.syncStatus.set(status),
      error: (err) => this.error.set('Failed to load sync status')
    });

    this.api.projectsList({}).subscribe({
      next: (response) => this.projects.set(response.projects),
      error: (err) => this.error.set('Failed to load projects')
    });
  }

  startSync(): void {
    this.api.syncExecute({}).subscribe({
      next: () => this.loadData(),
      error: (err) => this.error.set('Sync failed')
    });
  }
}
