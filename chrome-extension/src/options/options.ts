import type { Settings, ProjectConfig, AuthMethod } from '../lib/types';

// DOM elements
const endpointEl = document.getElementById('endpoint') as HTMLInputElement;
const authBrowserEl = document.getElementById('authBrowser') as HTMLInputElement;
const authApiTokenEl = document.getElementById('authApiToken') as HTMLInputElement;
const apiTokenFieldsEl = document.getElementById('apiTokenFields')!;
const usernameEl = document.getElementById('username') as HTMLInputElement;
const apiKeyEl = document.getElementById('apiKey') as HTMLInputElement;
const testConnectionBtnEl = document.getElementById('testConnectionBtn') as HTMLButtonElement;
const saveConnectionBtnEl = document.getElementById('saveConnectionBtn') as HTMLButtonElement;
const connectionStatusEl = document.getElementById('connectionStatus')!;

const incrementalSyncEl = document.getElementById('incrementalSync') as HTMLInputElement;
const marginMinutesEl = document.getElementById('marginMinutes') as HTMLInputElement;
const batchSizeEl = document.getElementById('batchSize') as HTMLInputElement;
const saveSyncSettingsBtnEl = document.getElementById('saveSyncSettingsBtn') as HTMLButtonElement;

const fetchProjectsBtnEl = document.getElementById('fetchProjectsBtn') as HTMLButtonElement;
const projectListEl = document.getElementById('projectList')!;
const projectsStatusEl = document.getElementById('projectsStatus')!;

const exportDataBtnEl = document.getElementById('exportDataBtn') as HTMLButtonElement;
const clearDataBtnEl = document.getElementById('clearDataBtn') as HTMLButtonElement;
const dataStatusEl = document.getElementById('dataStatus')!;

// State
let settings: Settings | null = null;
let projects: (ProjectConfig & { issueCount?: number; hasCheckpoint?: boolean })[] = [];

// Initialize
async function init() {
  const response = await sendMessage<Settings>({ type: 'GET_SETTINGS' });
  if (response.success && response.data) {
    settings = response.data;
    populateForm();
  }

  await loadProjects();
}

function populateForm() {
  if (!settings) return;

  // JIRA connection
  endpointEl.value = settings.jira.endpoint;

  // Auth method
  const authMethod = settings.jira.authMethod || 'browser';
  if (authMethod === 'browser') {
    authBrowserEl.checked = true;
    apiTokenFieldsEl.style.display = 'none';
  } else {
    authApiTokenEl.checked = true;
    apiTokenFieldsEl.style.display = 'block';
  }

  usernameEl.value = settings.jira.username;
  apiKeyEl.value = settings.jira.apiKey;

  // Sync settings
  incrementalSyncEl.checked = settings.sync.incrementalSyncEnabled;
  marginMinutesEl.value = String(settings.sync.incrementalSyncMarginMinutes);
  batchSizeEl.value = String(settings.sync.batchSize);
}

function getSelectedAuthMethod(): AuthMethod {
  return authApiTokenEl.checked ? 'api_token' : 'browser';
}

function toggleApiTokenFields() {
  apiTokenFieldsEl.style.display = authApiTokenEl.checked ? 'block' : 'none';
}

async function loadProjects() {
  const response = await sendMessage<(ProjectConfig & { issueCount?: number; hasCheckpoint?: boolean })[]>({
    type: 'GET_PROJECTS',
  });

  if (response.success && response.data) {
    projects = response.data;
    renderProjects();
  } else if (response.error) {
    // If GET_PROJECTS fails (e.g., database not ready), try to at least show projects from settings
    console.warn('Failed to load projects with status:', response.error);
    const settingsResponse = await sendMessage<Settings>({ type: 'GET_SETTINGS' });
    if (settingsResponse.success && settingsResponse.data?.projects) {
      projects = settingsResponse.data.projects.map(p => ({
        ...p,
        issueCount: undefined,
        hasCheckpoint: !!p.syncCheckpoint,
      }));
      renderProjects();
    }
  }
}

function renderProjects() {
  if (projects.length === 0) {
    projectListEl.innerHTML = `
      <div class="empty-state">
        <p>No projects loaded. Click "Fetch Projects" to load from JIRA.</p>
      </div>
    `;
    return;
  }

  projectListEl.innerHTML = projects
    .map((project) => {
      const issueCount = project.issueCount || 0;
      const lastSync = project.lastSyncedAt
        ? new Date(project.lastSyncedAt).toLocaleString()
        : 'Never';

      return `
        <div class="project-item" data-key="${project.key}">
          <div class="project-info">
            <span class="project-key">${escapeHtml(project.key)}</span>
            <span class="project-name">${escapeHtml(project.name)}</span>
            ${project.hasCheckpoint ? '<span class="checkpoint-badge">Resume available</span>' : ''}
          </div>
          <div class="project-meta">
            ${issueCount} issues | Last sync: ${lastSync}
          </div>
          <div class="project-actions">
            <label class="project-toggle">
              <input type="checkbox" ${project.enabled ? 'checked' : ''} data-project="${project.key}">
              <span class="slider"></span>
            </label>
          </div>
        </div>
      `;
    })
    .join('');

  // Add toggle handlers
  projectListEl.querySelectorAll('input[type="checkbox"]').forEach((checkbox) => {
    checkbox.addEventListener('change', async (e) => {
      const target = e.target as HTMLInputElement;
      const projectKey = target.dataset.project;
      if (projectKey) {
        const response = await sendMessage({
          type: 'ENABLE_PROJECT',
          payload: { projectKey, enabled: target.checked },
        });
        if (!response.success) {
          showStatus(projectsStatusEl, 'error', `Failed to update project: ${response.error}`);
          // Revert checkbox state
          target.checked = !target.checked;
        }
      }
    });
  });
}

