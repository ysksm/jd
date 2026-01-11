/**
 * Offscreen document for DuckDB WASM operations.
 *
 * This runs in a document context where Web Workers are available,
 * allowing DuckDB WASM to function properly.
 */

// Early logging to verify script is running
console.log('[Offscreen] Script starting...');

// Wrap everything in try-catch to catch initialization errors
try {
  console.log('[Offscreen] About to import DuckDB WASM...');
} catch (e) {
  console.error('[Offscreen] Early initialization error:', e);
}

import * as duckdb from '@duckdb/duckdb-wasm';
import type {
  DbIssue,
  DbProject,
  DbChangeHistory,
  SearchParams,
  SearchResult,
  JiraIssue,
  JiraProject,
} from '../lib/types';

// Database state
let db: duckdb.AsyncDuckDB | null = null;
let conn: duckdb.AsyncDuckDBConnection | null = null;

// Message types for offscreen communication
interface OffscreenMessage {
  target: 'offscreen';
  action: string;
  payload?: unknown;
  requestId: string;
}

// Get local bundle paths for DuckDB WASM files
function getLocalBundles(): duckdb.DuckDBBundles {
  // Use chrome.runtime.getURL to get the correct extension URLs
  const baseUrl = chrome.runtime.getURL('dist/');

  return {
    mvp: {
      mainModule: baseUrl + 'duckdb-mvp.wasm',
      mainWorker: baseUrl + 'duckdb-browser-mvp.worker.js',
    },
    eh: {
      mainModule: baseUrl + 'duckdb-eh.wasm',
      mainWorker: baseUrl + 'duckdb-browser-eh.worker.js',
    },
  };
}

// Initialize DuckDB
async function initDatabase(): Promise<void> {
  if (db && conn) {
    console.log('[Offscreen] Database already initialized, skipping');
    return;
  }

  console.log('[Offscreen] Initializing DuckDB WASM...');

  try {
    // Use local bundles instead of CDN (required for Manifest V3 CSP)
    const bundles = getLocalBundles();
    console.log('[Offscreen] Got local bundles, selecting...');
    const bundle = await duckdb.selectBundle(bundles);
    console.log('[Offscreen] Bundle selected:', bundle.mainModule);

    // Load worker directly from extension URL (no blob needed)
    console.log('[Offscreen] Creating worker from:', bundle.mainWorker);
    const worker = new Worker(bundle.mainWorker!);
    const logger = new duckdb.ConsoleLogger();
    db = new duckdb.AsyncDuckDB(logger, worker);

    console.log('[Offscreen] Instantiating DuckDB...');
    await db.instantiate(bundle.mainModule, bundle.pthreadWorker);

    console.log('[Offscreen] Connecting...');
    conn = await db.connect();

    console.log('[Offscreen] Creating tables...');
    await createTables();
    console.log('[Offscreen] DuckDB initialized successfully');
  } catch (error) {
    console.error('[Offscreen] Failed to initialize DuckDB:', error);
    // Reset state on failure
    db = null;
    conn = null;
    throw error;
  }
}

// Helper to run a SQL statement
async function runSql(sql: string): Promise<void> {
  if (!conn) throw new Error('Database not initialized');
  await conn.query(sql);
}

