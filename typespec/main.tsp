/**
 * JiraDb API Definition
 *
 * Design principles:
 * - All operations use POST method (RPC-style)
 * - Parameters are passed in request body
 * - Consistent with Tauri command invocation pattern
 */

// ============================================================
// Models - Core Entities
// ============================================================

model Project {
  id: string;
  key: string;
  name: string;
  description?: string;
  enabled: boolean;
  lastSyncedAt?: utcDateTime;
}

model Issue {
  id: string;
  key: string;
  projectKey: string;
  summary: string;
  description?: string;
  status: string;
  priority: string;
  issueType: string;
  assignee?: string;
  reporter?: string;
  parentKey?: string;
  labels: string[];
  components: string[];
  fixVersions: string[];
  team?: string;
  dueDate?: utcDateTime;
  createdAt: utcDateTime;
  updatedAt: utcDateTime;
}

model ChangeHistoryItem {
  id: string;
  issueKey: string;
  author: string;
  field: string;
  fieldType: string;
  fromValue?: string;
  fromString?: string;
  toValue?: string;
  toString?: string;
  changedAt: utcDateTime;
}

model Status {
  name: string;
  description?: string;
  category: string;
}

model Priority {
  name: string;
  description?: string;
  iconUrl?: string;
}

model IssueType {
  name: string;
  description?: string;
  iconUrl?: string;
  subtask: boolean;
}

model Label {
  name: string;
}

model Component {
  name: string;
  description?: string;
  lead?: string;
}

model FixVersion {
  name: string;
  description?: string;
  released: boolean;
  releaseDate?: utcDateTime;
}

model ProjectMetadata {
  projectKey: string;
  statuses: Status[];
  priorities: Priority[];
  issueTypes: IssueType[];
  labels: Label[];
  components: Component[];
  fixVersions: FixVersion[];
}

model SyncResult {
  projectKey: string;
  issueCount: int32;
  metadataUpdated: boolean;
  duration: float64;
  success: boolean;
  error?: string;
}

model SyncProgress {
  projectKey: string;
  phase: string;
  current: int32;
  total: int32;
  message: string;
}

model EmbeddingStats {
  totalIssues: int32;
  processedIssues: int32;
  duration: float64;
}

model SemanticSearchResult {
  issue: Issue;
  score: float64;
}

model ReportResult {
  outputPath: string;
  issueCount: int32;
}

// ============================================================
// SQL Query Models
// ============================================================

model SavedQuery {
  id: string;
  name: string;
  query: string;
  description?: string;
  createdAt: utcDateTime;
  updatedAt: utcDateTime;
}

model SqlColumn {
  name: string;
  dataType: string;
  isNullable: boolean;
}

model SqlTable {
  name: string;
  columns?: SqlColumn[];
}

// ============================================================
// Config Models
// ============================================================

model JiraConfig {
  endpoint: string;
  username: string;
  apiKey: string;
}

/** JIRA endpoint configuration (for multiple endpoints) */
model JiraEndpoint {
  name: string;
  displayName?: string;
  endpoint: string;
  username: string;
  apiKey: string;
}

model DatabaseConfig {
  path: string;
}

model EmbeddingsConfig {
  provider: string;
  modelName?: string;
  endpoint?: string;
  autoGenerate: boolean;
}

model LogConfig {
  /** Enable file logging */
  fileEnabled: boolean;
  /** Log file directory (defaults to database_dir/logs) */
  fileDir?: string;
  /** Log level: "error", "warn", "info", "debug", "trace" */
  level: string;
  /** Maximum number of log files to keep (0 = unlimited) */
  maxFiles: int32;
}

model SyncConfig {
  incrementalSyncEnabled: boolean;
  incrementalSyncMarginMinutes: int32;
}

model Settings {
  jira: JiraConfig;
  database: DatabaseConfig;
  projects: ProjectConfig[];
  embeddings?: EmbeddingsConfig;
  log?: LogConfig;
  sync?: SyncConfig;
  /** Multiple JIRA endpoints (new) */
  jiraEndpoints?: JiraEndpoint[];
  /** Active endpoint name (new) */
  activeEndpoint?: string;
}

model ProjectConfig {
  key: string;
  enabled: boolean;
}