function escapeHtml(str: string): string {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

function showStatus(
  element: HTMLElement,
  type: 'success' | 'error' | 'info',
  message: string
) {
  element.className = `status-message ${type}`;
  element.textContent = message;

  // Auto-hide after 5 seconds
  setTimeout(() => {
    element.className = 'status-message';
    element.textContent = '';
  }, 5000);
}

// Test JIRA connection
async function testConnection() {
  testConnectionBtnEl.disabled = true;
  testConnectionBtnEl.innerHTML = '<span class="loading"><span class="spinner"></span>Testing...</span>';

  const authMethod = getSelectedAuthMethod();

  // Temporarily save settings for testing
  const tempSettings: Settings = {
    ...settings!,
    jira: {
      endpoint: endpointEl.value.trim(),
      authMethod,
      username: usernameEl.value.trim(),
      apiKey: apiKeyEl.value.trim(),
    },
  };

  await sendMessage({ type: 'SAVE_SETTINGS', payload: tempSettings });

  // Try to init projects (this will test the connection)
  const response = await sendMessage({ type: 'INIT_PROJECTS' });

  testConnectionBtnEl.disabled = false;
  testConnectionBtnEl.textContent = 'Test Connection';

  if (response.success) {
    showStatus(connectionStatusEl, 'success', 'Connection successful!');
    await loadProjects();
  } else {
    showStatus(connectionStatusEl, 'error', `Connection failed: ${response.error}`);
  }
}

// Save connection settings
async function saveConnection() {
  if (!settings) return;

  const authMethod = getSelectedAuthMethod();

  settings.jira = {
    endpoint: endpointEl.value.trim(),
    authMethod,
    username: usernameEl.value.trim(),
    apiKey: apiKeyEl.value.trim(),
  };

  await sendMessage({ type: 'SAVE_SETTINGS', payload: settings });
  showStatus(connectionStatusEl, 'success', 'Connection settings saved!');
}

// Save sync settings
async function saveSyncSettings() {
  if (!settings) return;

  settings.sync = {
    incrementalSyncEnabled: incrementalSyncEl.checked,
    incrementalSyncMarginMinutes: parseInt(marginMinutesEl.value, 10) || 5,
    batchSize: parseInt(batchSizeEl.value, 10) || 100,
  };

  await sendMessage({ type: 'SAVE_SETTINGS', payload: settings });
  showStatus(projectsStatusEl, 'success', 'Sync settings saved!');
}

// Fetch projects from JIRA
async function fetchProjects() {
  fetchProjectsBtnEl.disabled = true;
  fetchProjectsBtnEl.innerHTML = '<span class="loading"><span class="spinner"></span>Fetching...</span>';

  const response = await sendMessage({ type: 'INIT_PROJECTS' });

  fetchProjectsBtnEl.disabled = false;
  fetchProjectsBtnEl.textContent = 'Fetch Projects from JIRA';

  if (response.success) {
    showStatus(projectsStatusEl, 'success', 'Projects loaded successfully!');
    await loadProjects();
  } else {
    showStatus(projectsStatusEl, 'error', `Failed to load projects: ${response.error}`);
  }
}

// Export database (placeholder - needs offscreen document for full implementation)
async function exportData() {
  showStatus(dataStatusEl, 'info', 'Export functionality coming soon!');
}

// Clear all data
async function clearData() {
  if (!confirm('Are you sure you want to clear all synced data? This cannot be undone.')) {
    return;
  }

  // Clear storage
  chrome.storage.local.clear(() => {
    showStatus(dataStatusEl, 'success', 'All data cleared. Please reload the extension.');
    setTimeout(() => {
      chrome.runtime.reload();
    }, 2000);
  });
}

// Message helper
async function sendMessage<T>(message: { type: string; payload?: unknown }): Promise<{
  success: boolean;
  data?: T;
  error?: string;
}> {
  return new Promise((resolve) => {
    chrome.runtime.sendMessage(message, (response) => {
      if (chrome.runtime.lastError) {
        resolve({ success: false, error: chrome.runtime.lastError.message });
      } else {
        resolve(response || { success: false, error: 'No response' });
      }
    });
  });
}

// Event listeners
testConnectionBtnEl.addEventListener('click', testConnection);
saveConnectionBtnEl.addEventListener('click', saveConnection);
saveSyncSettingsBtnEl.addEventListener('click', saveSyncSettings);
fetchProjectsBtnEl.addEventListener('click', fetchProjects);
exportDataBtnEl.addEventListener('click', exportData);
clearDataBtnEl.addEventListener('click', clearData);

// Auth method toggle
authBrowserEl.addEventListener('change', toggleApiTokenFields);
authApiTokenEl.addEventListener('change', toggleApiTokenFields);

// Initialize on load
init();
