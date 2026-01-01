import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'app-issues',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="page">
      <h2>Issues</h2>
      <p>Issue search and browsing coming soon...</p>
    </div>
  `
})
export class IssuesComponent {}
