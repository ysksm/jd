/**
 * AI instruction extraction utilities
 * Supports multiple AI services (Claude, ChatGPT Codex, etc.)
 */

// Extract instructions from JIRA description
// Supports format: ```ai\n...\n``` (generic AI instruction block)
export function extractAiInstructions(description: string | null): string | null {
  if (!description) return null;

  // Match ```ai ... ``` blocks (case insensitive)
  const codeBlockRegex = /```ai\s*\n([\s\S]*?)```/gi;
  const matches: string[] = [];

  let match;
  while ((match = codeBlockRegex.exec(description)) !== null) {
    matches.push(match[1].trim());
  }

  if (matches.length === 0) return null;

  // Join multiple blocks with newlines
  return matches.join('\n\n');
}

// AI service types
export type AiService = 'claude' | 'chatgpt-codex';

// AI service configurations
export const AI_SERVICES: Record<AiService, {
  name: string;
  url: string;
  urlPattern: string;
}> = {
  'claude': {
    name: 'Claude',
    url: 'https://claude.ai/new',
    urlPattern: 'https://claude.ai/*',
  },
  'chatgpt-codex': {
    name: 'ChatGPT Codex',
    url: 'https://chatgpt.com/codex',
    urlPattern: 'https://chatgpt.com/*',
  },
};

// Format for documentation
export const INSTRUCTION_FORMAT = `
## AI Instructions Format

To include instructions for AI assistants in your JIRA ticket,
use the following format in the description:

\`\`\`ai
Your instructions here.
This can be multiple lines.
\`\`\`

You can have multiple instruction blocks in a single ticket.
They will be combined when sent to the AI assistant.

Supported AI services:
- Claude (claude.ai)
- ChatGPT Codex (chatgpt.com/codex)
`;
