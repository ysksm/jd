/**
 * Claude Code integration utilities
 */

// Extract instructions from JIRA description
// Supports format: ```claude\n...\n```
export function extractClaudeInstructions(description: string | null): string | null {
  if (!description) return null;

  // Match ```claude ... ``` blocks
  const codeBlockRegex = /```claude\s*\n([\s\S]*?)```/gi;
  const matches: string[] = [];

  let match;
  while ((match = codeBlockRegex.exec(description)) !== null) {
    matches.push(match[1].trim());
  }

  if (matches.length === 0) return null;

  // Join multiple blocks with newlines
  return matches.join('\n\n');
}

// Open Claude Code and paste instructions
export async function sendToClaudeCode(instructions: string, issueKey: string): Promise<void> {
  console.log('[Claude Code] sendToClaudeCode called for issue:', issueKey);

  // Create the full prompt with context
  const fullPrompt = `[JIRA: ${issueKey}]\n\n${instructions}`;
  console.log('[Claude Code] Prompt length:', fullPrompt.length);

  // Store in chrome.storage for the content script to pick up
  await chrome.storage.local.set({
    claudeCodePendingPrompt: fullPrompt,
    claudeCodeTimestamp: Date.now(),
  });
  console.log('[Claude Code] Stored prompt in chrome.storage');

  // Open Claude Code in a new tab
  // Note: claude.ai/code redirects to claude.ai/new or similar
  const url = 'https://claude.ai/new';

  // Check if Claude is already open
  const tabs = await chrome.tabs.query({ url: 'https://claude.ai/*' });
  console.log('[Claude Code] Found existing Claude tabs:', tabs.length);

  if (tabs.length > 0 && tabs[0].id) {
    // Focus existing tab
    console.log('[Claude Code] Focusing existing tab:', tabs[0].id, 'URL:', tabs[0].url);
    await chrome.tabs.update(tabs[0].id, { active: true });
    // Inject script to paste
    await injectPasteScript(tabs[0].id);
  } else {
    // Create new tab
    console.log('[Claude Code] Creating new tab with URL:', url);
    const tab = await chrome.tabs.create({ url });
    console.log('[Claude Code] Created tab:', tab.id);
    // Listen for tab to finish loading, then inject
    chrome.tabs.onUpdated.addListener(function listener(tabId, info) {
      if (tabId === tab.id && info.status === 'complete') {
        console.log('[Claude Code] Tab loaded, injecting script in 2 seconds');
        chrome.tabs.onUpdated.removeListener(listener);
        setTimeout(() => injectPasteScript(tabId), 2000); // Wait for React to render
      }
    });
  }
}

// Inject script to paste into Claude textarea
async function injectPasteScript(tabId: number): Promise<void> {
  console.log('[Claude Code] injectPasteScript called for tab:', tabId);

  try {
    const results = await chrome.scripting.executeScript({
      target: { tabId },
      func: async () => {
        console.log('[JIRA DB] Injected script running...');

        // Get the pending prompt from storage
        const result = await chrome.storage.local.get(['claudeCodePendingPrompt', 'claudeCodeTimestamp']);
        const prompt = result.claudeCodePendingPrompt;
        const timestamp = result.claudeCodeTimestamp;

        console.log('[JIRA DB] Retrieved prompt:', prompt ? `${prompt.length} chars` : 'none', 'timestamp:', timestamp);

        // Only use if recent (within 60 seconds)
        if (!prompt || !timestamp || Date.now() - timestamp > 60000) {
          console.log('[JIRA DB] No pending prompt or expired');
          return { success: false, error: 'No pending prompt or expired' };
        }

        // Clear the pending prompt
        await chrome.storage.local.remove(['claudeCodePendingPrompt', 'claudeCodeTimestamp']);

        // Try multiple selectors to find the textarea
        const selectors = [
          'div[contenteditable="true"]',  // Claude uses contenteditable
          'textarea',
          'textarea[placeholder]',
          '[data-placeholder]',
          '.ProseMirror',  // Claude uses ProseMirror
        ];

        let inputEl: HTMLElement | null = null;
        for (const selector of selectors) {
          inputEl = document.querySelector(selector);
          if (inputEl) {
            console.log('[JIRA DB] Found input element with selector:', selector);
            break;
          }
        }

        if (!inputEl) {
          console.error('[JIRA DB] Could not find Claude input element');
          console.log('[JIRA DB] Available elements:', document.body.innerHTML.substring(0, 1000));
          // Copy to clipboard as fallback
          await navigator.clipboard.writeText(prompt);
          return { success: false, error: 'Could not find input element - copied to clipboard' };
        }

        // Check if it's a contenteditable div or textarea
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
          textarea.focus();
        } else {
          // For contenteditable divs
          inputEl.focus();
          // Use execCommand for contenteditable (deprecated but works)
          document.execCommand('selectAll', false);
          document.execCommand('insertText', false, prompt);
        }

        console.log('[JIRA DB] Prompt pasted successfully');
        return { success: true };
      },
    });

    console.log('[Claude Code] Script execution results:', results);

    // Check if we need to show a fallback message
    const result = results?.[0]?.result as { success: boolean; error?: string } | undefined;
    if (result && !result.success && result.error?.includes('copied to clipboard')) {
      alert('Copied to clipboard. Please paste into Claude manually (Ctrl/Cmd+V).');
    }
  } catch (error) {
    console.error('[Claude Code] Failed to inject paste script:', error);
    // Fallback: copy to clipboard
    try {
      const result = await chrome.storage.local.get(['claudeCodePendingPrompt']);
      if (result.claudeCodePendingPrompt) {
        // We can't write to clipboard from the service worker, so show instructions
        alert('Could not paste automatically. The prompt is stored - try opening Claude and pasting manually.');
      }
    } catch (e) {
      console.error('[Claude Code] Fallback also failed:', e);
    }
  }
}

// Format for documentation
export const INSTRUCTION_FORMAT = `
## Claude Code Instructions Format

To include instructions for Claude Code in your JIRA ticket,
use the following format in the description:

\`\`\`claude
Your instructions here.
This can be multiple lines.
\`\`\`

You can have multiple instruction blocks in a single ticket.
They will be combined when sent to Claude Code.
`;