async function createTables(): Promise<void> {
  // Projects table
  await runSql(`
    CREATE TABLE IF NOT EXISTS projects (
      id VARCHAR PRIMARY KEY,
      key VARCHAR UNIQUE NOT NULL,
      name VARCHAR NOT NULL,
      project_type VARCHAR,
      created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
      updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    )
  `);

  // Issues table
  await runSql(`
    CREATE TABLE IF NOT EXISTS issues (
      id VARCHAR PRIMARY KEY,
      key VARCHAR UNIQUE NOT NULL,
      project_id VARCHAR NOT NULL,
      project_key VARCHAR NOT NULL,
      summary VARCHAR NOT NULL,
      description VARCHAR,
      status VARCHAR NOT NULL,
      status_category VARCHAR,
      priority VARCHAR,
      issue_type VARCHAR NOT NULL,
      assignee_id VARCHAR,
      assignee_name VARCHAR,
      reporter_id VARCHAR,
      reporter_name VARCHAR,
      labels VARCHAR,
      components VARCHAR,
      fix_versions VARCHAR,
      created_at TIMESTAMP NOT NULL,
      updated_at TIMESTAMP NOT NULL,
      raw_data VARCHAR NOT NULL,
      is_deleted BOOLEAN DEFAULT FALSE,
      synced_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    )
  `);

  // Change history table
  await runSql(`
    CREATE TABLE IF NOT EXISTS issue_change_history (
      id INTEGER PRIMARY KEY,
      issue_id VARCHAR NOT NULL,
      issue_key VARCHAR NOT NULL,
      history_id VARCHAR NOT NULL,
      author_account_id VARCHAR,
      author_display_name VARCHAR,
      field VARCHAR NOT NULL,
      field_type VARCHAR NOT NULL,
      from_value VARCHAR,
      from_string VARCHAR,
      to_value VARCHAR,
      to_string VARCHAR,
      changed_at TIMESTAMP NOT NULL,
      UNIQUE(issue_id, history_id, field)
    )
  `);

  // Sync history table
  await runSql(`
    CREATE TABLE IF NOT EXISTS sync_history (
      id INTEGER PRIMARY KEY,
      project_key VARCHAR NOT NULL,
      started_at TIMESTAMP NOT NULL,
      completed_at TIMESTAMP,
      status VARCHAR NOT NULL,
      issues_synced INTEGER DEFAULT 0,
      error_message VARCHAR
    )
  `);

  // Metadata tables
  await runSql(`
    CREATE TABLE IF NOT EXISTS statuses (
      project_id VARCHAR NOT NULL,
      id VARCHAR NOT NULL,
      name VARCHAR NOT NULL,
      description VARCHAR,
      category VARCHAR,
      created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
      updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
      PRIMARY KEY (project_id, name)
    )
  `);

  await runSql(`
    CREATE TABLE IF NOT EXISTS priorities (
      id VARCHAR PRIMARY KEY,
      name VARCHAR UNIQUE NOT NULL,
      description VARCHAR,
      icon_url VARCHAR,
      created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
      updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    )
  `);

  await runSql(`
    CREATE TABLE IF NOT EXISTS issue_types (
      project_id VARCHAR NOT NULL,
      id VARCHAR NOT NULL,
      name VARCHAR NOT NULL,
      description VARCHAR,
      icon_url VARCHAR,
      subtask BOOLEAN DEFAULT FALSE,
      created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
      updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
      PRIMARY KEY (project_id, name)
    )
  `);

  // Create indexes
  await runSql(`CREATE INDEX IF NOT EXISTS idx_issues_summary ON issues(summary)`);
  await runSql(`CREATE INDEX IF NOT EXISTS idx_issues_project_key ON issues(project_key)`);
  await runSql(`CREATE INDEX IF NOT EXISTS idx_issues_status ON issues(status)`);
  await runSql(`CREATE INDEX IF NOT EXISTS idx_issues_updated_at ON issues(updated_at)`);
}

// Helper to convert description to string
function descriptionToString(description: unknown): string | null {
  if (!description) return null;
  if (typeof description === 'string') return description;
  if (typeof description === 'object' && description !== null) {
    try {
      return extractTextFromAdf(description);
    } catch {
      return JSON.stringify(description);
    }
  }
  return null;
}

function extractTextFromAdf(node: unknown): string {
  if (!node || typeof node !== 'object') return '';
  const n = node as Record<string, unknown>;

  if (n.type === 'text' && typeof n.text === 'string') {
    return n.text;
  }

  if (Array.isArray(n.content)) {
    return n.content.map(extractTextFromAdf).join('');
  }

  return '';
}

// Escape single quotes for SQL strings
function escapeSQL(value: string | null): string {
  if (value === null) return 'NULL';
  return `'${value.replace(/'/g, "''")}'`;
}

