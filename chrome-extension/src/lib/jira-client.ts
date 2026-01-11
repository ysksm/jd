import type {
  JiraSettings,
  JiraProject,
  JiraIssue,
  JiraSearchResponse,
  JiraStatus,
  JiraPriority,
  JiraIssueType,
} from './types';

export class JiraClient {
  private endpoint: string;
  private authHeader: string | null;
  private useBrowserAuth: boolean;

  constructor(settings: JiraSettings) {
    this.endpoint = settings.endpoint.replace(/\/$/, '');
    this.useBrowserAuth = settings.authMethod === 'browser';

    if (this.useBrowserAuth) {
      // Use browser session cookies - no auth header needed
      this.authHeader = null;
    } else {
      // Base64 encode for Basic Auth
      const credentials = btoa(`${settings.username}:${settings.apiKey}`);
      this.authHeader = `Basic ${credentials}`;
    }
  }

  private async request<T>(path: string, options: RequestInit = {}): Promise<T> {
    const url = `${this.endpoint}${path}`;

    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      'Accept': 'application/json',
    };

    // Add auth header only for API token auth
    if (this.authHeader) {
      headers['Authorization'] = this.authHeader;
    }

    const response = await fetch(url, {
      ...options,
      // Include cookies for browser auth
      credentials: this.useBrowserAuth ? 'include' : 'omit',
      headers: {
        ...headers,
        ...options.headers,
      },
    });

    if (!response.ok) {
      const errorText = await response.text();
      if (response.status === 401) {
        if (this.useBrowserAuth) {
          throw new Error('Not logged in to JIRA. Please log in to JIRA in your browser first.');
        } else {
          throw new Error('Invalid API credentials. Please check your username and API token.');
        }
      }
      throw new Error(`JIRA API error: ${response.status} ${response.statusText} - ${errorText}`);
    }

    return response.json() as Promise<T>;
  }

  // Get all projects
  async getProjects(): Promise<JiraProject[]> {
    return this.request<JiraProject[]>('/rest/api/3/project');
  }

  // Get project by key
  async getProject(key: string): Promise<JiraProject> {
    return this.request<JiraProject>(`/rest/api/3/project/${key}`);
  }

  // Search issues with JQL
  async searchIssues(
    jql: string,
    startAt: number = 0,
    maxResults: number = 100
  ): Promise<JiraSearchResponse> {
    const params = new URLSearchParams({
      jql,
      startAt: startAt.toString(),
      maxResults: maxResults.toString(),
      fields: '*navigable',
      expand: 'changelog',
    });

    return this.request<JiraSearchResponse>(
      `/rest/api/3/search/jql?${params.toString()}`
    );
  }

  // Get single issue with changelog
  async getIssue(issueKey: string): Promise<JiraIssue> {
    const params = new URLSearchParams({
      fields: '*navigable',
      expand: 'changelog',
    });

    return this.request<JiraIssue>(
      `/rest/api/3/issue/${issueKey}?${params.toString()}`
    );
  }

  // Get project statuses
  async getProjectStatuses(projectKey: string): Promise<JiraStatus[]> {
    interface StatusResponse {
      id: string;
      name: string;
      statuses: JiraStatus[];
    }
    const response = await this.request<StatusResponse[]>(
      `/rest/api/3/project/${projectKey}/statuses`
    );
    // Flatten statuses from all issue types
    const statuses = new Map<string, JiraStatus>();
    for (const issueType of response) {
      for (const status of issueType.statuses) {
        statuses.set(status.id, status);
      }
    }
    return Array.from(statuses.values());
  }

  // Get priorities
  async getPriorities(): Promise<JiraPriority[]> {
    return this.request<JiraPriority[]>('/rest/api/3/priority');
  }

  // Get issue types for a project
  async getIssueTypes(projectId: string): Promise<JiraIssueType[]> {
    return this.request<JiraIssueType[]>(
      `/rest/api/3/issuetype/project?projectId=${projectId}`
    );
  }

  // Test connection
  async testConnection(): Promise<boolean> {
    try {
      await this.request<unknown>('/rest/api/3/myself');
      return true;
    } catch {
      return false;
    }
  }

  // Get all issues for a project with pagination
  async *getAllIssues(
    projectKey: string,
    updatedSince?: string,
    onProgress?: (current: number, total: number) => void
  ): AsyncGenerator<JiraIssue[], void, unknown> {
    let jql = `project = ${projectKey}`;

    if (updatedSince) {
      // Convert to JIRA date format (yyyy-MM-dd HH:mm)
      const date = new Date(updatedSince);
      const jiraDate = formatJiraDate(date);
      jql += ` AND updated >= "${jiraDate}"`;
    }

    jql += ' ORDER BY updated ASC';

    let startAt = 0;
    const maxResults = 100;
    let total = 0;

    do {
      const response = await this.searchIssues(jql, startAt, maxResults);
      total = response.total;

      if (response.issues.length === 0) break;

      yield response.issues;

      startAt += response.issues.length;

      if (onProgress) {
        onProgress(startAt, total);
      }
    } while (startAt < total);
  }

  // Get issues starting from a checkpoint
  async *getIssuesFromCheckpoint(
    projectKey: string,
    startPosition: number,
    updatedSince?: string,
    onProgress?: (current: number, total: number) => void
  ): AsyncGenerator<JiraIssue[], void, unknown> {
    let jql = `project = ${projectKey}`;

    if (updatedSince) {
      const date = new Date(updatedSince);
      const jiraDate = formatJiraDate(date);
      jql += ` AND updated >= "${jiraDate}"`;
    }

    jql += ' ORDER BY updated ASC';

    let startAt = startPosition;
    const maxResults = 100;
    let total = 0;

    do {
      const response = await this.searchIssues(jql, startAt, maxResults);
      total = response.total;

      if (response.issues.length === 0) break;

      yield response.issues;

      startAt += response.issues.length;

      if (onProgress) {
        onProgress(startAt, total);
      }
    } while (startAt < total);
  }
}

// Format date for JQL (in local timezone)
function formatJiraDate(date: Date): string {
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, '0');
  const day = String(date.getDate()).padStart(2, '0');
  const hours = String(date.getHours()).padStart(2, '0');
  const minutes = String(date.getMinutes()).padStart(2, '0');

  return `${year}-${month}-${day} ${hours}:${minutes}`;
}
