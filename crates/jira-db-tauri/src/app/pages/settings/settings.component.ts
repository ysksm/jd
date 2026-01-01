import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';

@Component({
  selector: 'app-settings',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="page">
      <h2>Settings</h2>
      <p>Configuration settings coming soon...</p>
    </div>
  `
})
export class SettingsComponent {}