// Project operations
async function upsertProject(project: JiraProject): Promise<void> {
  const sql = `
    INSERT INTO projects (id, key, name, project_type, updated_at)
    VALUES (${escapeSQL(project.id)}, ${escapeSQL(project.key)}, ${escapeSQL(project.name)}, ${escapeSQL(project.projectTypeKey)}, CURRENT_TIMESTAMP)
    ON CONFLICT (id) DO UPDATE SET
      key = excluded.key,
      name = excluded.name,
      project_type = excluded.project_type,
      updated_at = CURRENT_TIMESTAMP
  `;
  await runSql(sql);
}

async function getProjects(): Promise<DbProject[]> {
  if (!conn) throw new Error('Database not initialized');
  const result = await conn.query('SELECT * FROM projects ORDER BY key');
  return result.toArray().map(row => {
    const r = row as Record<string, unknown>;
    return {
      id: String(r.id || ''),
      key: String(r.key || ''),
      name: String(r.name || ''),
      project_type: String(r.project_type || ''),
      created_at: String(r.created_at || ''),
      updated_at: String(r.updated_at || ''),
    };
  });
}

// Issue operations
async function upsertIssue(issue: JiraIssue): Promise<void> {
  const fields = issue.fields;
  const description = descriptionToString(fields.description);
  const labels = fields.labels ? JSON.stringify(fields.labels) : null;
  const components = fields.components ? JSON.stringify(fields.components.map(c => c.name)) : null;
  const fixVersions = fields.fixVersions ? JSON.stringify(fields.fixVersions.map(v => v.name)) : null;
  const rawData = JSON.stringify(issue);

  const sql = `
    INSERT INTO issues (
      id, key, project_id, project_key, summary, description,
      status, status_category, priority, issue_type,
      assignee_id, assignee_name, reporter_id, reporter_name,
      labels, components, fix_versions,
      created_at, updated_at, raw_data, is_deleted, synced_at
    ) VALUES (
      ${escapeSQL(issue.id)}, ${escapeSQL(issue.key)}, ${escapeSQL(fields.project.id)}, ${escapeSQL(fields.project.key)},
      ${escapeSQL(fields.summary)}, ${escapeSQL(description)},
      ${escapeSQL(fields.status.name)}, ${escapeSQL(fields.status.statusCategory?.name || null)},
      ${escapeSQL(fields.priority?.name || null)}, ${escapeSQL(fields.issuetype.name)},
      ${escapeSQL(fields.assignee?.accountId || null)}, ${escapeSQL(fields.assignee?.displayName || null)},
      ${escapeSQL(fields.reporter?.accountId || null)}, ${escapeSQL(fields.reporter?.displayName || null)},
      ${escapeSQL(labels)}, ${escapeSQL(components)}, ${escapeSQL(fixVersions)},
      ${escapeSQL(fields.created)}, ${escapeSQL(fields.updated)}, ${escapeSQL(rawData)},
      FALSE, CURRENT_TIMESTAMP
    )
    ON CONFLICT (id) DO UPDATE SET
      key = excluded.key,
      project_id = excluded.project_id,
      project_key = excluded.project_key,
      summary = excluded.summary,
      description = excluded.description,
      status = excluded.status,
      status_category = excluded.status_category,
      priority = excluded.priority,
      issue_type = excluded.issue_type,
      assignee_id = excluded.assignee_id,
      assignee_name = excluded.assignee_name,
      reporter_id = excluded.reporter_id,
      reporter_name = excluded.reporter_name,
      labels = excluded.labels,
      components = excluded.components,
      fix_versions = excluded.fix_versions,
      created_at = excluded.created_at,
      updated_at = excluded.updated_at,
      raw_data = excluded.raw_data,
      is_deleted = FALSE,
      synced_at = CURRENT_TIMESTAMP
  `;
  await runSql(sql);

  // Insert change history
  if (issue.changelog?.histories) {
    for (const history of issue.changelog.histories) {
      for (const item of history.items) {
        const historySql = `
          INSERT INTO issue_change_history (
            issue_id, issue_key, history_id,
            author_account_id, author_display_name,
            field, field_type, from_value, from_string, to_value, to_string,
            changed_at
          ) VALUES (
            ${escapeSQL(issue.id)}, ${escapeSQL(issue.key)}, ${escapeSQL(history.id)},
            ${escapeSQL(history.author?.accountId || null)}, ${escapeSQL(history.author?.displayName || null)},
            ${escapeSQL(item.field)}, ${escapeSQL(item.fieldtype)},
            ${escapeSQL(item.from || null)}, ${escapeSQL(item.fromString || null)},
            ${escapeSQL(item.to || null)}, ${escapeSQL(item.toString || null)},
            ${escapeSQL(history.created)}
          )
          ON CONFLICT (issue_id, history_id, field) DO UPDATE SET
            author_account_id = excluded.author_account_id,
            author_display_name = excluded.author_display_name,
            field_type = excluded.field_type,
            from_value = excluded.from_value,
            from_string = excluded.from_string,
            to_value = excluded.to_value,
            to_string = excluded.to_string,
            changed_at = excluded.changed_at
        `;
        await runSql(historySql);
      }
    }
  }
}

