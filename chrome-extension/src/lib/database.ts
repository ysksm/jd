/**
 * Database proxy for communicating with the offscreen document.
 *
 * This module provides the same API as the original database module,
 * but forwards all operations to the offscreen document where DuckDB
 * WASM can run (since service workers don't support Web Workers).
 */

import type {
  DbIssue,
  DbProject,
  DbChangeHistory,
  SearchParams,
  SearchResult,
  JiraIssue,
  JiraProject,
} from './types';

// Track offscreen document state
let creatingOffscreen = false;
let offscreenReady = false;

// Ensure offscreen document exists and is ready
async function ensureOffscreenDocument(): Promise<void> {
  // Check if offscreen document already exists and is ready
  const existingContexts = await chrome.runtime.getContexts({
    contextTypes: [chrome.runtime.ContextType.OFFSCREEN_DOCUMENT],
  });

  if (existingContexts.length > 0 && offscreenReady) {
    return;
  }

  // Prevent creating multiple offscreen documents
  if (creatingOffscreen) {
    // Wait for the existing creation to complete
    while (creatingOffscreen || !offscreenReady) {
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    return;
  }

  if (existingContexts.length === 0) {
    creatingOffscreen = true;
    try {
      await chrome.offscreen.createDocument({
        url: 'offscreen.html',
        reasons: [chrome.offscreen.Reason.WORKERS],
        justification: 'DuckDB WASM requires Web Workers which are not available in service workers',
      });
    } finally {
      creatingOffscreen = false;
    }
  }

  // Wait for the offscreen document to signal it's ready
  // by sending a ping and waiting for a pong
  let retries = 50; // 5 seconds max
  while (retries > 0) {
    try {
      const response = await new Promise<{ success: boolean; data?: string }>((resolve) => {
        chrome.runtime.sendMessage(
          { target: 'offscreen', action: 'PING' },
          (resp) => {
            if (chrome.runtime.lastError) {
              resolve({ success: false });
            } else {
              resolve(resp || { success: false });
            }
          }
        );
      });
      if (response.success && response.data === 'PONG') {
        offscreenReady = true;
        return;
      }
    } catch {
      // Ignore errors, keep retrying
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
    retries--;
  }

  throw new Error('Offscreen document failed to initialize');
}

// Send message to offscreen document
async function sendToOffscreen<T>(action: string, payload?: unknown): Promise<T> {
  await ensureOffscreenDocument();

  return new Promise((resolve, reject) => {
    chrome.runtime.sendMessage(
      {
        target: 'offscreen',
        action,
        payload,
      },
      (response) => {
        if (chrome.runtime.lastError) {
          reject(new Error(chrome.runtime.lastError.message));
        } else if (!response) {
          reject(new Error('No response from offscreen document'));
        } else if (!response.success) {
          reject(new Error(response.error || 'Unknown error'));
        } else {
          resolve(response.data as T);
        }
      }
    );
  });
}

// Database initialization (creates offscreen document if needed)
export async function initDatabase(): Promise<void> {
  await sendToOffscreen('INIT_DATABASE');
}

// Placeholder for connection (not used in proxy mode)
export async function getConnection(): Promise<unknown> {
  await initDatabase();
  return null;
}

// Project operations
export async function upsertProject(project: JiraProject): Promise<void> {
  await sendToOffscreen('UPSERT_PROJECT', project);
}

export async function getProjects(): Promise<DbProject[]> {
  return await sendToOffscreen<DbProject[]>('GET_PROJECTS');
}

// Issue operations
export async function upsertIssue(issue: JiraIssue): Promise<void> {
  await sendToOffscreen('UPSERT_ISSUE', issue);
}

export async function getIssue(key: string): Promise<DbIssue | null> {
  return await sendToOffscreen<DbIssue | null>('GET_ISSUE', { issueKey: key });
}

export async function searchIssues(params: SearchParams): Promise<SearchResult> {
  return await sendToOffscreen<SearchResult>('SEARCH_ISSUES', params);
}

export async function getIssueHistory(issueKey: string, field?: string): Promise<DbChangeHistory[]> {
  return await sendToOffscreen<DbChangeHistory[]>('GET_ISSUE_HISTORY', { issueKey, field });
}

// Get the latest updated_at for incremental sync
export async function getLatestUpdatedAt(projectKey: string): Promise<string | null> {
  return await sendToOffscreen<string | null>('GET_LATEST_UPDATED_AT', { projectKey });
}

// Get issue count for a project
export async function getIssueCount(projectKey: string): Promise<number> {
  return await sendToOffscreen<number>('GET_ISSUE_COUNT', { projectKey });
}

// Sync history operations
export async function startSyncHistory(projectKey: string): Promise<number> {
  return await sendToOffscreen<number>('START_SYNC_HISTORY', { projectKey });
}

export async function completeSyncHistory(
  id: number,
  success: boolean,
  issuesSynced: number,
  errorMessage?: string
): Promise<void> {
  await sendToOffscreen('COMPLETE_SYNC_HISTORY', { id, success, issuesSynced, errorMessage });
}

export async function updateSyncHistoryProgress(id: number, issuesSynced: number): Promise<void> {
  await sendToOffscreen('UPDATE_SYNC_HISTORY_PROGRESS', { id, issuesSynced });
}

// Get distinct statuses for a project
export async function getProjectStatuses(projectKey: string): Promise<string[]> {
  return await sendToOffscreen<string[]>('GET_PROJECT_STATUSES', { projectKey });
}

// Export database to Uint8Array for backup
export async function exportDatabase(): Promise<Uint8Array> {
  return await sendToOffscreen<Uint8Array>('EXPORT_DATABASE');
}

// Close database connection (no-op in proxy mode)
export async function closeDatabase(): Promise<void> {
  // Offscreen document manages its own lifecycle
}
