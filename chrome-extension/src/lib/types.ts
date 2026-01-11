// Settings types
export type AuthMethod = 'browser' | 'api_token';

export interface JiraSettings {
  endpoint: string;
  authMethod: AuthMethod;
  username: string;  // Only used for api_token auth
  apiKey: string;    // Only used for api_token auth
}

export interface SyncSettings {
  incrementalSyncEnabled: boolean;
  incrementalSyncMarginMinutes: number;
  batchSize: number;
}

export interface Settings {
  jira: JiraSettings;
  sync: SyncSettings;
  projects: ProjectConfig[];
}

export interface ProjectConfig {
  key: string;
  name: string;
  enabled: boolean;
  lastSyncedAt?: string;
  syncCheckpoint?: SyncCheckpoint;
}

export interface SyncCheckpoint {
  lastProcessedUpdatedAt: string;
  startPosition: number;
  totalIssues: number;
}

// JIRA API types
export interface JiraProject {
  id: string;
  key: string;
  name: string;
  projectTypeKey: string;
  avatarUrls?: Record<string, string>;
}

export interface JiraIssue {
  id: string;
  key: string;
  self: string;
  fields: JiraIssueFields;
  changelog?: JiraChangelog;
}

export interface JiraIssueFields {
  summary: string;
  description?: unknown;
  status: JiraStatus;
  priority?: JiraPriority;
  issuetype: JiraIssueType;
  project: JiraProjectRef;
  assignee?: JiraUser;
  reporter?: JiraUser;
  created: string;
  updated: string;
  labels?: string[];
  components?: JiraComponent[];
  fixVersions?: JiraVersion[];
  [key: string]: unknown;
}

export interface JiraProjectRef {
  id: string;
  key: string;
  name: string;
}

export interface JiraStatus {
  id: string;
  name: string;
  description?: string;
  statusCategory?: {
    key: string;
    name: string;
  };
}

export interface JiraPriority {
  id: string;
  name: string;
  iconUrl?: string;
}

export interface JiraIssueType {
  id: string;
  name: string;
  description?: string;
  iconUrl?: string;
  subtask: boolean;
}

export interface JiraUser {
  accountId: string;
  displayName: string;
  emailAddress?: string;
  avatarUrls?: Record<string, string>;
}

export interface JiraComponent {
  id: string;
  name: string;
  description?: string;
}

export interface JiraVersion {
  id: string;
  name: string;
  description?: string;
  released: boolean;
  releaseDate?: string;
}

export interface JiraChangelog {
  startAt: number;
  maxResults: number;
  total: number;
  histories: JiraChangeHistory[];
}

export interface JiraChangeHistory {
  id: string;
  author: JiraUser;
  created: string;
  items: JiraChangeItem[];
}

export interface JiraChangeItem {
  field: string;
  fieldtype: string;
  from?: string;
  fromString?: string;
  to?: string;
  toString?: string;
}

export interface JiraSearchResponse {
  expand: string;
  startAt: number;
  maxResults: number;
  total: number;
  issues: JiraIssue[];
}

// Database types
export interface DbIssue {
  id: string;
  key: string;
  project_id: string;
  project_key: string;
  summary: string;
  description: string | null;
  status: string;
  status_category: string | null;
  priority: string | null;
  issue_type: string;
  assignee_id: string | null;
  assignee_name: string | null;
  reporter_id: string | null;
  reporter_name: string | null;
  labels: string | null;
  components: string | null;
  fix_versions: string | null;
  created_at: string;
  updated_at: string;
  raw_data: string;
  is_deleted: boolean;
  synced_at: string;
}

export interface DbProject {
  id: string;
  key: string;
  name: string;
  project_type: string;
  created_at: string;
  updated_at: string;
}

export interface DbChangeHistory {
  id: number;
  issue_id: string;
  issue_key: string;
  history_id: string;
  author_account_id: string | null;
  author_display_name: string | null;
  field: string;
  field_type: string;
  from_value: string | null;
  from_string: string | null;
  to_value: string | null;
  to_string: string | null;
  changed_at: string;
}

export interface DbSyncHistory {
  id: number;
  project_key: string;
  started_at: string;
  completed_at: string | null;
  status: 'running' | 'completed' | 'failed';
  issues_synced: number;
  error_message: string | null;
}

// Search types
export interface SearchParams {
  query?: string;
  project?: string;
  status?: string;
  assignee?: string;
  limit?: number;
  offset?: number;
}

export interface SearchResult {
  issues: DbIssue[];
  total: number;
}

// Sync types
export interface SyncProgress {
  projectKey: string;
  phase: 'metadata' | 'issues';
  current: number;
  total: number;
  message: string;
}

export interface SyncResult {
  projectKey: string;
  issuesSynced: number;
  issuesTotalInJira: number;
  startedAt: string;
  completedAt: string;
  success: boolean;
  errorMessage?: string;
}

// Message types for extension communication
export type MessageType =
  | 'GET_SETTINGS'
  | 'SAVE_SETTINGS'
  | 'INIT_PROJECTS'
  | 'GET_PROJECTS'
  | 'ENABLE_PROJECT'
  | 'DISABLE_PROJECT'
  | 'START_SYNC'
  | 'GET_SYNC_STATUS'
  | 'SEARCH_ISSUES'
  | 'GET_ISSUE'
  | 'GET_ISSUE_HISTORY'
  | 'CANCEL_SYNC';

export interface Message<T = unknown> {
  type: MessageType;
  payload?: T;
}

export interface MessageResponse<T = unknown> {
  success: boolean;
  data?: T;
  error?: string;
}