// ============================================================
// Request/Response Models
// ============================================================

// -- Config --
model ConfigGetRequest {}

model ConfigGetResponse {
  settings: Settings;
}

model ConfigUpdateRequest {
  jira?: JiraConfig;
  database?: DatabaseConfig;
  embeddings?: EmbeddingsConfig;
  log?: LogConfig;
  sync?: SyncConfig;
  /** Add a new endpoint */
  addEndpoint?: JiraEndpoint;
  /** Remove an endpoint by name */
  removeEndpoint?: string;
  /** Set active endpoint */
  setActiveEndpoint?: string;
}

model ConfigUpdateResponse {
  success: boolean;
  settings: Settings;
}

model ConfigInitRequest {
  endpoint: string;
  username: string;
  apiKey: string;
  databasePath?: string;
}

model ConfigInitResponse {
  success: boolean;
  settings: Settings;
}

// -- Projects --
model ProjectListRequest {}

model ProjectListResponse {
  projects: Project[];
}

model ProjectInitRequest {
  /** Specific endpoint name to fetch from. If not provided, uses active endpoint. */
  endpointName?: string;
  /** If true, fetch from all configured endpoints */
  allEndpoints?: boolean;
}

model ProjectInitResponse {
  projects: Project[];
  newCount: int32;
  /** Results per endpoint when fetching from multiple endpoints */
  endpointResults?: EndpointFetchResult[];
}

model EndpointFetchResult {
  endpointName: string;
  projectCount: int32;
  success: boolean;
  error?: string;
}

model ProjectEnableRequest {
  key: string;
}

model ProjectEnableResponse {
  project: Project;
}

model ProjectDisableRequest {
  key: string;
}

model ProjectDisableResponse {
  project: Project;
}

// -- Sync --
model SyncExecuteRequest {
  projectKey?: string;
  /** Force full sync (ignore last_synced, re-fetch all issues) */
  force?: boolean;
}

model SyncExecuteResponse {
  results: SyncResult[];
}

model SyncStatusRequest {}

model SyncStatusResponse {
  inProgress: boolean;
  progress?: SyncProgress;
}

// -- Issues --
model IssueSearchRequest {
  query?: string;
  project?: string;
  status?: string;
  assignee?: string;
  priority?: string;
  issueType?: string;
  team?: string;
  limit?: int32;
  offset?: int32;
}

model IssueSearchResponse {
  issues: Issue[];
  total: int32;
}

model IssueGetRequest {
  key: string;
}

model IssueGetResponse {
  issue: Issue;
}

model IssueHistoryRequest {
  key: string;
  field?: string;
  limit?: int32;
}

model IssueHistoryResponse {
  history: ChangeHistoryItem[];
}

// -- Metadata --
model MetadataGetRequest {
  projectKey: string;
  type?: string;  // status, priority, issue-type, label, component, version
}

model MetadataGetResponse {
  metadata: ProjectMetadata;
}

// -- Embeddings --
model EmbeddingsGenerateRequest {
  projectKey?: string;
  force?: boolean;
  batchSize?: int32;
  provider?: string;
}

model EmbeddingsGenerateResponse {
  stats: EmbeddingStats;
}

model SemanticSearchRequest {
  query: string;
  projectKey?: string;
  limit?: int32;
}

model SemanticSearchResponse {
  results: SemanticSearchResult[];
}

// -- Reports --
model ReportGenerateRequest {
  interactive?: boolean;
  outputPath?: string;
  projectKey?: string;
}

model ReportGenerateResponse {
  result: ReportResult;
}

// -- SQL Query --
model SqlExecuteRequest {
  projectKey?: string;
  query: string;
  limit?: int32;
}

model SqlExecuteResponse {
  columns: string[];
  rows: unknown[];
  rowCount: int32;
  executionTimeMs: float64;
}

model SqlGetSchemaRequest {
  projectKey?: string;
  table?: string;
}

model SqlGetSchemaResponse {
  tables: SqlTable[];
}

model SqlQueryListRequest {}

model SqlQueryListResponse {
  queries: SavedQuery[];
}

model SqlQuerySaveRequest {
  id?: string;
  name: string;
  query: string;
  description?: string;
}

