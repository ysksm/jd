import { Component, OnInit, signal, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { ApiService } from '../../generated/api.service';
import { Settings, JiraConfig, EmbeddingsConfig } from '../../generated/models';

@Component({
  selector: 'app-settings',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './settings.component.html',
  styleUrl: './settings.component.scss'
})
export class SettingsComponent implements OnInit {
  settings = signal<Settings | null>(null);
  loading = signal(true);
  saving = signal(false);
  error = signal<string | null>(null);
  success = signal<string | null>(null);
  initialized = signal(false);

  // Form fields
  jiraEndpoint = signal('');
  jiraUsername = signal('');
  jiraApiKey = signal('');
  databasePath = signal('');
  embeddingsProvider = signal('openai');
  embeddingsModel = signal('');
  embeddingsEndpoint = signal('');
  embeddingsAutoGenerate = signal(false);

  constructor(private api: ApiService) {}

  ngOnInit(): void {
    this.loadSettings();
  }

  loadSettings(): void {
    this.loading.set(true);
    this.error.set(null);

    this.api.configGet({}).subscribe({
      next: (response) => {
        this.settings.set(response.settings);
        this.populateForm(response.settings);
        this.initialized.set(true);
        this.loading.set(false);
      },
      error: (err) => {
        // Not initialized yet
        this.initialized.set(false);
        this.loading.set(false);
      }
    });
  }

  populateForm(settings: Settings): void {
    this.jiraEndpoint.set(settings.jira.endpoint);
    this.jiraUsername.set(settings.jira.username);
    this.jiraApiKey.set(settings.jira.apiKey);
    this.databasePath.set(settings.database.path);

    if (settings.embeddings) {
      this.embeddingsProvider.set(settings.embeddings.provider);
      this.embeddingsModel.set(settings.embeddings.modelName || '');
      this.embeddingsEndpoint.set(settings.embeddings.endpoint || '');
      this.embeddingsAutoGenerate.set(settings.embeddings.autoGenerate);
    }
  }

  initializeSettings(): void {
    const endpoint = this.jiraEndpoint();
    const username = this.jiraUsername();
    const apiKey = this.jiraApiKey();

    console.log('Initialize settings - values:', {
      endpoint: endpoint || '(empty)',
      username: username || '(empty)',
      apiKey: apiKey ? '(set)' : '(empty)'
    });

    const missingFields: string[] = [];
    if (!endpoint || endpoint.trim() === '') missingFields.push('Endpoint');
    if (!username || username.trim() === '') missingFields.push('Username');
    if (!apiKey || apiKey.trim() === '') missingFields.push('API Key');

    if (missingFields.length > 0) {
      this.error.set(`Missing required fields: ${missingFields.join(', ')}`);
      return;
    }

    this.saving.set(true);
    this.error.set(null);
    this.success.set(null);

    this.api.configInitialize({
      endpoint: this.jiraEndpoint(),
      username: this.jiraUsername(),
      apiKey: this.jiraApiKey(),
      databasePath: this.databasePath() || undefined
    }).subscribe({
      next: (response) => {
        this.settings.set(response.settings);
        this.initialized.set(true);
        this.success.set('Settings initialized successfully!');
        this.saving.set(false);
      },
      error: (err) => {
        this.error.set('Failed to initialize settings: ' + err);
        this.saving.set(false);
      }
    });
  }

  saveSettings(): void {
    this.saving.set(true);
    this.error.set(null);
    this.success.set(null);

    this.api.configUpdate({
      jira: {
        endpoint: this.jiraEndpoint(),
        username: this.jiraUsername(),
        apiKey: this.jiraApiKey()
      },
      database: {
        path: this.databasePath()
      },
      embeddings: {
        provider: this.embeddingsProvider(),
        modelName: this.embeddingsModel() || undefined,
        endpoint: this.embeddingsEndpoint() || undefined,
        autoGenerate: this.embeddingsAutoGenerate()
      }
    }).subscribe({
      next: (response) => {
        this.settings.set(response.settings);
        this.success.set('Settings saved successfully!');
        this.saving.set(false);
      },
      error: (err) => {
        this.error.set('Failed to save settings: ' + err);
        this.saving.set(false);
      }
    });
  }
}
