// src/popup/popup.ts
var currentPage = 0;
var pageSize = 20;
var totalResults = 0;
var currentIssue = null;
var settings = null;
var projects = [];
var notConfiguredEl = document.getElementById("notConfigured");
var mainContentEl = document.getElementById("mainContent");
var syncStatusEl = document.getElementById("syncStatus");
var syncMessageEl = document.getElementById("syncMessage");
var progressBarEl = document.getElementById("progressBar");
var searchInputEl = document.getElementById("searchInput");
var projectFilterEl = document.getElementById("projectFilter");
var statusFilterEl = document.getElementById("statusFilter");
var resultsCountEl = document.getElementById("resultsCount");
var issueListEl = document.getElementById("issueList");
var emptyStateEl = document.getElementById("emptyState");
var paginationEl = document.getElementById("pagination");
var prevBtnEl = document.getElementById("prevBtn");
var nextBtnEl = document.getElementById("nextBtn");
var pageInfoEl = document.getElementById("pageInfo");
var issueModalEl = document.getElementById("issueModal");
var modalTitleEl = document.getElementById("modalTitle");
var modalBodyEl = document.getElementById("modalBody");
var syncBtnEl = document.getElementById("syncBtn");
var settingsBtnEl = document.getElementById("settingsBtn");
var openSettingsBtnEl = document.getElementById("openSettingsBtn");
var closeModalBtnEl = document.getElementById("closeModalBtn");
var openInJiraBtnEl = document.getElementById("openInJiraBtn");
var cancelSyncBtnEl = document.getElementById("cancelSyncBtn");
var copyTemplateBtnEl = document.getElementById("copyTemplateBtn");
var AI_TEMPLATE = `\`\`\`ai
\u3053\u3053\u306BAI\u3078\u306E\u6307\u793A\u3092\u8A18\u8FF0\u3057\u3066\u304F\u3060\u3055\u3044\u3002

\u4F8B:
- \u3053\u306E\u30C1\u30B1\u30C3\u30C8\u306E\u5185\u5BB9\u3092\u5206\u6790\u3057\u3066\u3001\u5B9F\u88C5\u306B\u5FC5\u8981\u306A\u30BF\u30B9\u30AF\u3092\u6D17\u3044\u51FA\u3057\u3066\u304F\u3060\u3055\u3044
- \u3053\u306E\u30D0\u30B0\u306E\u539F\u56E0\u3092\u8ABF\u67FB\u3057\u3001\u4FEE\u6B63\u65B9\u6CD5\u3092\u63D0\u6848\u3057\u3066\u304F\u3060\u3055\u3044
- \u3053\u306E\u30C1\u30B1\u30C3\u30C8\u306B\u57FA\u3065\u3044\u3066\u30C6\u30B9\u30C8\u30B1\u30FC\u30B9\u3092\u4F5C\u6210\u3057\u3066\u304F\u3060\u3055\u3044
\`\`\``;
async function init() {
  const response = await sendMessage({ type: "GET_SETTINGS" });
  if (response.success && response.data) {
    settings = response.data;
    const isBrowserAuth = settings.jira.authMethod === "browser";
    const isConfigured = settings.jira.endpoint && (isBrowserAuth || settings.jira.username && settings.jira.apiKey);
    if (!isConfigured) {
      showNotConfigured();
      return;
    }
    await loadProjects();
    showMainContent();
    await search();
    await checkSyncStatus();
  }
}
function showNotConfigured() {
  notConfiguredEl.style.display = "flex";
  mainContentEl.style.display = "none";
}
function showMainContent() {
  notConfiguredEl.style.display = "none";
  mainContentEl.style.display = "flex";
}
async function loadProjects() {
  const response = await sendMessage({
    type: "GET_PROJECTS"
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
      const option = document.createElement("option");
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
  const response = await sendMessage({
    type: "SEARCH_ISSUES",
    payload: {
      query: query || void 0,
      project: project || void 0,
      status: status || void 0,
      limit: pageSize,
      offset: currentPage * pageSize
    }
  });
  if (response.success && response.data) {
    totalResults = response.data.total;
    renderIssues(response.data.issues);
    updatePagination();
    if (response.data.issues.length > 0 && statusFilterEl.options.length <= 1) {
      updateStatusFilter(response.data.issues);
    }
  }
}
function updateStatusFilter(issues) {
  const statuses = [...new Set(issues.map((i) => i.status))].sort();
  const currentValue = statusFilterEl.value;
  statusFilterEl.innerHTML = '<option value="">All Statuses</option>';
  for (const status of statuses) {
    const option = document.createElement("option");
    option.value = status;
    option.textContent = status;
    statusFilterEl.appendChild(option);
  }
  statusFilterEl.value = currentValue;
}
function renderIssues(issues) {
  resultsCountEl.textContent = `${totalResults} issue${totalResults !== 1 ? "s" : ""}`;
  if (issues.length === 0) {
    issueListEl.style.display = "none";
    emptyStateEl.style.display = "flex";
    return;
  }
  issueListEl.style.display = "block";
  emptyStateEl.style.display = "none";
  issueListEl.innerHTML = issues.map((issue) => {
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
              ${issue.assignee_name ? `<span>${escapeHtml(issue.assignee_name)}</span>` : ""}
              <span>${updatedAt}</span>
            </div>
          </div>
        </div>
      `;
  }).join("");
  issueListEl.querySelectorAll(".issue-item").forEach((el) => {
    el.addEventListener("click", () => {
      const key = el.getAttribute("data-key");
      if (key)
        showIssueDetail(key);
    });
  });
}
function getStatusClass(statusCategory) {
  if (!statusCategory)
    return "";
  const category = statusCategory.toLowerCase();
  if (category.includes("done") || category.includes("complete"))
    return "done";
  if (category.includes("progress"))
    return "in-progress";
  return "todo";
}
function formatDate(dateStr) {
  const date = new Date(dateStr);
  const now = /* @__PURE__ */ new Date();
  const diff = now.getTime() - date.getTime();
  if (diff < 6e4)
    return "Just now";
  if (diff < 36e5)
    return `${Math.floor(diff / 6e4)}m ago`;
  if (diff < 864e5)
    return `${Math.floor(diff / 36e5)}h ago`;
  if (diff < 6048e5)
    return `${Math.floor(diff / 864e5)}d ago`;
  return date.toLocaleDateString();
}
function escapeHtml(str) {
  const div = document.createElement("div");
  div.textContent = str;
  return div.innerHTML;
}
function updatePagination() {
  const totalPages = Math.ceil(totalResults / pageSize);
  if (totalPages <= 1) {
    paginationEl.style.display = "none";
    return;
  }
  paginationEl.style.display = "flex";
  prevBtnEl.disabled = currentPage === 0;
  nextBtnEl.disabled = currentPage >= totalPages - 1;
  pageInfoEl.textContent = `Page ${currentPage + 1} of ${totalPages}`;
}
async function showIssueDetail(key) {
  const response = await sendMessage({
    type: "GET_ISSUE",
    payload: { issueKey: key }
  });
  if (response.success && response.data) {
    currentIssue = response.data;
    modalTitleEl.textContent = key;
    const historyResponse = await sendMessage({
      type: "GET_ISSUE_HISTORY",
      payload: { issueKey: key }
    });
    const history = historyResponse.success ? historyResponse.data || [] : [];
    renderIssueDetail(currentIssue, history);
    issueModalEl.style.display = "flex";
  }
}
function renderIssueDetail(issue, history) {
  const labels = issue.labels ? JSON.parse(issue.labels) : [];
  const components = issue.components ? JSON.parse(issue.components) : [];
  modalBodyEl.innerHTML = `
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
    ` : ""}

    <div class="detail-section">
      <div class="detail-label">Details</div>
      <div class="detail-value">
        <div><strong>Type:</strong> ${escapeHtml(issue.issue_type)}</div>
        <div><strong>Priority:</strong> ${escapeHtml(issue.priority || "None")}</div>
        <div><strong>Assignee:</strong> ${escapeHtml(issue.assignee_name || "Unassigned")}</div>
        <div><strong>Reporter:</strong> ${escapeHtml(issue.reporter_name || "Unknown")}</div>
        <div><strong>Created:</strong> ${new Date(issue.created_at).toLocaleString()}</div>
        <div><strong>Updated:</strong> ${new Date(issue.updated_at).toLocaleString()}</div>
      </div>
    </div>

    ${labels.length > 0 ? `
    <div class="detail-section">
      <div class="detail-label">Labels</div>
      <div class="tags">
        ${labels.map((l) => `<span class="tag">${escapeHtml(l)}</span>`).join("")}
      </div>
    </div>
    ` : ""}

    ${components.length > 0 ? `
    <div class="detail-section">
      <div class="detail-label">Components</div>
      <div class="tags">
        ${components.map((c) => `<span class="tag">${escapeHtml(c)}</span>`).join("")}
      </div>
    </div>
    ` : ""}

    ${history.length > 0 ? `
    <div class="detail-section">
      <div class="detail-label">Recent Changes</div>
      <div class="history-list">
        ${history.slice(0, 10).map((h) => `
          <div class="history-item">
            <div class="history-header">
              <span>${escapeHtml(h.author_display_name || "Unknown")}</span>
              <span>${new Date(h.changed_at).toLocaleString()}</span>
            </div>
            <div class="history-change">
              <span class="history-field">${escapeHtml(h.field)}</span>:
              ${h.from_string ? `<span class="history-from">${escapeHtml(h.from_string)}</span> \u2192 ` : ""}
              <span class="history-to">${escapeHtml(h.to_string || "")}</span>
            </div>
          </div>
        `).join("")}
      </div>
    </div>
    ` : ""}
  `;
}
function closeModal() {
  issueModalEl.style.display = "none";
  currentIssue = null;
}
function openInJira() {
  if (currentIssue && settings) {
    const url = `${settings.jira.endpoint}/browse/${currentIssue.key}`;
    chrome.tabs.create({ url });
  }
}
async function startSync() {
  syncBtnEl.disabled = true;
  syncBtnEl.classList.add("syncing");
  console.log("[Popup] Starting sync...");
  showSyncStatus("Starting sync...");
  const response = await sendMessage({ type: "START_SYNC" });
  console.log("[Popup] START_SYNC response:", response);
  if (!response.success) {
    console.error("[Popup] Sync failed to start:", response.error);
    hideSyncStatus();
    alert(`Failed to start sync: ${response.error}`);
  }
}
function showSyncStatus(message, progress = 0) {
  syncStatusEl.style.display = "block";
  syncMessageEl.textContent = message;
  progressBarEl.style.width = `${progress}%`;
}
function hideSyncStatus() {
  syncStatusEl.style.display = "none";
  syncBtnEl.disabled = false;
  syncBtnEl.classList.remove("syncing");
}
async function cancelSyncHandler() {
  await sendMessage({ type: "CANCEL_SYNC" });
  hideSyncStatus();
}
async function checkSyncStatus() {
  const response = await sendMessage({
    type: "GET_SYNC_STATUS"
  });
  if (response.success && response.data?.isSyncing && response.data.progress) {
    const progress = response.data.progress;
    const percent = progress.total > 0 ? progress.current / progress.total * 100 : 0;
    showSyncStatus(progress.message, percent);
    syncBtnEl.disabled = true;
    syncBtnEl.classList.add("syncing");
  }
}
function openSettings() {
  chrome.runtime.openOptionsPage();
}
async function copyTemplateToClipboard() {
  try {
    await navigator.clipboard.writeText(AI_TEMPLATE);
    const originalTitle = copyTemplateBtnEl.title;
    copyTemplateBtnEl.title = "\u30B3\u30D4\u30FC\u3057\u307E\u3057\u305F!";
    copyTemplateBtnEl.style.color = "#00875a";
    setTimeout(() => {
      copyTemplateBtnEl.title = originalTitle;
      copyTemplateBtnEl.style.color = "";
    }, 2e3);
  } catch (error) {
    console.error("Failed to copy template:", error);
    alert("\u30C6\u30F3\u30D7\u30EC\u30FC\u30C8\u306E\u30B3\u30D4\u30FC\u306B\u5931\u6557\u3057\u307E\u3057\u305F");
  }
}
async function sendMessage(message) {
  return new Promise((resolve) => {
    chrome.runtime.sendMessage(message, (response) => {
      if (chrome.runtime.lastError) {
        resolve({ success: false, error: chrome.runtime.lastError.message });
      } else {
        resolve(response || { success: false, error: "No response" });
      }
    });
  });
}
searchInputEl.addEventListener("input", debounce(search, 300));
projectFilterEl.addEventListener("change", () => {
  currentPage = 0;
  search();
});
statusFilterEl.addEventListener("change", () => {
  currentPage = 0;
  search();
});
prevBtnEl.addEventListener("click", () => {
  if (currentPage > 0) {
    currentPage--;
    search();
  }
});
nextBtnEl.addEventListener("click", () => {
  const totalPages = Math.ceil(totalResults / pageSize);
  if (currentPage < totalPages - 1) {
    currentPage++;
    search();
  }
});
syncBtnEl.addEventListener("click", startSync);
settingsBtnEl.addEventListener("click", openSettings);
openSettingsBtnEl.addEventListener("click", openSettings);
closeModalBtnEl.addEventListener("click", closeModal);
openInJiraBtnEl.addEventListener("click", openInJira);
cancelSyncBtnEl.addEventListener("click", cancelSyncHandler);
copyTemplateBtnEl.addEventListener("click", copyTemplateToClipboard);
chrome.runtime.onMessage.addListener((message) => {
  if (message.type === "SYNC_PROGRESS") {
    const progress = message.payload;
    const percent = progress.total > 0 ? progress.current / progress.total * 100 : 0;
    showSyncStatus(progress.message, percent);
  } else if (message.type === "SYNC_COMPLETE") {
    hideSyncStatus();
    loadProjects();
    search();
  } else if (message.type === "SYNC_ERROR") {
    hideSyncStatus();
    alert(`Sync failed: ${message.payload}`);
  }
});
function debounce(fn, delay) {
  let timeoutId;
  return (...args) => {
    clearTimeout(timeoutId);
    timeoutId = setTimeout(() => fn(...args), delay);
  };
}
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && issueModalEl.style.display !== "none") {
    closeModal();
  }
});
issueModalEl.addEventListener("click", (e) => {
  if (e.target === issueModalEl) {
    closeModal();
  }
});
init();
