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

// IndexedDB constants for persistence
const IDB_NAME = 'jira-db-storage';
const IDB_STORE = 'database';
const IDB_KEY = 'duckdb-data';
const IDB_JSON_KEY = 'duckdb-json-data';

// IndexedDB helpers for persistence
function openIndexedDB(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(IDB_NAME, 1);
    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve(request.result);
    request.onupgradeneeded = (event) => {
      const db = (event.target as IDBOpenDBRequest).result;
      if (!db.objectStoreNames.contains(IDB_STORE)) {
        db.createObjectStore(IDB_STORE);
      }
    };
  });
}

async function saveToIndexedDB(data: Uint8Array): Promise<void> {
  console.log('[Offscreen] Saving database to IndexedDB...', data.length, 'bytes');
  const idb = await openIndexedDB();
  return new Promise((resolve, reject) => {
    const tx = idb.transaction(IDB_STORE, 'readwrite');
    const store = tx.objectStore(IDB_STORE);
    const request = store.put(data, IDB_KEY);
    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      console.log('[Offscreen] Database saved to IndexedDB');
      resolve();
    };
  });
}

async function loadFromIndexedDB(): Promise<Uint8Array | null> {
  try {
    const idb = await openIndexedDB();
    return new Promise((resolve, reject) => {
      const tx = idb.transaction(IDB_STORE, 'readonly');
      const store = tx.objectStore(IDB_STORE);
      const request = store.get(IDB_KEY);
      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        const data = request.result;
        if (data instanceof Uint8Array) {
          console.log('[Offscreen] Loaded database from IndexedDB:', data.length, 'bytes');
          resolve(data);
        } else {
          console.log('[Offscreen] No saved database found in IndexedDB');
          resolve(null);
        }
      };
    });
  } catch (error) {
    console.error('[Offscreen] Failed to load from IndexedDB:', error);
    return null;
  }
}

// Interface for JSON data export
interface DatabaseExport {
  version: number;
  exportedAt: string;
  projects: Record<string, unknown>[];
  issues: Record<string, unknown>[];
  changeHistory: Record<string, unknown>[];
  syncHistory: Record<string, unknown>[];
}

async function saveJsonToIndexedDB(data: DatabaseExport): Promise<void> {
  console.log('[Offscreen] Saving database JSON to IndexedDB...',
    `projects: ${data.projects.length}, issues: ${data.issues.length}, history: ${data.changeHistory.length}`);
  const idb = await openIndexedDB();
  return new Promise((resolve, reject) => {
    const tx = idb.transaction(IDB_STORE, 'readwrite');
    const store = tx.objectStore(IDB_STORE);
    const request = store.put(data, IDB_JSON_KEY);
    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      console.log('[Offscreen] Database JSON saved to IndexedDB');
      resolve();
    };
  });
}

