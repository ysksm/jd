import type {
  Message,
  MessageResponse,
  Settings,
  SearchParams,
} from '../lib/types';
import {
  loadSettings,
  saveSettings,
  setProjectEnabled,
} from '../lib/settings';
import {
  initProjects,
  syncAllProjects,
  getSyncStatus,
  cancelSync,
  getProjectsWithStatus,
} from '../lib/sync-service';
import {
  initDatabase,
  searchIssues,
  getIssue,
  getIssueHistory,
} from '../lib/database';

// Listen for messages from popup/options
chrome.runtime.onMessage.addListener(
  (
    message: Message,
    _sender: chrome.runtime.MessageSender,
    sendResponse: (response: MessageResponse) => void
  ) => {
    handleMessage(message)
      .then((response) => sendResponse(response))
      .catch((error) => {
        console.error('Message handler error:', error);
        sendResponse({
          success: false,
          error: error instanceof Error ? error.message : String(error),
        });
      });

    // Return true to indicate async response
    return true;
  }
);

async function handleMessage(message: Message): Promise<MessageResponse> {
  switch (message.type) {
    case 'GET_SETTINGS': {
      const settings = await loadSettings();
      return { success: true, data: settings };
    }

    case 'SAVE_SETTINGS': {
      await saveSettings(message.payload as Settings);
      return { success: true };
    }

    case 'INIT_PROJECTS': {
      await initProjects();
      const projects = await getProjectsWithStatus();
      return { success: true, data: projects };
    }

    case 'GET_PROJECTS': {
      const projects = await getProjectsWithStatus();
      return { success: true, data: projects };
    }

    case 'ENABLE_PROJECT': {
      const { projectKey, enabled } = message.payload as {
        projectKey: string;
        enabled: boolean;
      };
      await setProjectEnabled(projectKey, enabled);
      return { success: true };
    }

    case 'DISABLE_PROJECT': {
      const { projectKey } = message.payload as { projectKey: string };
      await setProjectEnabled(projectKey, false);
      return { success: true };
    }

    case 'START_SYNC': {
      // Run sync in background
      syncAllProjects((progress) => {
        // Broadcast progress to all extension pages
        chrome.runtime.sendMessage({
          type: 'SYNC_PROGRESS',
          payload: progress,
        }).catch(() => {
          // Ignore errors if no listeners
        });
      })
        .then((results) => {
          chrome.runtime.sendMessage({
            type: 'SYNC_COMPLETE',
            payload: results,
          }).catch(() => {});
        })
        .catch((error) => {
          chrome.runtime.sendMessage({
            type: 'SYNC_ERROR',
            payload: error instanceof Error ? error.message : String(error),
          }).catch(() => {});
        });

      return { success: true, data: { started: true } };
    }

    case 'GET_SYNC_STATUS': {
      const status = getSyncStatus();
      return { success: true, data: status };
    }

    case 'CANCEL_SYNC': {
      cancelSync();
      return { success: true };
    }

    case 'SEARCH_ISSUES': {
      await initDatabase();
      const params = message.payload as SearchParams;
      const result = await searchIssues(params);
      return { success: true, data: result };
    }

    case 'GET_ISSUE': {
      await initDatabase();
      const { issueKey } = message.payload as { issueKey: string };
      const issue = await getIssue(issueKey);
      return { success: true, data: issue };
    }

    case 'GET_ISSUE_HISTORY': {
      await initDatabase();
      const { issueKey, field } = message.payload as {
        issueKey: string;
        field?: string;
      };
      const history = await getIssueHistory(issueKey, field);
      return { success: true, data: history };
    }

    default:
      return { success: false, error: `Unknown message type: ${message.type}` };
  }
}

// Initialize database on extension load
initDatabase().catch(console.error);

console.log('JIRA DB Sync background service started');