model SqlQuerySaveResponse {
  query: SavedQuery;
}

model SqlQueryDeleteRequest {
  id: string;
}

model SqlQueryDeleteResponse {
  success: boolean;
}

// ============================================================
// Operations - RPC Style (POST only, body params)
// ============================================================

@doc("Configuration management operations")
namespace Config {
  @doc("Get current configuration")
  op get(request: ConfigGetRequest): ConfigGetResponse;

  @doc("Update configuration")
  op update(request: ConfigUpdateRequest): ConfigUpdateResponse;

  @doc("Initialize configuration")
  op initialize(request: ConfigInitRequest): ConfigInitResponse;
}

@doc("Project management operations")
namespace Projects {
  @doc("List all projects")
  op list(request: ProjectListRequest): ProjectListResponse;

  @doc("Initialize projects from JIRA")
  op initialize(request: ProjectInitRequest): ProjectInitResponse;

  @doc("Enable project sync")
  op enable(request: ProjectEnableRequest): ProjectEnableResponse;

  @doc("Disable project sync")
  op disable(request: ProjectDisableRequest): ProjectDisableResponse;
}

@doc("Synchronization operations")
namespace Sync {
  @doc("Execute sync for enabled projects")
  op execute(request: SyncExecuteRequest): SyncExecuteResponse;

  @doc("Get sync status")
  op status(request: SyncStatusRequest): SyncStatusResponse;
}

@doc("Issue operations")
namespace Issues {
  @doc("Search issues with filters")
  op search(request: IssueSearchRequest): IssueSearchResponse;

  @doc("Get issue by key")
  op get(request: IssueGetRequest): IssueGetResponse;

  @doc("Get issue change history")
  op history(request: IssueHistoryRequest): IssueHistoryResponse;
}

@doc("Metadata operations")
namespace Metadata {
  @doc("Get project metadata")
  op get(request: MetadataGetRequest): MetadataGetResponse;
}

@doc("Embeddings operations")
namespace Embeddings {
  @doc("Generate embeddings for semantic search")
  op generate(request: EmbeddingsGenerateRequest): EmbeddingsGenerateResponse;

  @doc("Semantic search using embeddings")
  op search(request: SemanticSearchRequest): SemanticSearchResponse;
}

@doc("Report operations")
namespace Reports {
  @doc("Generate HTML report")
  op generate(request: ReportGenerateRequest): ReportGenerateResponse;
}

@doc("SQL query operations")
namespace Sql {
  @doc("Execute SQL query (read-only)")
  op execute(request: SqlExecuteRequest): SqlExecuteResponse;

  @doc("Get database schema")
  op getSchema(request: SqlGetSchemaRequest): SqlGetSchemaResponse;

  @doc("List saved queries")
  op listQueries(request: SqlQueryListRequest): SqlQueryListResponse;

  @doc("Save a query")
  op saveQuery(request: SqlQuerySaveRequest): SqlQuerySaveResponse;

  @doc("Delete a saved query")
  op deleteQuery(request: SqlQueryDeleteRequest): SqlQueryDeleteResponse;
}

// ============================================================
// Debug Types
// ============================================================

model CreatedIssue {
  key: string;
  id: string;
  selfUrl: string;
}

model Transition {
  id: string;
  name: string;
  toStatus: string;
}

model BulkTransitionResult {
  issueKey: string;
  success: boolean;
  error?: string;
}

model IssueTypeInfo {
  name: string;
  description?: string;
  iconUrl?: string;
  subtask: boolean;
}

// Debug Request/Response Types
model DebugStatusRequest {}

model DebugStatusResponse {
  enabled: boolean;
  message: string;
}

model DebugGetIssueTypesRequest {
  project: string;
}

model DebugGetIssueTypesResponse {
  issueTypes: IssueTypeInfo[];
}

model DebugCreateIssuesRequest {
  project: string;
  count?: int32;
  issueType?: string;
  summary?: string;
  description?: string;
}

model DebugCreateIssuesResponse {
  success: boolean;
  created: CreatedIssue[];
  error?: string;
}

model DebugListTransitionsRequest {
  issueKey: string;
}

model DebugListTransitionsResponse {
  transitions: Transition[];
}

model DebugTransitionIssueRequest {
  issueKey: string;
  transitionId: string;
}

