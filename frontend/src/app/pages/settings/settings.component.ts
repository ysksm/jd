import { Component, OnInit, signal, computed, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Settings, JiraConfig, JiraEndpoint, EmbeddingsConfig, LogConfig } from '../../generated/models';
import { API_SERVICE, IApiService } from '../../api.provider';

@Component({
  selector: 'app-settings',
  standalone: true,
  imports: [CommonModule, FormsModule],
  templateUrl: './settings.component.html',
  styleUrl: './settings.component.scss'
})
export class SettingsComponent implements OnInit {
  private api = inject<IApiService>(API_SERVICE);

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

  // Log settings
  logFileEnabled = signal(false);
  logFileDir = signal('');
  logLevel = signal('info');
  logMaxFiles = signal(10);

  // Multiple endpoints support
  endpoints = signal<JiraEndpoint[]>([]);
  activeEndpoint = signal<string | null>(null);
  showAddEndpoint = signal(false);
  newEndpointName = signal('');
  newEndpointDisplayName = signal('');
  // Dedicated fields for new endpoint
  newEndpointUrl = signal('');
  newEndpointUsername = signal('');
  newEndpointApiKey = signal('');

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

    if (settings.log) {
      this.logFileEnabled.set(settings.log.fileEnabled);
      this.logFileDir.set(settings.log.fileDir || '');
      this.logLevel.set(settings.log.level);
      this.logMaxFiles.set(settings.log.maxFiles);
    }

    // Populate endpoints
    if (settings.jiraEndpoints && settings.jiraEndpoints.length > 0) {
      this.endpoints.set(settings.jiraEndpoints);
      this.activeEndpoint.set(settings.activeEndpoint || settings.jiraEndpoints[0].name);
    } else {
      // Create default endpoint from legacy jira config
      const defaultEndpoint: JiraEndpoint = {
        name: 'default',
        displayName: 'Default',
        endpoint: settings.jira.endpoint,
        username: settings.jira.username,
        apiKey: settings.jira.apiKey
      };
      this.endpoints.set([defaultEndpoint]);
      this.activeEndpoint.set('default');
    }
  }

  selectEndpoint(name: string): void {
    const endpoint = this.endpoints().find(e => e.name === name);
    if (endpoint) {
      this.activeEndpoint.set(name);
      this.jiraEndpoint.set(endpoint.endpoint);
      this.jiraUsername.set(endpoint.username);
      this.jiraApiKey.set(endpoint.apiKey);
    }
  }

  addEndpoint(): void {
    const name = this.newEndpointName().trim();
    const endpointUrl = this.newEndpointUrl().trim();
    const username = this.newEndpointUsername().trim();
    const apiKey = this.newEndpointApiKey().trim();

    // Validate required fields
    const missingFields: string[] = [];
    if (!name) missingFields.push('Endpoint Name');
    if (!endpointUrl) missingFields.push('Endpoint URL');
    if (!username) missingFields.push('Username');
    if (!apiKey) missingFields.push('API Key');

    if (missingFields.length > 0) {
      this.error.set(`Missing required fields: ${missingFields.join(', ')}`);
      return;
    }

    if (this.endpoints().some(e => e.name === name)) {
      this.error.set('An endpoint with this name already exists');
      return;
    }

    const newEndpoint: JiraEndpoint = {
      name: name,
      displayName: this.newEndpointDisplayName().trim() || name,
      endpoint: endpointUrl,
      username: username,
      apiKey: apiKey
    };

    this.saving.set(true);
    this.error.set(null);

    this.api.configUpdate({
      addEndpoint: newEndpoint
    }).subscribe({
      next: (response) => {
        this.settings.set(response.settings);
        this.populateForm(response.settings);
        this.resetNewEndpointForm();
        this.success.set('Endpoint added successfully!');
        this.saving.set(false);
      },
      error: (err) => {
        this.error.set('Failed to add endpoint: ' + err);
        this.saving.set(false);
      }
    });
  }

  resetNewEndpointForm(): void {
    this.showAddEndpoint.set(false);
    this.newEndpointName.set('');
    this.newEndpointDisplayName.set('');
    this.newEndpointUrl.set('');
    this.newEndpointUsername.set('');
    this.newEndpointApiKey.set('');
  }

  removeEndpoint(name: string): void {
    if (this.endpoints().length <= 1) {
      this.error.set('Cannot remove the last endpoint');
      return;
    }

    if (!confirm(`Are you sure you want to remove the endpoint "${name}"?`)) {
      return;
    }

    this.saving.set(true);
    this.error.set(null);

    this.api.configUpdate({
      removeEndpoint: name
    }).subscribe({
      next: (response) => {
        this.settings.set(response.settings);
        this.populateForm(response.settings);
        this.success.set('Endpoint removed successfully!');
        this.saving.set(false);
      },
      error: (err) => {
        this.error.set('Failed to remove endpoint: ' + err);
        this.saving.set(false);
      }
    });
  }

  setActiveEndpoint(name: string): void {
    this.saving.set(true);
    this.error.set(null);

    this.api.configUpdate({
      setActiveEndpoint: name
    }).subscribe({
      next: (response) => {
        this.settings.set(response.settings);
        this.populateForm(response.settings);
        this.selectEndpoint(name);
        this.success.set('Active endpoint changed successfully!');
        this.saving.set(false);
      },
      error: (err) => {
        this.error.set('Failed to set active endpoint: ' + err);
        this.saving.set(false);
      }
    });
  }

  getEndpointDisplayName(endpoint: JiraEndpoint): string {
    return endpoint.displayName || endpoint.name;
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
      },
      log: {
        fileEnabled: this.logFileEnabled(),
        fileDir: this.logFileDir() || undefined,
        level: this.logLevel(),
        maxFiles: this.logMaxFiles()
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