async function loadJsonFromIndexedDB(): Promise<DatabaseExport | null> {
  try {
    const idb = await openIndexedDB();
    return new Promise((resolve, reject) => {
      const tx = idb.transaction(IDB_STORE, 'readonly');
      const store = tx.objectStore(IDB_STORE);
      const request = store.get(IDB_JSON_KEY);
      request.onerror = () => reject(request.error);
      request.onsuccess = () => {
        const data = request.result;
        if (data && typeof data === 'object' && 'version' in data) {
          console.log('[Offscreen] Loaded database JSON from IndexedDB:',
            `projects: ${data.projects?.length || 0}, issues: ${data.issues?.length || 0}`);
          resolve(data as DatabaseExport);
        } else {
          console.log('[Offscreen] No saved database JSON found in IndexedDB');
          resolve(null);
        }
      };
    });
  } catch (error) {
    console.error('[Offscreen] Failed to load JSON from IndexedDB:', error);
    return null;
  }
}

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
    console.log('[Offscreen] Got local bundles:', JSON.stringify(bundles, null, 2));

    console.log('[Offscreen] Selecting bundle...');
    const bundle = await duckdb.selectBundle(bundles);
    console.log('[Offscreen] Bundle selected:', bundle.mainModule);
    console.log('[Offscreen] Worker URL:', bundle.mainWorker);

    // First, verify the worker script is accessible
    console.log('[Offscreen] Verifying worker script is accessible...');
    try {
      const testFetch = await fetch(bundle.mainWorker!);
      console.log('[Offscreen] Worker script fetch status:', testFetch.status, testFetch.statusText);
      if (!testFetch.ok) {
        throw new Error(`Worker script not accessible: ${testFetch.status}`);
      }
    } catch (fetchError) {
      console.error('[Offscreen] Failed to fetch worker script:', fetchError);
      throw fetchError;
    }

    // Create worker - try different approaches
    console.log('[Offscreen] Creating worker...');
    let worker: Worker;
    try {
      // First try: direct URL
      worker = new Worker(bundle.mainWorker!);
      console.log('[Offscreen] Worker created successfully (direct URL)');
    } catch (workerError) {
      console.error('[Offscreen] Direct worker creation failed:', workerError);

      // Second try: fetch and create blob
      console.log('[Offscreen] Trying blob approach...');
      try {
        const response = await fetch(bundle.mainWorker!);
        const text = await response.text();
        const blob = new Blob([text], { type: 'application/javascript' });
        const blobUrl = URL.createObjectURL(blob);
        worker = new Worker(blobUrl);
        console.log('[Offscreen] Worker created successfully (blob URL)');
      } catch (blobError) {
        console.error('[Offscreen] Blob worker creation failed:', blobError);
        throw blobError;
      }
    }

    const logger = new duckdb.ConsoleLogger();
    db = new duckdb.AsyncDuckDB(logger, worker);

    console.log('[Offscreen] Instantiating DuckDB...');
    await db.instantiate(bundle.mainModule, bundle.pthreadWorker);

    console.log('[Offscreen] Connecting...');
    conn = await db.connect();

    console.log('[Offscreen] Creating tables...');
    await createTables();

    // Restore data from IndexedDB if available
    console.log('[Offscreen] Checking for saved data in IndexedDB...');
    await restoreFromJson();

    // Verify restoration by checking issue count
    try {
      const countResult = await conn.query('SELECT COUNT(*) as count FROM issues');
      const countRow = countResult.toArray()[0] as Record<string, unknown>;
      console.log(`[Offscreen] After restore: ${countRow?.count || 0} issues in database`);

      // Also check for latest updated_at
      const latestResult = await conn.query('SELECT MAX(updated_at) as max_updated FROM issues');
      const latestRow = latestResult.toArray()[0] as Record<string, unknown>;
      console.log(`[Offscreen] Latest updated_at in DB:`, latestRow?.max_updated);
    } catch (e) {
      console.warn('[Offscreen] Could not verify issue count:', e);
    }

    console.log('[Offscreen] DuckDB initialized successfully');
  } catch (error) {
    console.error('[Offscreen] Failed to initialize DuckDB:', error);
    if (error instanceof Error) {
      console.error('[Offscreen] Error name:', error.name);
      console.error('[Offscreen] Error message:', error.message);
      console.error('[Offscreen] Error stack:', error.stack);
    }
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
      created_at TIMESTAMP DEFAULT now(),
      updated_at TIMESTAMP DEFAULT now()
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
      synced_at TIMESTAMP DEFAULT now()
    )
  `);

  // Create sequences for auto-increment IDs
  await runSql(`CREATE SEQUENCE IF NOT EXISTS seq_change_history_id START 1`);
  await runSql(`CREATE SEQUENCE IF NOT EXISTS seq_sync_history_id START 1`);

  // Change history table
  await runSql(`
    CREATE TABLE IF NOT EXISTS issue_change_history (
      id INTEGER DEFAULT nextval('seq_change_history_id') PRIMARY KEY,
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
      id INTEGER DEFAULT nextval('seq_sync_history_id') PRIMARY KEY,
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
      created_at TIMESTAMP DEFAULT now(),
      updated_at TIMESTAMP DEFAULT now(),
      PRIMARY KEY (project_id, name)
    )
  `);

  await runSql(`
    CREATE TABLE IF NOT EXISTS priorities (
      id VARCHAR PRIMARY KEY,
      name VARCHAR UNIQUE NOT NULL,
      description VARCHAR,
      icon_url VARCHAR,
      created_at TIMESTAMP DEFAULT now(),
      updated_at TIMESTAMP DEFAULT now()
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
      created_at TIMESTAMP DEFAULT now(),
      updated_at TIMESTAMP DEFAULT now(),
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

// Convert ISO timestamp string to DuckDB-compatible format
// DuckDB TIMESTAMP is timezone-naive and interprets strings as LOCAL time
// So we must use local time components, not UTC
function escapeTimestamp(isoString: string | null): string {
  if (!isoString) return 'NULL';

  try {
    // Parse the ISO string (JS Date handles timezone correctly)
    const date = new Date(isoString);
    if (isNaN(date.getTime())) {
      console.warn(`[Offscreen] Invalid timestamp: ${isoString}`);
      return 'NULL';
    }

    // Format as DuckDB-compatible timestamp string (in LOCAL time)
    // DuckDB will interpret this as local time, which matches what we want
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    const hours = String(date.getHours()).padStart(2, '0');
    const minutes = String(date.getMinutes()).padStart(2, '0');
    const seconds = String(date.getSeconds()).padStart(2, '0');

    const formatted = `${year}-${month}-${day} ${hours}:${minutes}:${seconds}`;
    return `'${formatted}'`;
  } catch (e) {
    console.warn(`[Offscreen] Failed to parse timestamp: ${isoString}`, e);
    return 'NULL';
  }
}

// Project operations
async function upsertProject(project: JiraProject): Promise<void> {
  const sql = `
    INSERT INTO projects (id, key, name, project_type, updated_at)
    VALUES (${escapeSQL(project.id)}, ${escapeSQL(project.key)}, ${escapeSQL(project.name)}, ${escapeSQL(project.projectTypeKey)}, now())
    ON CONFLICT (id) DO UPDATE SET
      key = excluded.key,
      name = excluded.name,
      project_type = excluded.project_type,
      updated_at = now()
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
      created_at: timestampToISOString(r.created_at),
      updated_at: timestampToISOString(r.updated_at),
    };
  });
}

// Issue operations
async function upsertIssue(issue: JiraIssue): Promise<void> {
  const fields = issue.fields;
  console.log(`[Offscreen] upsertIssue: ${issue.key} created=${fields.created} updated=${fields.updated}`);

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
      ${escapeTimestamp(fields.created)}, ${escapeTimestamp(fields.updated)}, ${escapeSQL(rawData)},
      FALSE, now()
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
      synced_at = now()
  `;
  await runSql(sql);

  // Insert change history
  if (issue.changelog?.histories) {
    for (const history of issue.changelog.histories) {
      for (const item of history.items) {
        // Check if this record already exists (by unique constraint)
        const checkSql = `
          SELECT id FROM issue_change_history
          WHERE issue_id = ${escapeSQL(issue.id)}
            AND history_id = ${escapeSQL(history.id)}
            AND field = ${escapeSQL(item.field)}
        `;
        const existingResult = await conn!.query(checkSql);
        const existingRows = existingResult.toArray();

        if (existingRows.length > 0) {
          // Update existing record
          const updateSql = `
            UPDATE issue_change_history SET
              author_account_id = ${escapeSQL(history.author?.accountId || null)},
              author_display_name = ${escapeSQL(history.author?.displayName || null)},
              field_type = ${escapeSQL(item.fieldtype)},
              from_value = ${escapeSQL(item.from || null)},
              from_string = ${escapeSQL(item.fromString || null)},
              to_value = ${escapeSQL(item.to || null)},
              to_string = ${escapeSQL(item.toString || null)},
              changed_at = ${escapeTimestamp(history.created)}
            WHERE issue_id = ${escapeSQL(issue.id)}
              AND history_id = ${escapeSQL(history.id)}
              AND field = ${escapeSQL(item.field)}
          `;
          await runSql(updateSql);
        } else {
          // Manually calculate next ID to avoid sequence issues after restore
          const maxIdResult = await conn!.query('SELECT COALESCE(MAX(id), 0) + 1 as next_id FROM issue_change_history');
          const maxIdRow = maxIdResult.toArray()[0] as Record<string, unknown>;
          const nextId = Number(maxIdRow.next_id || 1);

          const insertSql = `
            INSERT INTO issue_change_history (
              id, issue_id, issue_key, history_id,
              author_account_id, author_display_name,
              field, field_type, from_value, from_string, to_value, to_string,
              changed_at
            ) VALUES (
              ${nextId}, ${escapeSQL(issue.id)}, ${escapeSQL(issue.key)}, ${escapeSQL(history.id)},
              ${escapeSQL(history.author?.accountId || null)}, ${escapeSQL(history.author?.displayName || null)},
              ${escapeSQL(item.field)}, ${escapeSQL(item.fieldtype)},
              ${escapeSQL(item.from || null)}, ${escapeSQL(item.fromString || null)},
              ${escapeSQL(item.to || null)}, ${escapeSQL(item.toString || null)},
              ${escapeTimestamp(history.created)}
            )
          `;
          await runSql(insertSql);
        }
      }
    }
  }
}

function rowToDbIssue(row: Record<string, unknown>): DbIssue {
  // Debug: log raw timestamp values from DB
  console.log(`[Offscreen] rowToDbIssue: key=${row.key} raw updated_at=`, row.updated_at, `type=${typeof row.updated_at}`);

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
    created_at: timestampToISOString(row.created_at),
    updated_at: timestampToISOString(row.updated_at),
    raw_data: String(row.raw_data || '{}'),
    is_deleted: Boolean(row.is_deleted),
    synced_at: timestampToISOString(row.synced_at),
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

// Helper to convert DuckDB timestamp to ISO string
function timestampToISOString(value: unknown): string {
  if (!value) return '';

  // If it's already a string, return it
  if (typeof value === 'string') {
    console.log(`[Offscreen] timestampToISOString: string "${value}"`);
    return value;
  }

  // If it's a BigInt (microseconds since epoch), convert to milliseconds
  if (typeof value === 'bigint') {
    const ms = Number(value / 1000n);
    const isoStr = new Date(ms).toISOString();
    console.log(`[Offscreen] timestampToISOString: bigint ${value} -> ms ${ms} -> "${isoStr}"`);
    return isoStr;
  }

  // If it's a number (milliseconds or seconds)
  if (typeof value === 'number') {
    // If it's in seconds (< year 3000 in seconds), convert to milliseconds
    if (value < 32503680000) {
      const isoStr = new Date(value * 1000).toISOString();
      console.log(`[Offscreen] timestampToISOString: number(seconds) ${value} -> "${isoStr}"`);
      return isoStr;
    }
    const isoStr = new Date(value).toISOString();
    console.log(`[Offscreen] timestampToISOString: number(ms) ${value} -> "${isoStr}"`);
    return isoStr;
  }

  // If it's a Date object
  if (value instanceof Date) {
    const isoStr = value.toISOString();
    console.log(`[Offscreen] timestampToISOString: Date -> "${isoStr}"`);
    return isoStr;
  }

  // Try to convert to string
  console.log(`[Offscreen] timestampToISOString: unknown type ${typeof value}:`, value);
  return String(value);
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
      changed_at: timestampToISOString(r.changed_at),
    };
  });
}

