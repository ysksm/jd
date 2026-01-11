import { JiraClient } from './jira-client';
import {
  initDatabase,
  upsertIssue,
  upsertProject,
  getLatestUpdatedAt,
  startSyncHistory,
  completeSyncHistory,
  updateSyncHistoryProgress,
  persistDatabase,
} from './database';
import {
  loadSettings,
  saveSyncCheckpoint,
  clearSyncCheckpoint,
  getSyncCheckpoint,
  upsertProjectInSettings,
} from './settings';
import type {
  SyncProgress,
  SyncResult,
  SyncCheckpoint,
  ProjectConfig,
} from './types';

// Sync state
let isSyncing = false;
let cancelRequested = false;
let currentSyncProgress: SyncProgress | null = null;

export function getSyncStatus(): { isSyncing: boolean; progress: SyncProgress | null } {
  return { isSyncing, progress: currentSyncProgress };
}

export function cancelSync(): void {
  cancelRequested = true;
}

// Initialize projects from JIRA
export async function initProjects(): Promise<void> {
  const settings = await loadSettings();
  const client = new JiraClient(settings.jira);

  const jiraProjects = await client.getProjects();

  // Add all projects to settings (this is the primary goal)
  for (const project of jiraProjects) {
    await upsertProjectInSettings({ key: project.key, name: project.name });
  }

  // Also try to add to database, but don't fail if database isn't ready
  try {
    await initDatabase();
    for (const project of jiraProjects) {
      await upsertProject(project);
    }
  } catch (error) {
    console.warn('Could not save projects to database (will be saved during sync):', error);
    // This is not a fatal error - projects are saved to settings
  }
}

// Sync a single project
export async function syncProject(
  projectKey: string,
  onProgress?: (progress: SyncProgress) => void
): Promise<SyncResult> {
  console.log(`[SyncService] syncProject started for ${projectKey}`);
  const startedAt = new Date().toISOString();
  let issuesSynced = 0;
  let syncHistoryId = 0;

  try {
    const settings = await loadSettings();
    console.log(`[SyncService] Settings loaded, creating JIRA client`);
    const client = new JiraClient(settings.jira);

    console.log(`[SyncService] Initializing database...`);
    await initDatabase();
    console.log(`[SyncService] Database initialized`);

    // Start sync history record
    syncHistoryId = await startSyncHistory(projectKey);
    console.log(`[SyncService] Sync history started with ID ${syncHistoryId}`);

    // Check for existing checkpoint (resume support)
    const checkpoint = await getSyncCheckpoint(projectKey);
    let startPosition = 0;
    let lastProcessedUpdatedAt: string | undefined;

    if (checkpoint) {
      startPosition = checkpoint.startPosition;
      lastProcessedUpdatedAt = checkpoint.lastProcessedUpdatedAt;
      console.log(`Resuming sync for ${projectKey} from position ${startPosition}`);
    }

    // Determine if we should do incremental sync
    let updatedSince: string | undefined;
    if (settings.sync.incrementalSyncEnabled && !checkpoint) {
      const latestInDb = await getLatestUpdatedAt(projectKey);
      if (latestInDb) {
        // Apply safety margin
        const marginMs = settings.sync.incrementalSyncMarginMinutes * 60 * 1000;
        const date = new Date(new Date(latestInDb).getTime() - marginMs);
        updatedSince = date.toISOString();
        console.log(`Incremental sync from ${updatedSince}`);
      }
    } else if (checkpoint) {
      // Use checkpoint's last processed date for resume
      updatedSince = lastProcessedUpdatedAt;
    }

    // Fetch and sync issues
    let totalIssues = 0;
    const generator = checkpoint
      ? client.getIssuesFromCheckpoint(
          projectKey,
          startPosition,
          updatedSince,
          (current, total) => {
            totalIssues = total;
            currentSyncProgress = {
              projectKey,
              phase: 'issues',
              current,
              total,
              message: `Syncing issues: ${current}/${total}`,
            };
            onProgress?.(currentSyncProgress);
          }
        )
      : client.getAllIssues(projectKey, updatedSince, (current, total) => {
          totalIssues = total;
          currentSyncProgress = {
            projectKey,
            phase: 'issues',
            current,
            total,
            message: `Syncing issues: ${current}/${total}`,
          };
          onProgress?.(currentSyncProgress);
        });

    for await (const batch of generator) {
      if (cancelRequested) {
        // Save checkpoint before cancelling
        const lastIssue = batch[batch.length - 1];
        if (lastIssue) {
          await saveSyncCheckpoint(projectKey, {
            lastProcessedUpdatedAt: lastIssue.fields.updated,
            startPosition: issuesSynced + batch.length,
            totalIssues,
          });
        }
        throw new Error('Sync cancelled by user');
      }

      // Process batch
      for (const issue of batch) {
        await upsertIssue(issue);
        issuesSynced++;
      }

      // Save checkpoint after each batch
      const lastIssue = batch[batch.length - 1];
      if (lastIssue) {
        const newCheckpoint: SyncCheckpoint = {
          lastProcessedUpdatedAt: lastIssue.fields.updated,
          startPosition: issuesSynced,
          totalIssues,
        };
        await saveSyncCheckpoint(projectKey, newCheckpoint);
      }

      // Update sync history progress
      await updateSyncHistoryProgress(syncHistoryId, issuesSynced);
    }

    // Sync completed successfully - clear checkpoint
    await clearSyncCheckpoint(projectKey);
    await completeSyncHistory(syncHistoryId, true, issuesSynced);

    // Persist database to IndexedDB for next session
    console.log(`[SyncService] Persisting database...`);
    try {
      await persistDatabase();
      console.log(`[SyncService] Database persisted successfully`);
    } catch (persistError) {
      console.error(`[SyncService] Failed to persist database:`, persistError);
      // Don't fail the sync if persistence fails
    }

    return {
      projectKey,
      issuesSynced,
      issuesTotalInJira: totalIssues,
      startedAt,
      completedAt: new Date().toISOString(),
      success: true,
    };
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    console.error(`[SyncService] syncProject error for ${projectKey}:`, errorMessage, error);
    if (syncHistoryId) {
      try {
        await completeSyncHistory(syncHistoryId, false, issuesSynced, errorMessage);
      } catch (e) {
        console.error(`[SyncService] Failed to complete sync history:`, e);
      }
    }
    return {
      projectKey,
      issuesSynced,
      issuesTotalInJira: 0,
      startedAt,
      completedAt: new Date().toISOString(),
      success: false,
      errorMessage,
    };
  }
}

