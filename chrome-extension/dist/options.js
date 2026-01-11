// src/options/options.ts
var endpointEl = document.getElementById("endpoint");
var authBrowserEl = document.getElementById("authBrowser");
var authApiTokenEl = document.getElementById("authApiToken");
var apiTokenFieldsEl = document.getElementById("apiTokenFields");
var usernameEl = document.getElementById("username");
var apiKeyEl = document.getElementById("apiKey");
var testConnectionBtnEl = document.getElementById("testConnectionBtn");
var saveConnectionBtnEl = document.getElementById("saveConnectionBtn");
var connectionStatusEl = document.getElementById("connectionStatus");
var incrementalSyncEl = document.getElementById("incrementalSync");
var marginMinutesEl = document.getElementById("marginMinutes");
var batchSizeEl = document.getElementById("batchSize");
var saveSyncSettingsBtnEl = document.getElementById("saveSyncSettingsBtn");
var fetchProjectsBtnEl = document.getElementById("fetchProjectsBtn");
var projectListEl = document.getElementById("projectList");
var projectsStatusEl = document.getElementById("projectsStatus");
var exportDataBtnEl = document.getElementById("exportDataBtn");
var clearDataBtnEl = document.getElementById("clearDataBtn");
var dataStatusEl = document.getElementById("dataStatus");
var settings = null;
var projects = [];
async function init() {
  const response = await sendMessage({ type: "GET_SETTINGS" });
  if (response.success && response.data) {
    settings = response.data;
    populateForm();
  }
  await loadProjects();
}
function populateForm() {
  if (!settings)
    return;
  endpointEl.value = settings.jira.endpoint;
  const authMethod = settings.jira.authMethod || "browser";
  if (authMethod === "browser") {
    authBrowserEl.checked = true;
    apiTokenFieldsEl.style.display = "none";
  } else {
    authApiTokenEl.checked = true;
    apiTokenFieldsEl.style.display = "block";
  }
  usernameEl.value = settings.jira.username;
  apiKeyEl.value = settings.jira.apiKey;
  incrementalSyncEl.checked = settings.sync.incrementalSyncEnabled;
  marginMinutesEl.value = String(settings.sync.incrementalSyncMarginMinutes);
  batchSizeEl.value = String(settings.sync.batchSize);
}
function getSelectedAuthMethod() {
  return authApiTokenEl.checked ? "api_token" : "browser";
}
function toggleApiTokenFields() {
  apiTokenFieldsEl.style.display = authApiTokenEl.checked ? "block" : "none";
}
async function loadProjects() {
  const response = await sendMessage({
    type: "GET_PROJECTS"
  });
  if (response.success && response.data) {
    projects = response.data;
    renderProjects();
  }
}
function renderProjects() {
  if (projects.length === 0) {
    projectListEl.innerHTML = `
      <div class="empty-state">
        <p>No projects loaded. Click "Fetch Projects" to load from JIRA.</p>
      </div>
    `;
    return;
  }
  projectListEl.innerHTML = projects.map((project) => {
    const issueCount = project.issueCount || 0;
    const lastSync = project.lastSyncedAt ? new Date(project.lastSyncedAt).toLocaleString() : "Never";
    return `
        <div class="project-item" data-key="${project.key}">
          <div class="project-info">
            <span class="project-key">${escapeHtml(project.key)}</span>
            <span class="project-name">${escapeHtml(project.name)}</span>
            ${project.hasCheckpoint ? '<span class="checkpoint-badge">Resume available</span>' : ""}
          </div>
          <div class="project-meta">
            ${issueCount} issues | Last sync: ${lastSync}
          </div>
          <div class="project-actions">
            <label class="project-toggle">
              <input type="checkbox" ${project.enabled ? "checked" : ""} data-project="${project.key}">
              <span class="slider"></span>
            </label>
          </div>
        </div>
      `;
  }).join("");
  projectListEl.querySelectorAll('input[type="checkbox"]').forEach((checkbox) => {
    checkbox.addEventListener("change", async (e) => {
      const target = e.target;
      const projectKey = target.dataset.project;
      if (projectKey) {
        await sendMessage({
          type: "ENABLE_PROJECT",
          payload: { projectKey, enabled: target.checked }
        });
      }
    });
  });
}
function escapeHtml(str) {
  const div = document.createElement("div");
  div.textContent = str;
  return div.innerHTML;
}
function showStatus(element, type, message) {
  element.className = `status-message ${type}`;
  element.textContent = message;
  setTimeout(() => {
    element.className = "status-message";
    element.textContent = "";
  }, 5e3);
}
async function testConnection() {
  testConnectionBtnEl.disabled = true;
  testConnectionBtnEl.innerHTML = '<span class="loading"><span class="spinner"></span>Testing...</span>';
  const authMethod = getSelectedAuthMethod();
  const tempSettings = {
    ...settings,
    jira: {
      endpoint: endpointEl.value.trim(),
      authMethod,
      username: usernameEl.value.trim(),
      apiKey: apiKeyEl.value.trim()
    }
  };
  await sendMessage({ type: "SAVE_SETTINGS", payload: tempSettings });
  const response = await sendMessage({ type: "INIT_PROJECTS" });
  testConnectionBtnEl.disabled = false;
  testConnectionBtnEl.textContent = "Test Connection";
  if (response.success) {
    showStatus(connectionStatusEl, "success", "Connection successful!");
    await loadProjects();
  } else {
    showStatus(connectionStatusEl, "error", `Connection failed: ${response.error}`);
  }
}
async function saveConnection() {
  if (!settings)
    return;
  const authMethod = getSelectedAuthMethod();
  settings.jira = {
    endpoint: endpointEl.value.trim(),
    authMethod,
    username: usernameEl.value.trim(),
    apiKey: apiKeyEl.value.trim()
  };
  await sendMessage({ type: "SAVE_SETTINGS", payload: settings });
  showStatus(connectionStatusEl, "success", "Connection settings saved!");
}
async function saveSyncSettings() {
  if (!settings)
    return;
  settings.sync = {
    incrementalSyncEnabled: incrementalSyncEl.checked,
    incrementalSyncMarginMinutes: parseInt(marginMinutesEl.value, 10) || 5,
    batchSize: parseInt(batchSizeEl.value, 10) || 100
  };
  await sendMessage({ type: "SAVE_SETTINGS", payload: settings });
  showStatus(projectsStatusEl, "success", "Sync settings saved!");
}
async function fetchProjects() {
  fetchProjectsBtnEl.disabled = true;
  fetchProjectsBtnEl.innerHTML = '<span class="loading"><span class="spinner"></span>Fetching...</span>';
  const response = await sendMessage({ type: "INIT_PROJECTS" });
  fetchProjectsBtnEl.disabled = false;
  fetchProjectsBtnEl.textContent = "Fetch Projects from JIRA";
  if (response.success) {
    showStatus(projectsStatusEl, "success", "Projects loaded successfully!");
    await loadProjects();
  } else {
    showStatus(projectsStatusEl, "error", `Failed to load projects: ${response.error}`);
  }
}
async function exportData() {
  showStatus(dataStatusEl, "info", "Export functionality coming soon!");
}
async function clearData() {
  if (!confirm("Are you sure you want to clear all synced data? This cannot be undone.")) {
    return;
  }
  chrome.storage.local.clear(() => {
    showStatus(dataStatusEl, "success", "All data cleared. Please reload the extension.");
    setTimeout(() => {
      chrome.runtime.reload();
    }, 2e3);
  });
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
testConnectionBtnEl.addEventListener("click", testConnection);
saveConnectionBtnEl.addEventListener("click", saveConnection);
saveSyncSettingsBtnEl.addEventListener("click", saveSyncSettings);
fetchProjectsBtnEl.addEventListener("click", fetchProjects);
exportDataBtnEl.addEventListener("click", exportData);
clearDataBtnEl.addEventListener("click", clearData);
authBrowserEl.addEventListener("change", toggleApiTokenFields);
authApiTokenEl.addEventListener("change", toggleApiTokenFields);
init();