async function getLatestUpdatedAt(projectKey: string): Promise<string | null> {
  if (!conn) throw new Error('Database not initialized');
  const sql = `SELECT MAX(updated_at) as max_updated FROM issues WHERE project_key = ${escapeSQL(projectKey)} AND is_deleted = FALSE`;
  console.log(`[Offscreen] getLatestUpdatedAt SQL: ${sql}`);
  const result = await conn.query(sql);
  const rows = result.toArray();
  console.log(`[Offscreen] getLatestUpdatedAt rows:`, rows);

  if (rows.length === 0) {
    console.log(`[Offscreen] getLatestUpdatedAt: no rows, returning null`);
    return null;
  }

  const row = rows[0] as Record<string, unknown>;
  console.log(`[Offscreen] getLatestUpdatedAt raw value:`, row.max_updated, `type:`, typeof row.max_updated);

  if (!row.max_updated) {
    console.log(`[Offscreen] getLatestUpdatedAt: max_updated is null/undefined, returning null`);
    return null;
  }

  const isoString = timestampToISOString(row.max_updated);
  console.log(`[Offscreen] getLatestUpdatedAt converted to ISO:`, isoString);
  return isoString;
}

async function getIssueCount(projectKey: string): Promise<number> {
  if (!conn) throw new Error('Database not initialized');
  const sql = `SELECT COUNT(*) as count FROM issues WHERE project_key = ${escapeSQL(projectKey)} AND is_deleted = FALSE`;
  const result = await conn.query(sql);
  const row = result.toArray()[0] as Record<string, unknown>;
  return Number(row?.count || 0);
}

