import type {
  DbIssue,
  SearchResult,
  SyncProgress,
  ProjectConfig,
  DbChangeHistory,
  Settings,
} from '../lib/types';
import { extractClaudeInstructions, sendToClaudeCode } from '../lib/claude-code';

// State
let currentPage = 0;
const pageSize = 20;
let totalResults = 0;
let currentIssue: DbIssue | null = null;
let settings: Settings | null = null;
let projects: (ProjectConfig & { issueCount?: number })[] = [];

// DOM elements
const notConfiguredEl = document.getElementById('notConfigured')!;
const mainContentEl = document.getElementById('mainContent')!;
const syncStatusEl = document.getElementById('syncStatus')!;
const syncMessageEl = document.getElementById('syncMessage')!;
const progressBarEl = document.getElementById('progressBar')!;
const searchInputEl = document.getElementById('searchInput') as HTMLInputElement;
const projectFilterEl = document.getElementById('projectFilter') as HTMLSelectElement;
const statusFilterEl = document.getElementById('statusFilter') as HTMLSelectElement;
const resultsCountEl = document.getElementById('resultsCount')!;
const issueListEl = document.getElementById('issueList')!;
const emptyStateEl = document.getElementById('emptyState')!;
const paginationEl = document.getElementById('pagination')!;
const prevBtnEl = document.getElementById('prevBtn') as HTMLButtonElement;
const nextBtnEl = document.getElementById('nextBtn') as HTMLButtonElement;
const pageInfoEl = document.getElementById('pageInfo')!;
const issueDetailEl = document.getElementById('issueDetail')!;
const detailTitleEl = document.getElementById('detailTitle')!;
const detailBodyEl = document.getElementById('detailBody')!;
const syncBtnEl = document.getElementById('syncBtn') as HTMLButtonElement;
const settingsBtnEl = document.getElementById('settingsBtn') as HTMLButtonElement;
const openSettingsBtnEl = document.getElementById('openSettingsBtn') as HTMLButtonElement;
const backBtnEl = document.getElementById('backBtn') as HTMLButtonElement;
const openInJiraBtnEl = document.getElementById('openInJiraBtn') as HTMLButtonElement;
const cancelSyncBtnEl = document.getElementById('cancelSyncBtn') as HTMLButtonElement;

// Initialize
async function init() {
  // Load settings
  const response = await sendMessage<Settings>({ type: 'GET_SETTINGS' });
  if (response.success && response.data) {
    settings = response.data;

    // Check if configured
    const isBrowserAuth = settings.jira.authMethod === 'browser';
    const isConfigured = settings.jira.endpoint && (
      isBrowserAuth || (settings.jira.username && settings.jira.apiKey)
    );

    if (!isConfigured) {
      showNotConfigured();
      return;
    }

    // Load projects
    await loadProjects();

    // Show main content
    showMainContent();

    // Initial search
    await search();

    // Check sync status
    await checkSyncStatus();
  }
}

function showNotConfigured() {
  notConfiguredEl.style.display = 'flex';
  mainContentEl.style.display = 'none';
}

function showMainContent() {
  notConfiguredEl.style.display = 'none';
  mainContentEl.style.display = 'flex';
  issueDetailEl.style.display = 'none';
}

async function loadProjects() {
  const response = await sendMessage<(ProjectConfig & { issueCount?: number })[]>({
    type: 'GET_PROJECTS',
  });

  if (response.success && response.data) {
    projects = response.data;
    updateProjectFilter();
  }
}

function updateProjectFilter() {
  projectFilterEl.innerHTML = '<option value="">All Projects</option>';
  for (const project of projects) {
    if (project.enabled) {
      const option = document.createElement('option');
      option.value = project.key;
      option.textContent = `${project.key} (${project.issueCount || 0})`;
      projectFilterEl.appendChild(option);
    }
  }
}

async function search() {
  const query = searchInputEl.value.trim();
  const project = projectFilterEl.value;
  const status = statusFilterEl.value;

  const response = await sendMessage<SearchResult>({
    type: 'SEARCH_ISSUES',
    payload: {
      query: query || undefined,
      project: project || undefined,
      status: status || undefined,
      limit: pageSize,
      offset: currentPage * pageSize,
    },
  });

  if (response.success && response.data) {
    totalResults = response.data.total;
    renderIssues(response.data.issues);
    updatePagination();

    // Update status filter with available statuses
    if (response.data.issues.length > 0 && statusFilterEl.options.length <= 1) {
      updateStatusFilter(response.data.issues);
    }
  }
}

