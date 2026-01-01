import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'app-projects',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="page">
      <h2>Projects</h2>
      <p>Project management coming soon...</p>
    </div>
  `
})
export class ProjectsComponent {}