function rowToDbIssue(row: Record<string, unknown>): DbIssue {
  return {
    id: String(row.id || ''),
    key: String(row.key || ''),
    project_id: String(row.project_id || ''),
    project_key: String(row.project_key || ''),
    summary: String(row.summary || ''),
    description: row.description ? String(row.description) : null,
    status: String(row.status || ''),
    status_category: row.status_category ? String(row.status_category) : null,
    priority: row.priority ? String(row.priority) : null,
    issue_type: String(row.issue_type || ''),
    assignee_id: row.assignee_id ? String(row.assignee_id) : null,
    assignee_name: row.assignee_name ? String(row.assignee_name) : null,
    reporter_id: row.reporter_id ? String(row.reporter_id) : null,
    reporter_name: row.reporter_name ? String(row.reporter_name) : null,
    labels: row.labels ? String(row.labels) : null,
    components: row.components ? String(row.components) : null,
    fix_versions: row.fix_versions ? String(row.fix_versions) : null,
    created_at: String(row.created_at || ''),
    updated_at: String(row.updated_at || ''),
    raw_data: String(row.raw_data || '{}'),
    is_deleted: Boolean(row.is_deleted),
    synced_at: String(row.synced_at || ''),
  };
}

async function getIssue(key: string): Promise<DbIssue | null> {
  if (!conn) throw new Error('Database not initialized');
  const sql = `SELECT * FROM issues WHERE key = ${escapeSQL(key)} AND is_deleted = FALSE`;
  const result = await conn.query(sql);
  const rows = result.toArray();
  if (rows.length === 0) return null;
  return rowToDbIssue(rows[0] as Record<string, unknown>);
}

async function searchIssues(params: SearchParams): Promise<SearchResult> {
  if (!conn) throw new Error('Database not initialized');

  const conditions: string[] = ['is_deleted = FALSE'];

  if (params.query) {
    const pattern = `%${params.query.replace(/'/g, "''")}%`;
    conditions.push(`(summary ILIKE '${pattern}' OR description ILIKE '${pattern}' OR key ILIKE '${pattern}')`);
  }

  if (params.project) {
    conditions.push(`project_key = ${escapeSQL(params.project)}`);
  }

  if (params.status) {
    conditions.push(`status = ${escapeSQL(params.status)}`);
  }

  if (params.assignee) {
    const pattern = `%${params.assignee.replace(/'/g, "''")}%`;
    conditions.push(`assignee_name ILIKE '${pattern}'`);
  }

  const whereClause = conditions.length > 0 ? `WHERE ${conditions.join(' AND ')}` : '';
  const limit = params.limit || 50;
  const offset = params.offset || 0;

  // Get total count
  const countSql = `SELECT COUNT(*) as count FROM issues ${whereClause}`;
  const countResult = await conn.query(countSql);
  const countRow = countResult.toArray()[0] as Record<string, unknown>;
  const total = Number(countRow?.count || 0);

  // Get issues
  const issuesSql = `SELECT * FROM issues ${whereClause} ORDER BY updated_at DESC LIMIT ${limit} OFFSET ${offset}`;
  const issuesResult = await conn.query(issuesSql);

  return {
    issues: issuesResult.toArray().map(row => rowToDbIssue(row as Record<string, unknown>)),
    total,
  };
}

