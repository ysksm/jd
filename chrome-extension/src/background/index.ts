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
        await openAiAndPaste('claude', instructions, issueKey);
        return { success: true };
      } catch (error) {
        console.error('[Background] Failed to send to Claude:', error);
        return {
          success: false,
          error: error instanceof Error ? error.message : String(error),
        };
      }
    }

    case 'SEND_TO_CHATGPT': {
      const { instructions, issueKey } = message.payload as {
        instructions: string;
        issueKey: string;
      };
      console.log('[Background] SEND_TO_CHATGPT received for issue:', issueKey);

      try {
        await openAiAndPaste('chatgpt', instructions, issueKey);
        return { success: true };
      } catch (error) {
        console.error('[Background] Failed to send to ChatGPT:', error);
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

// AI service configurations
type AiServiceType = 'claude' | 'chatgpt';

const AI_CONFIGS: Record<AiServiceType, {
  name: string;
  url: string;
  urlPattern: string;
  selectors: string[];
}> = {
  claude: {
    name: 'Claude',
    url: 'https://claude.ai/new',
    urlPattern: 'https://claude.ai/*',
    selectors: [
      '[data-placeholder="How can Claude help you today?"]',
      '[data-placeholder]',
      'div.ProseMirror[contenteditable="true"]',
      'div[contenteditable="true"].ProseMirror',
      '.ProseMirror[contenteditable="true"]',
      'div[contenteditable="true"]',
      '[contenteditable="true"]',
      'textarea[placeholder]',
      'textarea',
    ],
  },
  chatgpt: {
    name: 'ChatGPT Codex',
    url: 'https://chatgpt.com/codex',
    urlPattern: 'https://chatgpt.com/*',
    selectors: [
      '#prompt-textarea',
      'textarea[data-id="root"]',
      'textarea[placeholder]',
      'div[contenteditable="true"]',
      '[contenteditable="true"]',
      'textarea',
    ],
  },
};

// Open AI service and paste instructions
async function openAiAndPaste(service: AiServiceType, instructions: string, issueKey: string): Promise<void> {
  const config = AI_CONFIGS[service];
  const fullPrompt = `[JIRA: ${issueKey}]\n\n${instructions}`;
  console.log(`[Background] Opening ${config.name} with prompt length:`, fullPrompt.length);

  // Store prompt for the injected script to use
  try {
    await chrome.storage.local.set({
      aiPendingPrompt: fullPrompt,
      aiPendingService: service,
      aiPendingTimestamp: Date.now(),
    });
    console.log('[Background] Stored prompt in storage');
  } catch (storageError) {
    console.error('[Background] Failed to store prompt:', storageError);
    throw storageError;
  }

  // Check if AI service is already open
  let tabs: chrome.tabs.Tab[] = [];
  try {
    tabs = await chrome.tabs.query({ url: config.urlPattern });
    console.log(`[Background] Found existing ${config.name} tabs:`, tabs.length);
  } catch (queryError) {
    console.error('[Background] Failed to query tabs:', queryError);
    throw queryError;
  }

  let targetTabId: number;

  // Always create a new tab for reliability
  // (Existing tab reuse had issues with script injection)
  {
    // Create new tab
    console.log('[Background] Creating new tab:', config.url);

    let tab: chrome.tabs.Tab;
    try {
      tab = await chrome.tabs.create({ url: config.url });
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
    console.log('[Background] Waiting for page to render...');
    await new Promise(resolve => setTimeout(resolve, 2000));
    await injectAiScript(targetTabId, service);
  }
}

// Inject script to paste into AI service
async function injectAiScript(tabId: number, service: AiServiceType): Promise<void> {
  const config = AI_CONFIGS[service];
  console.log(`[Background] Injecting script into ${config.name} tab:`, tabId);

  try {
    // We need to pass selectors to the injected script
    const selectorsJson = JSON.stringify(config.selectors);

    const results = await chrome.scripting.executeScript({
      target: { tabId },
      func: async (selectorsStr: string) => {
        const selectors = JSON.parse(selectorsStr) as string[];
        console.log('[JIRA DB] Injected script running...');

        // Get the pending prompt
        const result = await chrome.storage.local.get(['aiPendingPrompt', 'aiPendingService', 'aiPendingTimestamp']);
        const prompt = result.aiPendingPrompt;
        const timestamp = result.aiPendingTimestamp;

        if (!prompt || !timestamp || Date.now() - timestamp > 60000) {
          console.log('[JIRA DB] No pending prompt or expired');
          return { success: false, error: 'expired' };
        }

        // Clear the prompt
        await chrome.storage.local.remove(['aiPendingPrompt', 'aiPendingService', 'aiPendingTimestamp']);

        // Try selectors
        let inputEl: HTMLElement | null = null;
        for (const selector of selectors) {
          const elements = document.querySelectorAll(selector);
          console.log(`[JIRA DB] Selector "${selector}" found ${elements.length} elements`);
          if (elements.length > 0) {
            inputEl = elements[0] as HTMLElement;
            console.log('[JIRA DB] Found input with selector:', selector);
            break;
          }
        }

        if (!inputEl) {
          console.error('[JIRA DB] Could not find input element');
          console.log('[JIRA DB] Page URL:', window.location.href);
          console.log('[JIRA DB] Page content preview:', document.body.innerHTML.substring(0, 3000));
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
          const textarea = inputEl as HTMLTextAreaElement;
          // Set the value and trigger React's change detection
          const nativeInputValueSetter = Object.getOwnPropertyDescriptor(
            window.HTMLTextAreaElement.prototype,
            'value'
          )?.set;

          if (nativeInputValueSetter) {
            nativeInputValueSetter.call(textarea, prompt);
          } else {
            textarea.value = prompt;
          }

          // Dispatch events to trigger React state update
          textarea.dispatchEvent(new Event('input', { bubbles: true }));
          textarea.dispatchEvent(new Event('change', { bubbles: true }));
        } else {
          // For contenteditable - clear and insert
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
      args: [selectorsJson],
    });

    console.log('[Background] Script results:', results);

    const result = results?.[0]?.result as { success: boolean; error?: string } | undefined;
    if (result && !result.success) {
      if (result.error === 'no_input_clipboard') {
        // Prompt was copied to clipboard - notify user
        console.log('[Background] Copied to clipboard as fallback');
        // Send notification to sidepanel
        chrome.runtime.sendMessage({
          type: 'AI_CLIPBOARD_FALLBACK',
          payload: { service },
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