function updateStatusFilter(issues: DbIssue[]) {
  const statuses = [...new Set(issues.map((i) => i.status))].sort();
  const currentValue = statusFilterEl.value;

  statusFilterEl.innerHTML = '<option value="">All Statuses</option>';
  for (const status of statuses) {
    const option = document.createElement('option');
    option.value = status;
    option.textContent = status;
    statusFilterEl.appendChild(option);
  }

  statusFilterEl.value = currentValue;
}

function renderIssues(issues: DbIssue[]) {
  resultsCountEl.textContent = `${totalResults} issue${totalResults !== 1 ? 's' : ''}`;

  if (issues.length === 0) {
    issueListEl.style.display = 'none';
    emptyStateEl.style.display = 'flex';
    return;
  }

  issueListEl.style.display = 'block';
  emptyStateEl.style.display = 'none';

  issueListEl.innerHTML = issues
    .map((issue) => {
      const statusClass = getStatusClass(issue.status_category);
      const updatedAt = formatDate(issue.updated_at);

      return `
        <div class="issue-item" data-key="${issue.key}">
          <div class="issue-content">
            <div class="issue-key">
              <span class="issue-key-text">${escapeHtml(issue.key)}</span>
              <span class="issue-status ${statusClass}">${escapeHtml(issue.status)}</span>
            </div>
            <div class="issue-summary">${escapeHtml(issue.summary)}</div>
            <div class="issue-meta">
              <span>${escapeHtml(issue.issue_type)}</span>
              ${issue.assignee_name ? `<span>${escapeHtml(issue.assignee_name)}</span>` : ''}
              <span>${updatedAt}</span>
            </div>
          </div>
        </div>
      `;
    })
    .join('');

  // Add click handlers
  issueListEl.querySelectorAll('.issue-item').forEach((el) => {
    el.addEventListener('click', () => {
      const key = el.getAttribute('data-key');
      if (key) showIssueDetail(key);
    });
  });
}

function getStatusClass(statusCategory: string | null): string {
  if (!statusCategory) return '';
  const category = statusCategory.toLowerCase();
  if (category.includes('done') || category.includes('complete')) return 'done';
  if (category.includes('progress')) return 'in-progress';
  return 'todo';
}

function formatDate(dateStr: string): string {
  const date = new Date(dateStr);
  const now = new Date();
  const diff = now.getTime() - date.getTime();

  if (diff < 60000) return 'Just now';
  if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
  if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
  if (diff < 604800000) return `${Math.floor(diff / 86400000)}d ago`;

  return date.toLocaleDateString();
}

function escapeHtml(str: string): string {
  const div = document.createElement('div');
  div.textContent = str;
  return div.innerHTML;
}

function updatePagination() {
  const totalPages = Math.ceil(totalResults / pageSize);

  if (totalPages <= 1) {
    paginationEl.style.display = 'none';
    return;
  }

  paginationEl.style.display = 'flex';
  prevBtnEl.disabled = currentPage === 0;
  nextBtnEl.disabled = currentPage >= totalPages - 1;
  pageInfoEl.textContent = `Page ${currentPage + 1} of ${totalPages}`;
}

async function showIssueDetail(key: string) {
  const response = await sendMessage<DbIssue>({
    type: 'GET_ISSUE',
    payload: { issueKey: key },
  });

  if (response.success && response.data) {
    currentIssue = response.data;
    detailTitleEl.textContent = key;

    // Fetch history
    const historyResponse = await sendMessage<DbChangeHistory[]>({
      type: 'GET_ISSUE_HISTORY',
      payload: { issueKey: key },
    });

    const history = historyResponse.success ? historyResponse.data || [] : [];

    renderIssueDetail(currentIssue, history);
    issueDetailEl.style.display = 'flex';
  }
}

