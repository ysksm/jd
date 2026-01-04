import { Component, OnInit, signal, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Issue } from '../../generated/models';
import { API_SERVICE, IApiService } from '../../api.provider';

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

  // Search filters
  searchQuery = signal('');
  projectFilter = signal('');
  statusFilter = signal('');
  assigneeFilter = signal('');

  ngOnInit(): void {
    this.search();
  }

  search(): void {
    this.loading.set(true);
    this.error.set(null);

    this.api.issuesSearch({
      query: this.searchQuery() || undefined,
      project: this.projectFilter() || undefined,
      status: this.statusFilter() || undefined,
      assignee: this.assigneeFilter() || undefined,
      limit: 50
    }).subscribe({
      next: (response) => {
        this.issues.set(response.issues);
        this.total.set(response.total);
        this.loading.set(false);
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
    this.assigneeFilter.set('');
    this.search();
  }

  onSearchKeypress(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      this.search();
    }
  }
}
