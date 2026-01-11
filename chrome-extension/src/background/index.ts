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
    message: Message & { target?: string },
    _sender: chrome.runtime.MessageSender,
    sendResponse: (response: MessageResponse) => void
  ) => {
    // Ignore messages intended for offscreen document
    if (message.target === 'offscreen') {
      return false;
    }

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
      console.log('[Background] START_SYNC received');
      // Run sync in background
      syncAllProjects((progress) => {
        console.log('[Background] Sync progress:', progress);
        // Broadcast progress to all extension pages
        chrome.runtime.sendMessage({
          type: 'SYNC_PROGRESS',
          payload: progress,
        }).catch(() => {
          // Ignore errors if no listeners
        });
      })
        .then((results) => {
          console.log('[Background] Sync complete:', results);
          chrome.runtime.sendMessage({
            type: 'SYNC_COMPLETE',
            payload: results,
          }).catch(() => {});
        })
        .catch((error) => {
          console.error('[Background] Sync error:', error);
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

    case 'SEND_TO_CLAUDE': {
      const { instructions, issueKey } = message.payload as {
        instructions: string;
        issueKey: string;
      };
      console.log('[Background] SEND_TO_CLAUDE received for issue:', issueKey);

      try {
        await openClaudeAndPaste(instructions, issueKey);
        return { success: true };
      } catch (error) {
        console.error('[Background] Failed to send to Claude:', error);
        return {
          success: false,
          error: error instanceof Error ? error.message : String(error),
        };
      }
    }

    default:
      return { success: false, error: `Unknown message type: ${message.type}` };
  }
}

// Open Claude and paste instructions
async function openClaudeAndPaste(instructions: string, issueKey: string): Promise<void> {
  const fullPrompt = `[JIRA: ${issueKey}]\n\n${instructions}`;
  console.log('[Background] Opening Claude with prompt length:', fullPrompt.length);

  // Store prompt for the injected script to use
  try {
    await chrome.storage.local.set({
      claudeCodePendingPrompt: fullPrompt,
      claudeCodeTimestamp: Date.now(),
    });
    console.log('[Background] Stored prompt in storage');
  } catch (storageError) {
    console.error('[Background] Failed to store prompt:', storageError);
    throw storageError;
  }

  // Check if Claude is already open
  let tabs: chrome.tabs.Tab[] = [];
  try {
    tabs = await chrome.tabs.query({ url: 'https://claude.ai/*' });
    console.log('[Background] Found existing Claude tabs:', tabs.length);
  } catch (queryError) {
    console.error('[Background] Failed to query tabs:', queryError);
    throw queryError;
  }

  let targetTabId: number;

  if (tabs.length > 0 && tabs[0].id) {
    // Focus existing tab
    console.log('[Background] Focusing existing tab:', tabs[0].id);
    try {
      await chrome.tabs.update(tabs[0].id, { active: true });
      targetTabId = tabs[0].id;
    } catch (updateError) {
      console.error('[Background] Failed to update tab:', updateError);
      throw updateError;
    }
    // Wait a moment then inject
    await new Promise(resolve => setTimeout(resolve, 500));
    await injectClaudeScript(targetTabId);
  } else {
    // Create new tab
    const url = 'https://claude.ai/new';
    console.log('[Background] Creating new tab:', url);

    let tab: chrome.tabs.Tab;
    try {
      tab = await chrome.tabs.create({ url });
      console.log('[Background] Tab created:', tab.id, tab.url);
    } catch (createError) {
      console.error('[Background] Failed to create tab:', createError);
      throw createError;
    }

    if (!tab.id) {
      throw new Error('Failed to create tab - no tab ID');
    }

    targetTabId = tab.id;

    // Wait for tab to load
    console.log('[Background] Waiting for tab to load...');
    await new Promise<void>((resolve) => {
      const listener = (tabId: number, info: chrome.tabs.TabChangeInfo) => {
        console.log('[Background] Tab updated:', tabId, info.status);
        if (tabId === targetTabId && info.status === 'complete') {
          chrome.tabs.onUpdated.removeListener(listener);
          resolve();
        }
      };
      chrome.tabs.onUpdated.addListener(listener);

      // Timeout after 10 seconds
      setTimeout(() => {
        console.log('[Background] Tab load timeout');
        chrome.tabs.onUpdated.removeListener(listener);
        resolve();
      }, 10000);
    });

    // Wait for React to render
    console.log('[Background] Waiting for React to render...');
    await new Promise(resolve => setTimeout(resolve, 2000));
    await injectClaudeScript(targetTabId);
  }
}

// Inject script to paste into Claude
async function injectClaudeScript(tabId: number): Promise<void> {
  console.log('[Background] Injecting script into tab:', tabId);

  try {
    const results = await chrome.scripting.executeScript({
      target: { tabId },
      func: async () => {
        console.log('[JIRA DB] Injected script running in Claude...');

        // Get the pending prompt
        const result = await chrome.storage.local.get(['claudeCodePendingPrompt', 'claudeCodeTimestamp']);
        const prompt = result.claudeCodePendingPrompt;
        const timestamp = result.claudeCodeTimestamp;

        if (!prompt || !timestamp || Date.now() - timestamp > 60000) {
          console.log('[JIRA DB] No pending prompt or expired');
          return { success: false, error: 'expired' };
        }

        // Clear the prompt
        await chrome.storage.local.remove(['claudeCodePendingPrompt', 'claudeCodeTimestamp']);

        // Try multiple selectors for Claude's input (Claude UI changes frequently)
        const selectors = [
          // Claude's main input area
          '[data-placeholder="How can Claude help you today?"]',
          '[data-placeholder]',
          // ProseMirror editor
          'div.ProseMirror[contenteditable="true"]',
          'div[contenteditable="true"].ProseMirror',
          '.ProseMirror[contenteditable="true"]',
          // Generic contenteditable
          'div[contenteditable="true"]',
          '[contenteditable="true"]',
          // Fallback to textarea
          'textarea[placeholder]',
          'textarea',
          // Any input-like element in the chat area
          'form div[contenteditable]',
          'main div[contenteditable]',
        ];

        let inputEl: HTMLElement | null = null;
        for (const selector of selectors) {
          const elements = document.querySelectorAll(selector);
          console.log(`[JIRA DB] Selector "${selector}" found ${elements.length} elements`);
          if (elements.length > 0) {
            inputEl = elements[0] as HTMLElement;
            console.log('[JIRA DB] Found input with selector:', selector, inputEl);
            break;
          }
        }

        if (!inputEl) {
          console.error('[JIRA DB] Could not find input element');
          console.log('[JIRA DB] Page content:', document.body.innerHTML.substring(0, 2000));
          // Try to copy to clipboard as fallback
          try {
            await navigator.clipboard.writeText(prompt);
            return { success: false, error: 'no_input_clipboard' };
          } catch {
            return { success: false, error: 'no_input' };
          }
        }

        // Focus and paste
        inputEl.focus();

        if (inputEl.tagName === 'TEXTAREA') {
          (inputEl as HTMLTextAreaElement).value = prompt;
          inputEl.dispatchEvent(new Event('input', { bubbles: true }));
        } else {
          // For contenteditable - clear and insert
          // Select all existing content first
          const selection = window.getSelection();
          const range = document.createRange();
          range.selectNodeContents(inputEl);
          selection?.removeAllRanges();
          selection?.addRange(range);

          // Insert the text
          document.execCommand('insertText', false, prompt);
        }

        console.log('[JIRA DB] Prompt pasted successfully');
        return { success: true };
      },
    });

    console.log('[Background] Script results:', results);

    const result = results?.[0]?.result as { success: boolean; error?: string } | undefined;
    if (result && !result.success) {
      if (result.error === 'no_input_clipboard') {
        // Prompt was copied to clipboard - notify user
        console.log('[Background] Copied to clipboard as fallback');
        // Send notification to sidepanel
        chrome.runtime.sendMessage({
          type: 'CLAUDE_CLIPBOARD_FALLBACK',
        }).catch(() => {});
      }
    }
  } catch (error) {
    console.error('[Background] Script injection failed:', error);
    throw error;
  }
}

// Database is initialized on-demand via offscreen document
// No need to initialize here

console.log('JIRA DB Sync background service started');