function renderIssueDetail(issue: DbIssue, history: DbChangeHistory[]) {
  const labels = issue.labels ? JSON.parse(issue.labels) : [];
  const components = issue.components ? JSON.parse(issue.components) : [];
  const claudeInstructions = extractClaudeInstructions(issue.description);

  detailBodyEl.innerHTML = `
    ${claudeInstructions ? `
    <div class="detail-section claude-section">
      <div class="detail-label">Claude Instructions</div>
      <div class="claude-instructions">
        <pre>${escapeHtml(claudeInstructions)}</pre>
        <button id="sendToClaudeBtn" class="btn btn-claude">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 2L11 13M22 2l-7 20-4-9-9-4 20-7z"/>
          </svg>
          Send to Claude Code
        </button>
      </div>
    </div>
    ` : ''}

    <div class="detail-section">
      <div class="detail-label">Summary</div>
      <div class="detail-value">${escapeHtml(issue.summary)}</div>
    </div>

    <div class="detail-section">
      <div class="detail-label">Status</div>
      <div class="detail-value">
        <span class="issue-status ${getStatusClass(issue.status_category)}">${escapeHtml(issue.status)}</span>
      </div>
    </div>

    ${issue.description ? `
    <div class="detail-section">
      <div class="detail-label">Description</div>
      <div class="detail-value description">${escapeHtml(issue.description)}</div>
    </div>
    ` : ''}

    <div class="detail-section">
      <div class="detail-label">Details</div>
      <div class="detail-value">
        <div><strong>Type:</strong> ${escapeHtml(issue.issue_type)}</div>
        <div><strong>Priority:</strong> ${escapeHtml(issue.priority || 'None')}</div>
        <div><strong>Assignee:</strong> ${escapeHtml(issue.assignee_name || 'Unassigned')}</div>
        <div><strong>Reporter:</strong> ${escapeHtml(issue.reporter_name || 'Unknown')}</div>
        <div><strong>Created:</strong> ${new Date(issue.created_at).toLocaleString()}</div>
        <div><strong>Updated:</strong> ${new Date(issue.updated_at).toLocaleString()}</div>
      </div>
    </div>

    ${labels.length > 0 ? `
    <div class="detail-section">
      <div class="detail-label">Labels</div>
      <div class="tags">
        ${labels.map((l: string) => `<span class="tag">${escapeHtml(l)}</span>`).join('')}
      </div>
    </div>
    ` : ''}

    ${components.length > 0 ? `
    <div class="detail-section">
      <div class="detail-label">Components</div>
      <div class="tags">
        ${components.map((c: string) => `<span class="tag">${escapeHtml(c)}</span>`).join('')}
      </div>
    </div>
    ` : ''}

    ${history.length > 0 ? `
    <div class="detail-section">
      <div class="detail-label">Recent Changes</div>
      <div class="history-list">
        ${history.slice(0, 10).map((h) => `
          <div class="history-item">
            <div class="history-header">
              <span>${escapeHtml(h.author_display_name || 'Unknown')}</span>
              <span>${new Date(h.changed_at).toLocaleString()}</span>
            </div>
            <div class="history-change">
              <span class="history-field">${escapeHtml(h.field)}</span>:
              ${h.from_string ? `<span class="history-from">${escapeHtml(h.from_string)}</span> → ` : ''}
              <span class="history-to">${escapeHtml(h.to_string || '')}</span>
            </div>
          </div>
        `).join('')}
      </div>
    </div>
    ` : ''}
  `;

  // Add click handler for Send to Claude Code button
  const sendToClaudeBtn = document.getElementById('sendToClaudeBtn');
  if (sendToClaudeBtn && claudeInstructions) {
    console.log('[SidePanel] Setting up Claude Code button handler');
    sendToClaudeBtn.addEventListener('click', async () => {
      console.log('[SidePanel] Send to Claude Code button clicked');
      try {
        sendToClaudeBtn.textContent = 'Sending...';
        (sendToClaudeBtn as HTMLButtonElement).disabled = true;
        console.log('[SidePanel] Calling sendToClaudeCode with issue:', issue.key);
        await sendToClaudeCode(claudeInstructions, issue.key);
        console.log('[SidePanel] sendToClaudeCode completed');
      } catch (error) {
        console.error('[SidePanel] Failed to send to Claude Code:', error);
        alert(`Failed to send to Claude Code: ${error instanceof Error ? error.message : String(error)}`);
      } finally {
        sendToClaudeBtn.innerHTML = `
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M22 2L11 13M22 2l-7 20-4-9-9-4 20-7z"/>
          </svg>
          Send to Claude Code
        `;
        (sendToClaudeBtn as HTMLButtonElement).disabled = false;
      }
    });
  } else {
    console.log('[SidePanel] No Claude Code button to set up:', { hasButton: !!sendToClaudeBtn, hasInstructions: !!claudeInstructions });
  }
}