model DebugTransitionIssueResponse {
  success: boolean;
  error?: string;
}

model DebugBulkTransitionRequest {
  issues: string[];
  transitionId: string;
}

model DebugBulkTransitionResponse {
  results: BulkTransitionResult[];
  successCount: int32;
  failureCount: int32;
}

// ============================================================
// AI Test Data Generation Types
// ============================================================

/** Generated issue from AI */
model GeneratedIssue {
  issueType: string;
  summary: string;
  description: string;
  priority: string;
  labels: string[];
  storyPoints?: int32;
  parentKey?: string;
  createdDayOffset: int32;
  startedDayOffset?: int32;
  completedDayOffset?: int32;
  assignee?: string;
}

/** Sprint scenario generated by AI */
model SprintScenario {
  sprintName: string;
  durationDays: int32;
  teamMembers: string[];
  issues: GeneratedIssue[];
}

/** Information about a created issue */
model AiCreatedIssueInfo {
  key: string;
  id: string;
  issueType: string;
  summary: string;
  status: string;
  selfUrl?: string;
}

/** Information about a failed issue creation */
model AiFailedIssueInfo {
  issueType: string;
  summary: string;
  error: string;
}

/** Statistics about the AI generation */
model AiGenerationStats {
  totalGenerated: int32;
  successfullyCreated: int32;
  failedToCreate: int32;
  epicsCreated: int32;
  storiesCreated: int32;
  tasksCreated: int32;
  bugsCreated: int32;
  transitionsApplied: int32;
  linksCreated: int32;
  dueDatesSet: int32;
}

/** Request for AI test data generation */
model DebugAiGenerateRequest {
  /** Target project key */
  project: string;
  /** Generation mode: sprint, epic, or bugs */
  mode: string;
  /** Project context/description for AI */
  projectContext?: string;
  /** Team size (number of developers) */
  teamSize?: int32;
  /** Sprint duration in days */
  sprintDurationDays?: int32;
  /** Whether to apply status transitions */
  applyTransitions?: boolean;
  /** Epic theme (for epic mode) */
  epicTheme?: string;
  /** Number of bugs to generate (for bugs mode) */
  bugCount?: int32;
  /** Use faster model (Haiku) */
  useFastModel?: boolean;
  /** Use Claude CLI instead of API */
  useClaudeCli?: boolean;
  /** Language for generated content (e.g., "ja" for Japanese, "en" for English) */
  language?: string;
}

/** Response from AI test data generation */
model DebugAiGenerateResponse {
  success: boolean;
  scenario?: SprintScenario;
  createdIssues: AiCreatedIssueInfo[];
  failedIssues: AiFailedIssueInfo[];
  stats: AiGenerationStats;
  error?: string;
}

/** Request to check AI configuration */
model DebugAiStatusRequest {}

/** Response for AI configuration status */
model DebugAiStatusResponse {
  configured: boolean;
  message: string;
  cliAvailable?: boolean;
  apiKeyConfigured?: boolean;
}

// ============================================================
// Debug Operations
// ============================================================

@doc("Debug operations (requires debug_mode in settings)")
namespace Debug {
  @doc("Get debug status")
  op status(request: DebugStatusRequest): DebugStatusResponse;

  @doc("Get issue types for a project")
  op getIssueTypes(request: DebugGetIssueTypesRequest): DebugGetIssueTypesResponse;

  @doc("Create test issues")
  op createIssues(request: DebugCreateIssuesRequest): DebugCreateIssuesResponse;

  @doc("List available transitions for an issue")
  op listTransitions(request: DebugListTransitionsRequest): DebugListTransitionsResponse;

  @doc("Transition an issue to a new status")
  op transitionIssue(request: DebugTransitionIssueRequest): DebugTransitionIssueResponse;

  @doc("Bulk transition multiple issues")
  op bulkTransition(request: DebugBulkTransitionRequest): DebugBulkTransitionResponse;

  @doc("Check AI test data generation status")
  op aiStatus(request: DebugAiStatusRequest): DebugAiStatusResponse;

  @doc("Generate test data using AI (Claude)")
  op aiGenerate(request: DebugAiGenerateRequest): DebugAiGenerateResponse;
}
