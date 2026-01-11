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
  // Create the full prompt with context
  const fullPrompt = `[JIRA: ${issueKey}]\n\n${instructions}`;

  // Store in chrome.storage for the content script to pick up
  await chrome.storage.local.set({
    claudeCodePendingPrompt: fullPrompt,
    claudeCodeTimestamp: Date.now(),
  });

  // Open Claude Code in a new tab
  const url = 'https://claude.ai/code';

  // Check if Claude Code is already open
  const tabs = await chrome.tabs.query({ url: 'https://claude.ai/code*' });

  if (tabs.length > 0 && tabs[0].id) {
    // Focus existing tab
    await chrome.tabs.update(tabs[0].id, { active: true });
    // Inject script to paste
    await injectPasteScript(tabs[0].id);
  } else {
    // Create new tab
    const tab = await chrome.tabs.create({ url });
    // Listen for tab to finish loading, then inject
    chrome.tabs.onUpdated.addListener(function listener(tabId, info) {
      if (tabId === tab.id && info.status === 'complete') {
        chrome.tabs.onUpdated.removeListener(listener);
        setTimeout(() => injectPasteScript(tabId), 1000); // Wait for React to render
      }
    });
  }
}

// Inject script to paste into Claude Code textarea
async function injectPasteScript(tabId: number): Promise<void> {
  try {
    await chrome.scripting.executeScript({
      target: { tabId },
      func: async () => {
        // Get the pending prompt from storage
        const result = await chrome.storage.local.get(['claudeCodePendingPrompt', 'claudeCodeTimestamp']);
        const prompt = result.claudeCodePendingPrompt;
        const timestamp = result.claudeCodeTimestamp;

        // Only use if recent (within 30 seconds)
        if (!prompt || !timestamp || Date.now() - timestamp > 30000) {
          console.log('[JIRA DB] No pending prompt or expired');
          return;
        }

        // Clear the pending prompt
        await chrome.storage.local.remove(['claudeCodePendingPrompt', 'claudeCodeTimestamp']);

        // Find the textarea
        const textarea = document.querySelector('textarea[placeholder*="Claude"]') as HTMLTextAreaElement;
        if (!textarea) {
          console.error('[JIRA DB] Could not find Claude Code textarea');
          alert('Could not find Claude Code input. Please paste manually.');
          // Copy to clipboard as fallback
          await navigator.clipboard.writeText(prompt);
          return;
        }

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

        // Focus the textarea
        textarea.focus();

        console.log('[JIRA DB] Prompt pasted successfully');
      },
    });
  } catch (error) {
    console.error('[JIRA DB] Failed to inject paste script:', error);
    // Fallback: copy to clipboard
    const result = await chrome.storage.local.get(['claudeCodePendingPrompt']);
    if (result.claudeCodePendingPrompt) {
      await navigator.clipboard.writeText(result.claudeCodePendingPrompt);
      alert('Copied to clipboard. Please paste into Claude Code manually.');
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