async function startSyncHistory(projectKey: string): Promise<number> {
  if (!conn) throw new Error('Database not initialized');

  // Manually calculate next ID to avoid sequence issues after restore
  const maxIdResult = await conn.query('SELECT COALESCE(MAX(id), 0) + 1 as next_id FROM sync_history');
  const maxIdRow = maxIdResult.toArray()[0] as Record<string, unknown>;
  const nextId = Number(maxIdRow.next_id || 1);

  const sql = `
    INSERT INTO sync_history (id, project_key, started_at, status, issues_synced)
    VALUES (${nextId}, ${escapeSQL(projectKey)}, now(), 'running', 0)
  `;
  await runSql(sql);
  return nextId;
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
      completed_at = now(),
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

// Export all data as JSON for persistence
async function exportToJson(): Promise<DatabaseExport> {
  if (!conn) throw new Error('Database not initialized');

  // Export projects
  const projectsResult = await conn.query('SELECT * FROM projects');
  const projects = projectsResult.toArray().map(row => {
    const r = row as Record<string, unknown>;
    return {
      id: String(r.id || ''),
      key: String(r.key || ''),
      name: String(r.name || ''),
      project_type: r.project_type ? String(r.project_type) : null,
      created_at: timestampToISOString(r.created_at),
      updated_at: timestampToISOString(r.updated_at),
    };
  });

  // Export issues
  const issuesResult = await conn.query('SELECT * FROM issues');
  const issues = issuesResult.toArray().map(row => {
    const r = row as Record<string, unknown>;
    return {
      id: String(r.id || ''),
      key: String(r.key || ''),
      project_id: String(r.project_id || ''),
      project_key: String(r.project_key || ''),
      summary: String(r.summary || ''),
      description: r.description ? String(r.description) : null,
      status: String(r.status || ''),
      status_category: r.status_category ? String(r.status_category) : null,
      priority: r.priority ? String(r.priority) : null,
      issue_type: String(r.issue_type || ''),
      assignee_id: r.assignee_id ? String(r.assignee_id) : null,
      assignee_name: r.assignee_name ? String(r.assignee_name) : null,
      reporter_id: r.reporter_id ? String(r.reporter_id) : null,
      reporter_name: r.reporter_name ? String(r.reporter_name) : null,
      labels: r.labels ? String(r.labels) : null,
      components: r.components ? String(r.components) : null,
      fix_versions: r.fix_versions ? String(r.fix_versions) : null,
      created_at: timestampToISOString(r.created_at),
      updated_at: timestampToISOString(r.updated_at),
      raw_data: String(r.raw_data || '{}'),
      is_deleted: Boolean(r.is_deleted),
      synced_at: timestampToISOString(r.synced_at),
    };
  });

  // Export change history
  const historyResult = await conn.query('SELECT * FROM issue_change_history');
  const changeHistory = historyResult.toArray().map(row => {
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
      changed_at: timestampToISOString(r.changed_at),
    };
  });

  // Export sync history
  const syncResult = await conn.query('SELECT * FROM sync_history');
  const syncHistory = syncResult.toArray().map(row => {
    const r = row as Record<string, unknown>;
    return {
      id: Number(r.id || 0),
      project_key: String(r.project_key || ''),
      started_at: timestampToISOString(r.started_at),
      completed_at: r.completed_at ? timestampToISOString(r.completed_at) : null,
      status: String(r.status || ''),
      issues_synced: Number(r.issues_synced || 0),
      error_message: r.error_message ? String(r.error_message) : null,
    };
  });

  return {
    version: 1,
    exportedAt: new Date().toISOString(),
    projects,
    issues,
    changeHistory,
    syncHistory,
  };
}