async function getIssueHistory(issueKey: string, field?: string): Promise<DbChangeHistory[]> {
  if (!conn) throw new Error('Database not initialized');

  let sql = `SELECT * FROM issue_change_history WHERE issue_key = ${escapeSQL(issueKey)}`;

  if (field) {
    sql += ` AND field = ${escapeSQL(field)}`;
  }

  sql += ' ORDER BY changed_at DESC';

  const result = await conn.query(sql);
  return result.toArray().map(row => {
    const r = row as Record<string, unknown>;
    return {
      id: Number(r.id || 0),
      issue_id: String(r.issue_id || ''),
      issue_key: String(r.issue_key || ''),
      history_id: String(r.history_id || ''),
      author_account_id: r.author_account_id ? String(r.author_account_id) : null,
      author_display_name: r.author_display_name ? String(r.author_display_name) : null,
      field: String(r.field || ''),
      field_type: String(r.field_type || ''),
      from_value: r.from_value ? String(r.from_value) : null,
      from_string: r.from_string ? String(r.from_string) : null,
      to_value: r.to_value ? String(r.to_value) : null,
      to_string: r.to_string ? String(r.to_string) : null,
      changed_at: String(r.changed_at || ''),
    };
  });
}

async function getLatestUpdatedAt(projectKey: string): Promise<string | null> {
  if (!conn) throw new Error('Database not initialized');
  const sql = `SELECT MAX(updated_at)::VARCHAR as max_updated FROM issues WHERE project_key = ${escapeSQL(projectKey)} AND is_deleted = FALSE`;
  const result = await conn.query(sql);
  const rows = result.toArray();
  if (rows.length === 0) return null;
  const row = rows[0] as Record<string, unknown>;
  return row.max_updated ? String(row.max_updated) : null;
}

async function getIssueCount(projectKey: string): Promise<number> {
  if (!conn) throw new Error('Database not initialized');
  const sql = `SELECT COUNT(*) as count FROM issues WHERE project_key = ${escapeSQL(projectKey)} AND is_deleted = FALSE`;
  const result = await conn.query(sql);
  const row = result.toArray()[0] as Record<string, unknown>;
  return Number(row?.count || 0);
}

async function startSyncHistory(projectKey: string): Promise<number> {
  const sql = `
    INSERT INTO sync_history (project_key, started_at, status, issues_synced)
    VALUES (${escapeSQL(projectKey)}, CURRENT_TIMESTAMP, 'running', 0)
  `;
  await runSql(sql);

  if (!conn) throw new Error('Database not initialized');
  const idResult = await conn.query(`SELECT MAX(id) as id FROM sync_history WHERE project_key = ${escapeSQL(projectKey)}`);
  const row = idResult.toArray()[0] as Record<string, unknown>;
  return Number(row?.id || 0);
}

async function completeSyncHistory(
  id: number,
  success: boolean,
  issuesSynced: number,
  errorMessage?: string
): Promise<void> {
  const status = success ? 'completed' : 'failed';
  const sql = `
    UPDATE sync_history SET
      completed_at = CURRENT_TIMESTAMP,
      status = ${escapeSQL(status)},
      issues_synced = ${issuesSynced},
      error_message = ${escapeSQL(errorMessage || null)}
    WHERE id = ${id}
  `;
  await runSql(sql);
}

async function updateSyncHistoryProgress(id: number, issuesSynced: number): Promise<void> {
  const sql = `UPDATE sync_history SET issues_synced = ${issuesSynced} WHERE id = ${id}`;
  await runSql(sql);
}

async function getProjectStatuses(projectKey: string): Promise<string[]> {
  if (!conn) throw new Error('Database not initialized');
  const sql = `SELECT DISTINCT status FROM issues WHERE project_key = ${escapeSQL(projectKey)} AND is_deleted = FALSE ORDER BY status`;
  const result = await conn.query(sql);
  return result.toArray().map(row => String((row as Record<string, unknown>).status || ''));
}