// Sync all enabled projects
export async function syncAllProjects(
  onProgress?: (progress: SyncProgress) => void
): Promise<SyncResult[]> {
  console.log('[SyncService] syncAllProjects called');

  if (isSyncing) {
    console.log('[SyncService] Sync already in progress');
    throw new Error('Sync is already in progress');
  }

  isSyncing = true;
  cancelRequested = false;
  const results: SyncResult[] = [];

  try {
    const settings = await loadSettings();
    const enabledProjects = settings.projects.filter((p) => p.enabled);
    console.log('[SyncService] Enabled projects:', enabledProjects.map(p => p.key));

    if (enabledProjects.length === 0) {
      console.log('[SyncService] No projects enabled for sync');
      throw new Error('No projects enabled for sync');
    }

    for (const project of enabledProjects) {
      if (cancelRequested) {
        console.log('[SyncService] Sync cancelled');
        break;
      }

      console.log('[SyncService] Syncing project:', project.key);
      const result = await syncProject(project.key, onProgress);
      console.log('[SyncService] Project sync result:', JSON.stringify(result, null, 2));
      results.push(result);
    }

    return results;
  } finally {
    isSyncing = false;
    currentSyncProgress = null;
  }
}

// Get projects with sync status
export async function getProjectsWithStatus(): Promise<
  (ProjectConfig & { issueCount?: number; hasCheckpoint?: boolean })[]
> {
  const settings = await loadSettings();

  // Try to get issue counts from database, but don't fail if database isn't ready
  let issueCountMap: Map<string, number> = new Map();
  try {
    const { getIssueCount } = await import('./database');
    await initDatabase();
    for (const project of settings.projects) {
      try {
        const count = await getIssueCount(project.key);
        issueCountMap.set(project.key, count);
      } catch {
        // Ignore individual project errors
      }
    }
  } catch (error) {
    console.warn('Could not load issue counts from database:', error);
    // Continue without issue counts
  }

  return settings.projects.map((project) => ({
    ...project,
    issueCount: issueCountMap.get(project.key),
    hasCheckpoint: !!project.syncCheckpoint,
  }));
}