// Restore data from JSON
async function restoreFromJson(): Promise<void> {
  const data = await loadJsonFromIndexedDB();
  if (!data) {
    console.log('[Offscreen] No saved data to restore');
    return;
  }

  console.log('[Offscreen] Restoring data from JSON...');

  // Restore projects
  for (const project of data.projects) {
    const sql = `
      INSERT INTO projects (id, key, name, project_type, created_at, updated_at)
      VALUES (${escapeSQL(project.id as string)}, ${escapeSQL(project.key as string)},
              ${escapeSQL(project.name as string)}, ${escapeSQL(project.project_type as string | null)},
              ${escapeTimestamp(project.created_at as string)}, ${escapeTimestamp(project.updated_at as string)})
      ON CONFLICT (id) DO NOTHING
    `;
    try {
      await runSql(sql);
    } catch (error) {
      console.warn('[Offscreen] Failed to restore project:', project.key, error);
    }
  }

  // Restore issues
  for (const issue of data.issues) {
    const sql = `
      INSERT INTO issues (
        id, key, project_id, project_key, summary, description,
        status, status_category, priority, issue_type,
        assignee_id, assignee_name, reporter_id, reporter_name,
        labels, components, fix_versions,
        created_at, updated_at, raw_data, is_deleted, synced_at
      ) VALUES (
        ${escapeSQL(issue.id as string)}, ${escapeSQL(issue.key as string)},
        ${escapeSQL(issue.project_id as string)}, ${escapeSQL(issue.project_key as string)},
        ${escapeSQL(issue.summary as string)}, ${escapeSQL(issue.description as string | null)},
        ${escapeSQL(issue.status as string)}, ${escapeSQL(issue.status_category as string | null)},
        ${escapeSQL(issue.priority as string | null)}, ${escapeSQL(issue.issue_type as string)},
        ${escapeSQL(issue.assignee_id as string | null)}, ${escapeSQL(issue.assignee_name as string | null)},
        ${escapeSQL(issue.reporter_id as string | null)}, ${escapeSQL(issue.reporter_name as string | null)},
        ${escapeSQL(issue.labels as string | null)}, ${escapeSQL(issue.components as string | null)},
        ${escapeSQL(issue.fix_versions as string | null)},
        ${escapeTimestamp(issue.created_at as string)}, ${escapeTimestamp(issue.updated_at as string)},
        ${escapeSQL(issue.raw_data as string)}, ${issue.is_deleted ? 'TRUE' : 'FALSE'},
        ${escapeTimestamp(issue.synced_at as string)}
      )
      ON CONFLICT (id) DO NOTHING
    `;
    try {
      await runSql(sql);
    } catch (error) {
      console.warn('[Offscreen] Failed to restore issue:', issue.key, error);
    }
  }

  // Restore change history
  for (const history of data.changeHistory) {
    const sql = `
      INSERT INTO issue_change_history (
        id, issue_id, issue_key, history_id,
        author_account_id, author_display_name,
        field, field_type, from_value, from_string, to_value, to_string,
        changed_at
      ) VALUES (
        ${history.id}, ${escapeSQL(history.issue_id as string)}, ${escapeSQL(history.issue_key as string)},
        ${escapeSQL(history.history_id as string)},
        ${escapeSQL(history.author_account_id as string | null)}, ${escapeSQL(history.author_display_name as string | null)},
        ${escapeSQL(history.field as string)}, ${escapeSQL(history.field_type as string)},
        ${escapeSQL(history.from_value as string | null)}, ${escapeSQL(history.from_string as string | null)},
        ${escapeSQL(history.to_value as string | null)}, ${escapeSQL(history.to_string as string | null)},
        ${escapeTimestamp(history.changed_at as string)}
      )
      ON CONFLICT (issue_id, history_id, field) DO NOTHING
    `;
    try {
      await runSql(sql);
    } catch (error) {
      console.warn('[Offscreen] Failed to restore change history:', history.issue_key, error);
    }
  }

  // Restore sync history
  for (const sync of data.syncHistory) {
    const sql = `
      INSERT INTO sync_history (
        id, project_key, started_at, completed_at, status, issues_synced, error_message
      ) VALUES (
        ${sync.id}, ${escapeSQL(sync.project_key as string)},
        ${escapeTimestamp(sync.started_at as string)}, ${escapeTimestamp(sync.completed_at as string | null)},
        ${escapeSQL(sync.status as string)}, ${sync.issues_synced},
        ${escapeSQL(sync.error_message as string | null)}
      )
      ON CONFLICT DO NOTHING
    `;
    try {
      await runSql(sql);
    } catch (error) {
      console.warn('[Offscreen] Failed to restore sync history:', error);
    }
  }

  // Update sequences to avoid conflicts
  try {
    const maxHistoryId = data.changeHistory.length > 0
      ? Math.max(...data.changeHistory.map(h => h.id as number)) + 1
      : 1;
    await runSql(`DROP SEQUENCE IF EXISTS seq_change_history_id`);
    await runSql(`CREATE SEQUENCE seq_change_history_id START ${maxHistoryId}`);

    const maxSyncId = data.syncHistory.length > 0
      ? Math.max(...data.syncHistory.map(s => s.id as number)) + 1
      : 1;
    await runSql(`DROP SEQUENCE IF EXISTS seq_sync_history_id`);
    await runSql(`CREATE SEQUENCE seq_sync_history_id START ${maxSyncId}`);
  } catch (error) {
    console.warn('[Offscreen] Failed to update sequences:', error);
  }

  console.log(`[Offscreen] Restored ${data.projects.length} projects, ${data.issues.length} issues, ${data.changeHistory.length} change history records`);
}

// Persist database to IndexedDB by exporting all data as JSON
async function persistDatabase(): Promise<void> {
  console.log('[Offscreen] Persisting database to IndexedDB...');
  try {
    const data = await exportToJson();
    console.log(`[Offscreen] Exporting: ${data.projects.length} projects, ${data.issues.length} issues`);
    if (data.issues.length > 0) {
      // Log the latest updated_at from exported data
      const latestUpdated = data.issues.reduce((max, issue) => {
        const updated = issue.updated_at as string;
        return updated > max ? updated : max;
      }, '');
      console.log(`[Offscreen] Latest updated_at in export: ${latestUpdated}`);
    }
    await saveJsonToIndexedDB(data);
    console.log('[Offscreen] Database persisted successfully');
  } catch (error) {
    console.error('[Offscreen] Failed to persist database:', error);
    throw error;
  }
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

    case 'PERSIST_DATABASE':
      await initDatabase();
      await persistDatabase();
      return null;

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