async function exportDatabase(): Promise<Uint8Array> {
  if (!db) throw new Error('Database not initialized');
  return await db.copyFileToBuffer('jira.db');
}

// Message handler
chrome.runtime.onMessage.addListener(
  (
    message: OffscreenMessage,
    _sender: chrome.runtime.MessageSender,
    sendResponse: (response: { success: boolean; data?: unknown; error?: string }) => void
  ) => {
    // Only handle messages targeted at offscreen
    if (message.target !== 'offscreen') return;

    console.log('[Offscreen] Received message:', message.action);

    handleAction(message.action, message.payload)
      .then((data) => {
        console.log('[Offscreen] Action completed:', message.action);
        sendResponse({ success: true, data });
      })
      .catch((error) => {
        console.error('[Offscreen] Action failed:', message.action, error);
        sendResponse({
          success: false,
          error: error instanceof Error ? error.message : String(error),
        });
      });

    // Return true to indicate async response
    return true;
  }
);

async function handleAction(action: string, payload: unknown): Promise<unknown> {
  switch (action) {
    case 'PING':
      // Simple ping to check if offscreen document is ready
      return 'PONG';

    case 'INIT_DATABASE':
      await initDatabase();
      return null;

    case 'UPSERT_PROJECT':
      await initDatabase();
      await upsertProject(payload as JiraProject);
      return null;

    case 'GET_PROJECTS':
      await initDatabase();
      return await getProjects();

    case 'UPSERT_ISSUE':
      await initDatabase();
      await upsertIssue(payload as JiraIssue);
      return null;

    case 'GET_ISSUE': {
      await initDatabase();
      const { issueKey } = payload as { issueKey: string };
      return await getIssue(issueKey);
    }

    case 'SEARCH_ISSUES':
      await initDatabase();
      return await searchIssues(payload as SearchParams);

    case 'GET_ISSUE_HISTORY': {
      await initDatabase();
      const { issueKey, field } = payload as { issueKey: string; field?: string };
      return await getIssueHistory(issueKey, field);
    }

    case 'GET_LATEST_UPDATED_AT': {
      await initDatabase();
      const { projectKey } = payload as { projectKey: string };
      return await getLatestUpdatedAt(projectKey);
    }

    case 'GET_ISSUE_COUNT': {
      await initDatabase();
      const { projectKey } = payload as { projectKey: string };
      return await getIssueCount(projectKey);
    }

    case 'START_SYNC_HISTORY': {
      await initDatabase();
      const { projectKey } = payload as { projectKey: string };
      return await startSyncHistory(projectKey);
    }

    case 'COMPLETE_SYNC_HISTORY': {
      await initDatabase();
      const { id, success, issuesSynced, errorMessage } = payload as {
        id: number;
        success: boolean;
        issuesSynced: number;
        errorMessage?: string;
      };
      await completeSyncHistory(id, success, issuesSynced, errorMessage);
      return null;
    }

    case 'UPDATE_SYNC_HISTORY_PROGRESS': {
      await initDatabase();
      const { id, issuesSynced } = payload as { id: number; issuesSynced: number };
      await updateSyncHistoryProgress(id, issuesSynced);
      return null;
    }

    case 'GET_PROJECT_STATUSES': {
      await initDatabase();
      const { projectKey } = payload as { projectKey: string };
      return await getProjectStatuses(projectKey);
    }

    case 'EXPORT_DATABASE':
      await initDatabase();
      return await exportDatabase();

    default:
      throw new Error(`Unknown action: ${action}`);
  }
}

// Set up global error handler to catch any uncaught errors
window.onerror = (message, source, lineno, colno, error) => {
  console.error('[Offscreen] Global error:', { message, source, lineno, colno, error });
  return true;
};

window.addEventListener('unhandledrejection', (event) => {
  console.error('[Offscreen] Unhandled promise rejection:', event.reason);
});

console.log('[Offscreen] Document loaded and ready');
