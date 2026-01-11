import type { Settings, ProjectConfig, SyncCheckpoint } from './types';

const SETTINGS_KEY = 'jira_db_settings';

const DEFAULT_SETTINGS: Settings = {
  jira: {
    endpoint: '',
    username: '',
    apiKey: '',
  },
  sync: {
    incrementalSyncEnabled: true,
    incrementalSyncMarginMinutes: 5,
    batchSize: 100,
  },
  projects: [],
};

// Load settings from Chrome storage
export async function loadSettings(): Promise<Settings> {
  return new Promise((resolve) => {
    chrome.storage.local.get([SETTINGS_KEY], (result: { [key: string]: unknown }) => {
      const stored = result[SETTINGS_KEY] as Partial<Settings> | undefined;
      if (stored) {
        // Merge with defaults to ensure all fields exist
        const settings: Settings = {
          ...DEFAULT_SETTINGS,
          ...stored,
          jira: {
            ...DEFAULT_SETTINGS.jira,
            ...(stored.jira || {}),
          },
          sync: {
            ...DEFAULT_SETTINGS.sync,
            ...(stored.sync || {}),
          },
          projects: stored.projects || [],
        };
        resolve(settings);
      } else {
        resolve(DEFAULT_SETTINGS);
      }
    });
  });
}

// Save settings to Chrome storage
export async function saveSettings(settings: Settings): Promise<void> {
  return new Promise((resolve, reject) => {
    chrome.storage.local.set({ [SETTINGS_KEY]: settings }, () => {
      if (chrome.runtime.lastError) {
        reject(new Error(chrome.runtime.lastError.message));
      } else {
        resolve();
      }
    });
  });
}

// Update specific project settings
export async function updateProjectConfig(
  projectKey: string,
  updates: Partial<ProjectConfig>
): Promise<void> {
  const settings = await loadSettings();
  const projectIndex = settings.projects.findIndex((p) => p.key === projectKey);

  if (projectIndex >= 0) {
    settings.projects[projectIndex] = {
      ...settings.projects[projectIndex],
      ...updates,
    };
  }

  await saveSettings(settings);
}

// Save sync checkpoint for resume support
export async function saveSyncCheckpoint(
  projectKey: string,
  checkpoint: SyncCheckpoint
): Promise<void> {
  await updateProjectConfig(projectKey, { syncCheckpoint: checkpoint });
}

// Clear sync checkpoint after successful sync
export async function clearSyncCheckpoint(projectKey: string): Promise<void> {
  const settings = await loadSettings();
  const projectIndex = settings.projects.findIndex((p) => p.key === projectKey);

  if (projectIndex >= 0) {
    delete settings.projects[projectIndex].syncCheckpoint;
    settings.projects[projectIndex].lastSyncedAt = new Date().toISOString();
  }

  await saveSettings(settings);
}

// Get sync checkpoint if exists
export async function getSyncCheckpoint(
  projectKey: string
): Promise<SyncCheckpoint | undefined> {
  const settings = await loadSettings();
  const project = settings.projects.find((p) => p.key === projectKey);
  return project?.syncCheckpoint;
}

// Add or update project in settings
export async function upsertProjectInSettings(
  project: { key: string; name: string },
  enabled: boolean = false
): Promise<void> {
  const settings = await loadSettings();
  const existingIndex = settings.projects.findIndex((p) => p.key === project.key);

  if (existingIndex >= 0) {
    settings.projects[existingIndex].name = project.name;
  } else {
    settings.projects.push({
      key: project.key,
      name: project.name,
      enabled,
    });
  }

  await saveSettings(settings);
}

// Enable or disable project sync
export async function setProjectEnabled(
  projectKey: string,
  enabled: boolean
): Promise<void> {
  await updateProjectConfig(projectKey, { enabled });
}

// Get enabled projects
export async function getEnabledProjects(): Promise<ProjectConfig[]> {
  const settings = await loadSettings();
  return settings.projects.filter((p) => p.enabled);
}

// Check if JIRA is configured
export async function isJiraConfigured(): Promise<boolean> {
  const settings = await loadSettings();
  return !!(
    settings.jira.endpoint &&
    settings.jira.username &&
    settings.jira.apiKey
  );
}