function hideDetail() {
  issueDetailEl.style.display = 'none';
  currentIssue = null;
}

function openInJira() {
  if (currentIssue && settings) {
    let endpoint = settings.jira.endpoint;
    if (!endpoint.startsWith('http://') && !endpoint.startsWith('https://')) {
      endpoint = `https://${endpoint}`;
    }
    const url = `${endpoint}/browse/${currentIssue.key}`;
    chrome.tabs.create({ url });
  }
}

async function startSync() {
  syncBtnEl.disabled = true;
  syncBtnEl.classList.add('syncing');

  console.log('[SidePanel] Starting sync...');
  showSyncStatus('Starting sync...');

  const response = await sendMessage({ type: 'START_SYNC' });
  console.log('[SidePanel] START_SYNC response:', response);

  if (!response.success) {
    console.error('[SidePanel] Sync failed to start:', response.error);
    hideSyncStatus();
    alert(`Failed to start sync: ${response.error}`);
  }
}

function showSyncStatus(message: string, progress: number = 0) {
  syncStatusEl.style.display = 'block';
  syncMessageEl.textContent = message;
  progressBarEl.style.width = `${progress}%`;
}

function hideSyncStatus() {
  syncStatusEl.style.display = 'none';
  syncBtnEl.disabled = false;
  syncBtnEl.classList.remove('syncing');
}

async function cancelSyncHandler() {
  await sendMessage({ type: 'CANCEL_SYNC' });
  hideSyncStatus();
}

async function checkSyncStatus() {
  const response = await sendMessage<{ isSyncing: boolean; progress: SyncProgress | null }>({
    type: 'GET_SYNC_STATUS',
  });

  if (response.success && response.data?.isSyncing && response.data.progress) {
    const progress = response.data.progress;
    const percent = progress.total > 0 ? (progress.current / progress.total) * 100 : 0;
    showSyncStatus(progress.message, percent);
    syncBtnEl.disabled = true;
    syncBtnEl.classList.add('syncing');
  }
}

function openSettings() {
  chrome.runtime.openOptionsPage();
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
searchInputEl.addEventListener('input', debounce(search, 300));
projectFilterEl.addEventListener('change', () => {
  currentPage = 0;
  search();
});
statusFilterEl.addEventListener('change', () => {
  currentPage = 0;
  search();
});

prevBtnEl.addEventListener('click', () => {
  if (currentPage > 0) {
    currentPage--;
    search();
  }
});

nextBtnEl.addEventListener('click', () => {
  const totalPages = Math.ceil(totalResults / pageSize);
  if (currentPage < totalPages - 1) {
    currentPage++;
    search();
  }
});

syncBtnEl.addEventListener('click', startSync);
settingsBtnEl.addEventListener('click', openSettings);
openSettingsBtnEl.addEventListener('click', openSettings);
backBtnEl.addEventListener('click', hideDetail);
openInJiraBtnEl.addEventListener('click', openInJira);
cancelSyncBtnEl.addEventListener('click', cancelSyncHandler);

// Listen for sync progress updates
chrome.runtime.onMessage.addListener((message) => {
  if (message.type === 'SYNC_PROGRESS') {
    const progress = message.payload as SyncProgress;
    const percent = progress.total > 0 ? (progress.current / progress.total) * 100 : 0;
    showSyncStatus(progress.message, percent);
  } else if (message.type === 'SYNC_COMPLETE') {
    hideSyncStatus();
    loadProjects();
    search();
  } else if (message.type === 'SYNC_ERROR') {
    hideSyncStatus();
    alert(`Sync failed: ${message.payload}`);
  }
});

// Debounce helper
function debounce<T extends (...args: unknown[]) => void>(
  fn: T,
  delay: number
): (...args: Parameters<T>) => void {
  let timeoutId: ReturnType<typeof setTimeout>;
  return (...args: Parameters<T>) => {
    clearTimeout(timeoutId);
    timeoutId = setTimeout(() => fn(...args), delay);
  };
}

// Close detail on escape
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && issueDetailEl.style.display !== 'none') {
    hideDetail();
  }
});

// Initialize on load
init();
