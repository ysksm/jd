"use strict";
(() => {
  var __defProp = Object.defineProperty;
  var __getOwnPropNames = Object.getOwnPropertyNames;
  var __esm = (fn, res) => function __init() {
    return fn && (res = (0, fn[__getOwnPropNames(fn)[0]])(fn = 0)), res;
  };
  var __export = (target, all) => {
    for (var name in all)
      __defProp(target, name, { get: all[name], enumerable: true });
  };

  // src/lib/database.ts
  var database_exports = {};
  __export(database_exports, {
    closeDatabase: () => closeDatabase,
    completeSyncHistory: () => completeSyncHistory,
    exportDatabase: () => exportDatabase,
    getConnection: () => getConnection,
    getIssue: () => getIssue,
    getIssueCount: () => getIssueCount,
    getIssueHistory: () => getIssueHistory,
    getLatestUpdatedAt: () => getLatestUpdatedAt,
    getProjectStatuses: () => getProjectStatuses,
    getProjects: () => getProjects,
    initDatabase: () => initDatabase,
    persistDatabase: () => persistDatabase,
    searchIssues: () => searchIssues,
    startSyncHistory: () => startSyncHistory,
    updateSyncHistoryProgress: () => updateSyncHistoryProgress,
    upsertIssue: () => upsertIssue,
    upsertProject: () => upsertProject
  });
  async function ensureOffscreenDocument() {
    const existingContexts = await chrome.runtime.getContexts({
      contextTypes: [chrome.runtime.ContextType.OFFSCREEN_DOCUMENT]
    });
    if (existingContexts.length > 0 && offscreenReady) {
      return;
    }
    if (creatingOffscreen) {
      while (creatingOffscreen || !offscreenReady) {
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
      return;
    }
    if (existingContexts.length === 0) {
      console.log("[Database] Creating offscreen document...");
      creatingOffscreen = true;
      try {
        await chrome.offscreen.createDocument({
          url: "offscreen.html",
          reasons: [chrome.offscreen.Reason.WORKERS],
          justification: "DuckDB WASM requires Web Workers which are not available in service workers"
        });
        console.log("[Database] Offscreen document created, waiting for it to be ready...");
        await new Promise((resolve) => setTimeout(resolve, 500));
      } catch (error) {
        console.error("[Database] Failed to create offscreen document:", error);
        throw error;
      } finally {
        creatingOffscreen = false;
      }
    } else {
      console.log("[Database] Offscreen document already exists");
    }
    console.log("[Database] Sending PING to offscreen document...");
    let retries = 50;
    while (retries > 0) {
      try {
        const response = await new Promise((resolve) => {
          chrome.runtime.sendMessage(
            { target: "offscreen", action: "PING" },
            (resp) => {
              if (chrome.runtime.lastError) {
                console.log("[Database] PING error:", chrome.runtime.lastError.message);
                resolve({ success: false });
              } else {
                console.log("[Database] PING response:", resp);
                resolve(resp || { success: false });
              }
            }
          );
        });
        if (response.success && response.data === "PONG") {
          console.log("[Database] Offscreen document is ready!");
          offscreenReady = true;
          return;
        }
      } catch (error) {
        console.log("[Database] PING exception:", error);
      }
      await new Promise((resolve) => setTimeout(resolve, 100));
      retries--;
    }
    console.error("[Database] Offscreen document failed to respond after 5 seconds");
    throw new Error("Offscreen document failed to initialize");
  }
  async function sendToOffscreen(action, payload) {
    await ensureOffscreenDocument();
    return new Promise((resolve, reject) => {
      chrome.runtime.sendMessage(
        {
          target: "offscreen",
          action,
          payload
        },
        (response) => {
          if (chrome.runtime.lastError) {
            reject(new Error(chrome.runtime.lastError.message));
          } else if (!response) {
            reject(new Error("No response from offscreen document"));
          } else if (!response.success) {
            reject(new Error(response.error || "Unknown error"));
          } else {
            resolve(response.data);
          }
        }
      );
    });
  }
  async function initDatabase() {
    await sendToOffscreen("INIT_DATABASE");
  }
  async function getConnection() {
    await initDatabase();
    return null;
  }
  async function upsertProject(project) {
    await sendToOffscreen("UPSERT_PROJECT", project);
  }
  async function getProjects() {
    return await sendToOffscreen("GET_PROJECTS");
  }
  async function upsertIssue(issue) {
    await sendToOffscreen("UPSERT_ISSUE", issue);
  }
  async function getIssue(key) {
    return await sendToOffscreen("GET_ISSUE", { issueKey: key });
  }
  async function searchIssues(params) {
    return await sendToOffscreen("SEARCH_ISSUES", params);
  }
  async function getIssueHistory(issueKey, field) {
    return await sendToOffscreen("GET_ISSUE_HISTORY", { issueKey, field });
  }
  async function getLatestUpdatedAt(projectKey) {
    return await sendToOffscreen("GET_LATEST_UPDATED_AT", { projectKey });
  }
  async function getIssueCount(projectKey) {
    return await sendToOffscreen("GET_ISSUE_COUNT", { projectKey });
  }
  async function startSyncHistory(projectKey) {
    return await sendToOffscreen("START_SYNC_HISTORY", { projectKey });
  }
  async function completeSyncHistory(id, success, issuesSynced, errorMessage) {
    await sendToOffscreen("COMPLETE_SYNC_HISTORY", { id, success, issuesSynced, errorMessage });
  }
  async function updateSyncHistoryProgress(id, issuesSynced) {
    await sendToOffscreen("UPDATE_SYNC_HISTORY_PROGRESS", { id, issuesSynced });
  }
  async function getProjectStatuses(projectKey) {
    return await sendToOffscreen("GET_PROJECT_STATUSES", { projectKey });
  }
  async function exportDatabase() {
    return await sendToOffscreen("EXPORT_DATABASE");
  }
  async function persistDatabase() {
    await sendToOffscreen("PERSIST_DATABASE");
  }
  async function closeDatabase() {
  }
  var creatingOffscreen, offscreenReady;
  var init_database = __esm({
    "src/lib/database.ts"() {
      "use strict";
      creatingOffscreen = false;
      offscreenReady = false;
    }
  });

  // src/lib/settings.ts
  var SETTINGS_KEY = "jira_db_settings";
  var DEFAULT_SETTINGS = {
    jira: {
      endpoint: "",
      authMethod: "browser",
      // Default to browser auth (no credentials needed)
      username: "",
      apiKey: ""
    },
    sync: {
      incrementalSyncEnabled: true,
      incrementalSyncMarginMinutes: 5,
      batchSize: 100
    },
    projects: []
  };
  async function loadSettings() {
    return new Promise((resolve) => {
      chrome.storage.local.get([SETTINGS_KEY], (result) => {
        const stored = result[SETTINGS_KEY];
        if (stored) {
          const settings = {
            ...DEFAULT_SETTINGS,
            ...stored,
            jira: {
              ...DEFAULT_SETTINGS.jira,
              ...stored.jira || {}
            },
            sync: {
              ...DEFAULT_SETTINGS.sync,
              ...stored.sync || {}
            },
            projects: stored.projects || []
          };
          resolve(settings);
        } else {
          resolve(DEFAULT_SETTINGS);
        }
      });
    });
  }
  async function saveSettings(settings) {
    return new Promise((resolve, reject) => {
      chrome.storage.local.set({ [SETTINGS_KEY]: settings }, () => {
        if (chrome.runtime.lastError) {
          reject(new Error(chrome.runtime.lastError.message));
        } else {
          resolve();
        }
      });
    });
  }
  async function updateProjectConfig(projectKey, updates) {
    const settings = await loadSettings();
    const projectIndex = settings.projects.findIndex((p) => p.key === projectKey);
    if (projectIndex >= 0) {
      settings.projects[projectIndex] = {
        ...settings.projects[projectIndex],
        ...updates
      };
      await saveSettings(settings);
      console.log(`[Settings] Updated project ${projectKey}:`, updates);
    } else {
      console.warn(
        `[Settings] Project ${projectKey} not found in settings. Available projects:`,
        settings.projects.map((p) => p.key)
      );
    }
  }
  async function saveSyncCheckpoint(projectKey, checkpoint) {
    await updateProjectConfig(projectKey, { syncCheckpoint: checkpoint });
  }
  async function clearSyncCheckpoint(projectKey) {
    const settings = await loadSettings();
    const projectIndex = settings.projects.findIndex((p) => p.key === projectKey);
    if (projectIndex >= 0) {
      delete settings.projects[projectIndex].syncCheckpoint;
      settings.projects[projectIndex].lastSyncedAt = (/* @__PURE__ */ new Date()).toISOString();
    }
    await saveSettings(settings);
  }
  async function getSyncCheckpoint(projectKey) {
    const settings = await loadSettings();
    const project = settings.projects.find((p) => p.key === projectKey);
    return project?.syncCheckpoint;
  }
  async function upsertProjectInSettings(project, enabled = false) {
    const settings = await loadSettings();
    const existingIndex = settings.projects.findIndex((p) => p.key === project.key);
    if (existingIndex >= 0) {
      settings.projects[existingIndex].name = project.name;
    } else {
      settings.projects.push({
        key: project.key,
        name: project.name,
        enabled
      });
    }
    await saveSettings(settings);
  }
  async function setProjectEnabled(projectKey, enabled) {
    await updateProjectConfig(projectKey, { enabled });
  }

  // src/lib/jira-client.ts
  var JiraClient = class {
    endpoint;
    authHeader;
    useBrowserAuth;
    constructor(settings) {
      this.endpoint = settings.endpoint.replace(/\/$/, "");
      this.useBrowserAuth = settings.authMethod === "browser";
      if (this.useBrowserAuth) {
        this.authHeader = null;
      } else {
        const credentials = btoa(`${settings.username}:${settings.apiKey}`);
        this.authHeader = `Basic ${credentials}`;
      }
    }
    async request(path, options = {}) {
      let endpoint = this.endpoint;
      if (!endpoint.startsWith("http://") && !endpoint.startsWith("https://")) {
        endpoint = `https://${endpoint}`;
      }
      const url = `${endpoint}${path}`;
      console.log(`[JIRA Client] Requesting: ${url}`);
      const headers = {
        "Content-Type": "application/json",
        "Accept": "application/json"
      };
      if (this.authHeader) {
        headers["Authorization"] = this.authHeader;
      }
      let response;
      try {
        response = await fetch(url, {
          ...options,
          // Include cookies for browser auth
          credentials: this.useBrowserAuth ? "include" : "omit",
          headers: {
            ...headers,
            ...options.headers
          }
        });
      } catch (error) {
        console.error("[JIRA Client] Fetch error:", error);
        if (error instanceof TypeError && error.message === "Failed to fetch") {
          throw new Error(
            `Could not connect to ${endpoint}. Please check:
1. The endpoint URL is correct
2. You have network connectivity
3. If using a custom domain (not *.atlassian.net), add it to the extension's permissions`
          );
        }
        throw error;
      }
      if (!response.ok) {
        const errorText = await response.text();
        if (response.status === 401) {
          if (this.useBrowserAuth) {
            throw new Error("Not logged in to JIRA. Please log in to JIRA in your browser first.");
          } else {
            throw new Error("Invalid API credentials. Please check your username and API token.");
          }
        }
        throw new Error(`JIRA API error: ${response.status} ${response.statusText} - ${errorText}`);
      }
      return response.json();
    }
    // Get all projects
    async getProjects() {
      return this.request("/rest/api/3/project");
    }
    // Get project by key
    async getProject(key) {
      return this.request(`/rest/api/3/project/${key}`);
    }
    // Search issues with JQL
    async searchIssues(jql, startAt = 0, maxResults = 100) {
      const params = new URLSearchParams({
        jql,
        startAt: startAt.toString(),
        maxResults: maxResults.toString(),
        fields: "*navigable",
        expand: "changelog"
      });
      return this.request(
        `/rest/api/3/search/jql?${params.toString()}`
      );
    }
    // Get single issue with changelog
    async getIssue(issueKey) {
      const params = new URLSearchParams({
        fields: "*navigable",
        expand: "changelog"
      });
      return this.request(
        `/rest/api/3/issue/${issueKey}?${params.toString()}`
      );
    }
    // Get project statuses
    async getProjectStatuses(projectKey) {
      const response = await this.request(
        `/rest/api/3/project/${projectKey}/statuses`
      );
      const statuses = /* @__PURE__ */ new Map();
      for (const issueType of response) {
        for (const status of issueType.statuses) {
          statuses.set(status.id, status);
        }
      }
      return Array.from(statuses.values());
    }
    // Get priorities
    async getPriorities() {
      return this.request("/rest/api/3/priority");
    }
    // Get issue types for a project
    async getIssueTypes(projectId) {
      return this.request(
        `/rest/api/3/issuetype/project?projectId=${projectId}`
      );
    }
    // Test connection
    async testConnection() {
      try {
        await this.request("/rest/api/3/myself");
        return true;
      } catch {
        return false;
      }
    }
    // Get all issues for a project with pagination
    async *getAllIssues(projectKey, updatedSince, onProgress) {
      let jql = `project = ${projectKey}`;
      if (updatedSince) {
        const date = new Date(updatedSince);
        const jiraDate = formatJiraDate(date);
        jql += ` AND updated >= "${jiraDate}"`;
        console.log(`[JIRA Client] Incremental sync: updatedSince=${updatedSince}, jiraDate=${jiraDate}`);
      } else {
        console.log(`[JIRA Client] Full sync: no updatedSince filter`);
      }
      jql += " ORDER BY updated ASC";
      console.log(`[JIRA Client] JQL query: ${jql}`);
      let startAt = 0;
      const maxResults = 100;
      let total = 0;
      do {
        const response = await this.searchIssues(jql, startAt, maxResults);
        total = response.total;
        if (response.issues.length === 0)
          break;
        yield response.issues;
        startAt += response.issues.length;
        if (onProgress) {
          onProgress(startAt, total);
        }
      } while (startAt < total);
    }
    // Get issues starting from a checkpoint
    async *getIssuesFromCheckpoint(projectKey, startPosition, updatedSince, onProgress) {
      let jql = `project = ${projectKey}`;
      if (updatedSince) {
        const date = new Date(updatedSince);
        const jiraDate = formatJiraDate(date);
        jql += ` AND updated >= "${jiraDate}"`;
      }
      jql += " ORDER BY updated ASC";
      let startAt = startPosition;
      const maxResults = 100;
      let total = 0;
      do {
        const response = await this.searchIssues(jql, startAt, maxResults);
        total = response.total;
        if (response.issues.length === 0)
          break;
        yield response.issues;
        startAt += response.issues.length;
        if (onProgress) {
          onProgress(startAt, total);
        }
      } while (startAt < total);
    }
  };
  function formatJiraDate(date) {
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, "0");
    const day = String(date.getDate()).padStart(2, "0");
    const hours = String(date.getHours()).padStart(2, "0");
    const minutes = String(date.getMinutes()).padStart(2, "0");
    return `${year}-${month}-${day} ${hours}:${minutes}`;
  }

  // src/lib/sync-service.ts
  init_database();
  var isSyncing = false;
  var cancelRequested = false;
  var currentSyncProgress = null;
  function getSyncStatus() {
    return { isSyncing, progress: currentSyncProgress };
  }
  function cancelSync() {
    cancelRequested = true;
  }
  async function initProjects() {
    const settings = await loadSettings();
    const client = new JiraClient(settings.jira);
    const jiraProjects = await client.getProjects();
    for (const project of jiraProjects) {
      await upsertProjectInSettings({ key: project.key, name: project.name });
    }
    try {
      await initDatabase();
      for (const project of jiraProjects) {
        await upsertProject(project);
      }
    } catch (error) {
      console.warn("Could not save projects to database (will be saved during sync):", error);
    }
  }
  async function syncProject(projectKey, onProgress) {
    console.log(`[SyncService] syncProject started for ${projectKey}`);
    const startedAt = (/* @__PURE__ */ new Date()).toISOString();
    let issuesSynced = 0;
    let syncHistoryId = 0;
    try {
      const settings = await loadSettings();
      console.log(`[SyncService] Settings loaded, creating JIRA client`);
      const client = new JiraClient(settings.jira);
      console.log(`[SyncService] Initializing database...`);
      await initDatabase();
      console.log(`[SyncService] Database initialized`);
      syncHistoryId = await startSyncHistory(projectKey);
      console.log(`[SyncService] Sync history started with ID ${syncHistoryId}`);
      const checkpoint = await getSyncCheckpoint(projectKey);
      let startPosition = 0;
      let lastProcessedUpdatedAt;
      if (checkpoint) {
        startPosition = checkpoint.startPosition;
        lastProcessedUpdatedAt = checkpoint.lastProcessedUpdatedAt;
        console.log(`Resuming sync for ${projectKey} from position ${startPosition}`);
      }
      let updatedSince;
      console.log(`[SyncService] Incremental sync settings: enabled=${settings.sync.incrementalSyncEnabled}, hasCheckpoint=${!!checkpoint}`);
      if (settings.sync.incrementalSyncEnabled && !checkpoint) {
        const latestInDb = await getLatestUpdatedAt(projectKey);
        console.log(`[SyncService] Latest updated_at in DB for ${projectKey}: "${latestInDb}" (type: ${typeof latestInDb})`);
        if (latestInDb) {
          const marginMs = settings.sync.incrementalSyncMarginMinutes * 60 * 1e3;
          const parsedDate = new Date(latestInDb);
          console.log(`[SyncService] Parsed date: ${parsedDate.toISOString()}, valid: ${!isNaN(parsedDate.getTime())}`);
          const date = new Date(parsedDate.getTime() - marginMs);
          updatedSince = date.toISOString();
          console.log(`[SyncService] Incremental sync from ${updatedSince} (with ${settings.sync.incrementalSyncMarginMinutes} min margin)`);
        } else {
          console.log(`[SyncService] No previous sync data found, performing full sync`);
        }
      } else if (checkpoint) {
        updatedSince = lastProcessedUpdatedAt;
        console.log(`[SyncService] Using checkpoint date: ${updatedSince}`);
      }
      let totalIssues = 0;
      const generator = checkpoint ? client.getIssuesFromCheckpoint(
        projectKey,
        startPosition,
        updatedSince,
        (current, total) => {
          totalIssues = total;
          currentSyncProgress = {
            projectKey,
            phase: "issues",
            current,
            total,
            message: `Syncing issues: ${current}/${total}`
          };
          onProgress?.(currentSyncProgress);
        }
      ) : client.getAllIssues(projectKey, updatedSince, (current, total) => {
        totalIssues = total;
        currentSyncProgress = {
          projectKey,
          phase: "issues",
          current,
          total,
          message: `Syncing issues: ${current}/${total}`
        };
        onProgress?.(currentSyncProgress);
      });
      for await (const batch of generator) {
        if (cancelRequested) {
          const lastIssue2 = batch[batch.length - 1];
          if (lastIssue2) {
            await saveSyncCheckpoint(projectKey, {
              lastProcessedUpdatedAt: lastIssue2.fields.updated,
              startPosition: issuesSynced + batch.length,
              totalIssues
            });
          }
          throw new Error("Sync cancelled by user");
        }
        for (const issue of batch) {
          await upsertIssue(issue);
          issuesSynced++;
        }
        const lastIssue = batch[batch.length - 1];
        if (lastIssue) {
          const newCheckpoint = {
            lastProcessedUpdatedAt: lastIssue.fields.updated,
            startPosition: issuesSynced,
            totalIssues
          };
          await saveSyncCheckpoint(projectKey, newCheckpoint);
        }
        await updateSyncHistoryProgress(syncHistoryId, issuesSynced);
      }
      await clearSyncCheckpoint(projectKey);
      await completeSyncHistory(syncHistoryId, true, issuesSynced);
      console.log(`[SyncService] Persisting database...`);
      try {
        await persistDatabase();
        console.log(`[SyncService] Database persisted successfully`);
      } catch (persistError) {
        console.error(`[SyncService] Failed to persist database:`, persistError);
      }
      return {
        projectKey,
        issuesSynced,
        issuesTotalInJira: totalIssues,
        startedAt,
        completedAt: (/* @__PURE__ */ new Date()).toISOString(),
        success: true
      };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      console.error(`[SyncService] syncProject error for ${projectKey}:`, errorMessage, error);
      if (syncHistoryId) {
        try {
          await completeSyncHistory(syncHistoryId, false, issuesSynced, errorMessage);
        } catch (e) {
          console.error(`[SyncService] Failed to complete sync history:`, e);
        }
      }
      return {
        projectKey,
        issuesSynced,
        issuesTotalInJira: 0,
        startedAt,
        completedAt: (/* @__PURE__ */ new Date()).toISOString(),
        success: false,
        errorMessage
      };
    }
  }
  async function syncAllProjects(onProgress) {
    console.log("[SyncService] syncAllProjects called");
    if (isSyncing) {
      console.log("[SyncService] Sync already in progress");
      throw new Error("Sync is already in progress");
    }
    isSyncing = true;
    cancelRequested = false;
    const results = [];
    try {
      const settings = await loadSettings();
      const enabledProjects = settings.projects.filter((p) => p.enabled);
      console.log("[SyncService] Enabled projects:", enabledProjects.map((p) => p.key));
      if (enabledProjects.length === 0) {
        console.log("[SyncService] No projects enabled for sync");
        throw new Error("No projects enabled for sync");
      }
      for (const project of enabledProjects) {
        if (cancelRequested) {
          console.log("[SyncService] Sync cancelled");
          break;
        }
        console.log("[SyncService] Syncing project:", project.key);
        const result = await syncProject(project.key, onProgress);
        console.log("[SyncService] Project sync result:", JSON.stringify(result, null, 2));
        results.push(result);
      }
      return results;
    } finally {
      isSyncing = false;
      currentSyncProgress = null;
    }
  }
  async function getProjectsWithStatus() {
    const settings = await loadSettings();
    let issueCountMap = /* @__PURE__ */ new Map();
    try {
      const { getIssueCount: getIssueCount2 } = await Promise.resolve().then(() => (init_database(), database_exports));
      await initDatabase();
      for (const project of settings.projects) {
        try {
          const count = await getIssueCount2(project.key);
          issueCountMap.set(project.key, count);
        } catch {
        }
      }
    } catch (error) {
      console.warn("Could not load issue counts from database:", error);
    }
    return settings.projects.map((project) => ({
      ...project,
      issueCount: issueCountMap.get(project.key),
      hasCheckpoint: !!project.syncCheckpoint
    }));
  }

  // src/background/index.ts
  init_database();
  chrome.runtime.onMessage.addListener(
    (message, _sender, sendResponse) => {
      if (message.target === "offscreen") {
        return false;
      }
      handleMessage(message).then((response) => sendResponse(response)).catch((error) => {
        console.error("Message handler error:", error);
        sendResponse({
          success: false,
          error: error instanceof Error ? error.message : String(error)
        });
      });
      return true;
    }
  );
  async function handleMessage(message) {
    switch (message.type) {
      case "GET_SETTINGS": {
        const settings = await loadSettings();
        return { success: true, data: settings };
      }
      case "SAVE_SETTINGS": {
        await saveSettings(message.payload);
        return { success: true };
      }
      case "INIT_PROJECTS": {
        await initProjects();
        const projects = await getProjectsWithStatus();
        return { success: true, data: projects };
      }
      case "GET_PROJECTS": {
        const projects = await getProjectsWithStatus();
        return { success: true, data: projects };
      }
      case "ENABLE_PROJECT": {
        const { projectKey, enabled } = message.payload;
        await setProjectEnabled(projectKey, enabled);
        return { success: true };
      }
      case "DISABLE_PROJECT": {
        const { projectKey } = message.payload;
        await setProjectEnabled(projectKey, false);
        return { success: true };
      }
      case "START_SYNC": {
        console.log("[Background] START_SYNC received");
        syncAllProjects((progress) => {
          console.log("[Background] Sync progress:", progress);
          chrome.runtime.sendMessage({
            type: "SYNC_PROGRESS",
            payload: progress
          }).catch(() => {
          });
        }).then((results) => {
          console.log("[Background] Sync complete:", results);
          chrome.runtime.sendMessage({
            type: "SYNC_COMPLETE",
            payload: results
          }).catch(() => {
          });
        }).catch((error) => {
          console.error("[Background] Sync error:", error);
          chrome.runtime.sendMessage({
            type: "SYNC_ERROR",
            payload: error instanceof Error ? error.message : String(error)
          }).catch(() => {
          });
        });
        return { success: true, data: { started: true } };
      }
      case "GET_SYNC_STATUS": {
        const status = getSyncStatus();
        return { success: true, data: status };
      }
      case "CANCEL_SYNC": {
        cancelSync();
        return { success: true };
      }
      case "SEARCH_ISSUES": {
        await initDatabase();
        const params = message.payload;
        const result = await searchIssues(params);
        return { success: true, data: result };
      }
      case "GET_ISSUE": {
        await initDatabase();
        const { issueKey } = message.payload;
        const issue = await getIssue(issueKey);
        return { success: true, data: issue };
      }
      case "GET_ISSUE_HISTORY": {
        await initDatabase();
        const { issueKey, field } = message.payload;
        const history = await getIssueHistory(issueKey, field);
        return { success: true, data: history };
      }
      case "SEND_TO_CLAUDE": {
        const { instructions, issueKey } = message.payload;
        console.log("[Background] SEND_TO_CLAUDE received for issue:", issueKey);
        try {
          await openAiAndPaste("claude", instructions, issueKey);
          return { success: true };
        } catch (error) {
          console.error("[Background] Failed to send to Claude:", error);
          return {
            success: false,
            error: error instanceof Error ? error.message : String(error)
          };
        }
      }
      case "SEND_TO_CHATGPT": {
        const { instructions, issueKey } = message.payload;
        console.log("[Background] SEND_TO_CHATGPT received for issue:", issueKey);
        try {
          await openAiAndPaste("chatgpt", instructions, issueKey);
          return { success: true };
        } catch (error) {
          console.error("[Background] Failed to send to ChatGPT:", error);
          return {
            success: false,
            error: error instanceof Error ? error.message : String(error)
          };
        }
      }
      default:
        return { success: false, error: `Unknown message type: ${message.type}` };
    }
  }
  var AI_CONFIGS = {
    claude: {
      name: "Claude",
      url: "https://claude.ai/new",
      urlPattern: "https://claude.ai/*",
      selectors: [
        '[data-placeholder="How can Claude help you today?"]',
        "[data-placeholder]",
        'div.ProseMirror[contenteditable="true"]',
        'div[contenteditable="true"].ProseMirror',
        '.ProseMirror[contenteditable="true"]',
        'div[contenteditable="true"]',
        '[contenteditable="true"]',
        "textarea[placeholder]",
        "textarea"
      ]
    },
    chatgpt: {
      name: "ChatGPT Codex",
      url: "https://chatgpt.com/codex",
      urlPattern: "https://chatgpt.com/*",
      selectors: [
        "#prompt-textarea",
        'textarea[data-id="root"]',
        "textarea[placeholder]",
        'div[contenteditable="true"]',
        '[contenteditable="true"]',
        "textarea"
      ]
    }
  };
  async function openAiAndPaste(service, instructions, issueKey) {
    const config = AI_CONFIGS[service];
    const fullPrompt = `[JIRA: ${issueKey}]

${instructions}`;
    console.log(`[Background] Opening ${config.name} with prompt length:`, fullPrompt.length);
    try {
      await chrome.storage.local.set({
        aiPendingPrompt: fullPrompt,
        aiPendingService: service,
        aiPendingTimestamp: Date.now()
      });
      console.log("[Background] Stored prompt in storage");
    } catch (storageError) {
      console.error("[Background] Failed to store prompt:", storageError);
      throw storageError;
    }
    let tabs = [];
    try {
      tabs = await chrome.tabs.query({ url: config.urlPattern });
      console.log(`[Background] Found existing ${config.name} tabs:`, tabs.length);
    } catch (queryError) {
      console.error("[Background] Failed to query tabs:", queryError);
      throw queryError;
    }
    let targetTabId;
    if (tabs.length > 0 && tabs[0].id) {
      console.log("[Background] Focusing existing tab:", tabs[0].id, "windowId:", tabs[0].windowId);
      try {
        await chrome.tabs.update(tabs[0].id, { active: true });
        if (tabs[0].windowId) {
          await chrome.windows.update(tabs[0].windowId, { focused: true });
        }
        targetTabId = tabs[0].id;
      } catch (updateError) {
        console.error("[Background] Failed to update tab:", updateError);
        throw updateError;
      }
      await new Promise((resolve) => setTimeout(resolve, 500));
      await injectAiScript(targetTabId, service);
    } else {
      console.log("[Background] Creating new tab:", config.url);
      let tab;
      try {
        tab = await chrome.tabs.create({ url: config.url });
        console.log("[Background] Tab created:", tab.id, tab.url);
      } catch (createError) {
        console.error("[Background] Failed to create tab:", createError);
        throw createError;
      }
      if (!tab.id) {
        throw new Error("Failed to create tab - no tab ID");
      }
      targetTabId = tab.id;
      console.log("[Background] Waiting for tab to load...");
      await new Promise((resolve) => {
        const listener = (tabId, info) => {
          console.log("[Background] Tab updated:", tabId, info.status);
          if (tabId === targetTabId && info.status === "complete") {
            chrome.tabs.onUpdated.removeListener(listener);
            resolve();
          }
        };
        chrome.tabs.onUpdated.addListener(listener);
        setTimeout(() => {
          console.log("[Background] Tab load timeout");
          chrome.tabs.onUpdated.removeListener(listener);
          resolve();
        }, 1e4);
      });
      console.log("[Background] Waiting for page to render...");
      await new Promise((resolve) => setTimeout(resolve, 2e3));
      await injectAiScript(targetTabId, service);
    }
  }
  async function injectAiScript(tabId, service) {
    const config = AI_CONFIGS[service];
    console.log(`[Background] Injecting script into ${config.name} tab:`, tabId);
    try {
      const selectorsJson = JSON.stringify(config.selectors);
      const results = await chrome.scripting.executeScript({
        target: { tabId },
        func: async (selectorsStr) => {
          const selectors = JSON.parse(selectorsStr);
          console.log("[JIRA DB] Injected script running...");
          const result2 = await chrome.storage.local.get(["aiPendingPrompt", "aiPendingService", "aiPendingTimestamp"]);
          const prompt = result2.aiPendingPrompt;
          const timestamp = result2.aiPendingTimestamp;
          if (!prompt || !timestamp || Date.now() - timestamp > 6e4) {
            console.log("[JIRA DB] No pending prompt or expired");
            return { success: false, error: "expired" };
          }
          await chrome.storage.local.remove(["aiPendingPrompt", "aiPendingService", "aiPendingTimestamp"]);
          let inputEl = null;
          for (const selector of selectors) {
            const elements = document.querySelectorAll(selector);
            console.log(`[JIRA DB] Selector "${selector}" found ${elements.length} elements`);
            if (elements.length > 0) {
              inputEl = elements[0];
              console.log("[JIRA DB] Found input with selector:", selector);
              break;
            }
          }
          if (!inputEl) {
            console.error("[JIRA DB] Could not find input element");
            console.log("[JIRA DB] Page URL:", window.location.href);
            console.log("[JIRA DB] Page content preview:", document.body.innerHTML.substring(0, 3e3));
            try {
              await navigator.clipboard.writeText(prompt);
              return { success: false, error: "no_input_clipboard" };
            } catch {
              return { success: false, error: "no_input" };
            }
          }
          inputEl.focus();
          if (inputEl.tagName === "TEXTAREA") {
            const textarea = inputEl;
            const nativeInputValueSetter = Object.getOwnPropertyDescriptor(
              window.HTMLTextAreaElement.prototype,
              "value"
            )?.set;
            if (nativeInputValueSetter) {
              nativeInputValueSetter.call(textarea, prompt);
            } else {
              textarea.value = prompt;
            }
            textarea.dispatchEvent(new Event("input", { bubbles: true }));
            textarea.dispatchEvent(new Event("change", { bubbles: true }));
          } else {
            const selection = window.getSelection();
            const range = document.createRange();
            range.selectNodeContents(inputEl);
            selection?.removeAllRanges();
            selection?.addRange(range);
            document.execCommand("insertText", false, prompt);
          }
          console.log("[JIRA DB] Prompt pasted successfully");
          return { success: true };
        },
        args: [selectorsJson]
      });
      console.log("[Background] Script results:", results);
      const result = results?.[0]?.result;
      if (result && !result.success) {
        if (result.error === "no_input_clipboard") {
          console.log("[Background] Copied to clipboard as fallback");
          chrome.runtime.sendMessage({
            type: "AI_CLIPBOARD_FALLBACK",
            payload: { service }
          }).catch(() => {
          });
        }
      }
    } catch (error) {
      console.error("[Background] Script injection failed:", error);
      throw error;
    }
  }
  console.log("JIRA DB Sync background service started");
})();
