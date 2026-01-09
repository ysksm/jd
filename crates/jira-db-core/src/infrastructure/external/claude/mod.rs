//! Claude API client for AI-powered test data generation
//!
//! Provides integration with Anthropic's Claude API for generating
//! realistic JIRA test data including epics, stories, tasks, and bugs.
//!
//! Supports two modes:
//! - API mode: Uses ANTHROPIC_API_KEY to call the Claude API directly
//! - CLI mode: Uses the `claude` command (Claude Code) with `-p` flag

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;

use crate::domain::error::{DomainError, DomainResult};

/// Default Claude API endpoint
pub const CLAUDE_API_ENDPOINT: &str = "https://api.anthropic.com/v1";

/// Claude model versions
pub mod models {
    pub const CLAUDE_SONNET_4: &str = "claude-sonnet-4-20250514";
    pub const CLAUDE_HAIKU: &str = "claude-3-5-haiku-20241022";
}

/// Configuration for Claude API client
#[derive(Debug, Clone)]
pub struct ClaudeConfig {
    /// Anthropic API key
    pub api_key: String,
    /// Model to use (default: claude-sonnet-4)
    pub model: String,
    /// API endpoint
    pub api_base: String,
    /// Request timeout
    pub timeout: Duration,
    /// Max tokens for response
    pub max_tokens: u32,
}

impl ClaudeConfig {
    /// Create a new configuration with API key
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: models::CLAUDE_SONNET_4.to_string(),
            api_base: CLAUDE_API_ENDPOINT.to_string(),
            timeout: Duration::from_secs(120),
            max_tokens: 4096,
        }
    }

    /// Use Claude Haiku for faster/cheaper responses
    pub fn with_haiku(mut self) -> Self {
        self.model = models::CLAUDE_HAIKU.to_string();
        self
    }

    /// Set custom model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

/// Claude API client
pub struct ClaudeClient {
    client: Client,
    config: ClaudeConfig,
}

impl ClaudeClient {
    /// Create a new Claude client
    pub fn new(config: ClaudeConfig) -> DomainResult<Self> {
        if config.api_key.is_empty() {
            return Err(DomainError::Configuration(
                "Anthropic API key is required".to_string(),
            ));
        }

        let client = Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| {
                DomainError::ExternalService(format!("Failed to create HTTP client: {}", e))
            })?;

        Ok(Self { client, config })
    }

    /// Send a message to Claude and get a response
    pub async fn message(&self, prompt: &str, system: Option<&str>) -> DomainResult<String> {
        let messages = vec![Message {
            role: "user".to_string(),
            content: prompt.to_string(),
        }];

        let request = MessageRequest {
            model: &self.config.model,
            max_tokens: self.config.max_tokens,
            system: system.map(|s| s.to_string()),
            messages: &messages,
        };

        let url = format!("{}/messages", self.config.api_base);

        let response = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                DomainError::ExternalService(format!("Failed to send Claude request: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(DomainError::ExternalService(format!(
                "Claude API error ({}): {}",
                status, error_text
            )));
        }

        let response: MessageResponse = response.json().await.map_err(|e| {
            DomainError::ExternalService(format!("Failed to parse Claude response: {}", e))
        })?;

        // Extract text from content blocks
        let text = response
            .content
            .into_iter()
            .filter_map(|c| {
                if c.content_type == "text" {
                    Some(c.text)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("");

        Ok(text)
    }

    /// Generate structured JSON response
    pub async fn generate_json<T: for<'de> Deserialize<'de>>(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> DomainResult<T> {
        let json_prompt = format!(
            "{}\n\nIMPORTANT: Respond with valid JSON only. No markdown code blocks, no explanation, just the JSON.",
            prompt
        );

        let response = self.message(&json_prompt, system).await?;

        // Try to parse directly
        let trimmed = response.trim();

        // Remove markdown code blocks if present
        let json_str = if trimmed.starts_with("```") {
            let lines: Vec<&str> = trimmed.lines().collect();
            lines[1..lines.len() - 1].join("\n")
        } else {
            trimmed.to_string()
        };

        serde_json::from_str(&json_str).map_err(|e| {
            DomainError::ExternalService(format!(
                "Failed to parse Claude JSON response: {}. Response was: {}",
                e,
                &json_str[..json_str.len().min(500)]
            ))
        })
    }

    /// Get the configuration
    pub fn config(&self) -> &ClaudeConfig {
        &self.config
    }
}

// Claude API request/response types
#[derive(Serialize)]
struct MessageRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    messages: &'a [Message],
}

#[derive(Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct MessageResponse {
    content: Vec<ContentBlock>,
    #[allow(dead_code)]
    model: String,
    #[allow(dead_code)]
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    content_type: String,
    #[serde(default)]
    text: String,
}

/// AI-generated JIRA issue data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedIssue {
    /// Issue type (Epic, Story, Task, Bug)
    pub issue_type: String,
    /// Issue summary
    pub summary: String,
    /// Issue description
    pub description: String,
    /// Priority (Highest, High, Medium, Low, Lowest)
    pub priority: String,
    /// Labels
    pub labels: Vec<String>,
    /// Story points (for stories/tasks)
    pub story_points: Option<i32>,
    /// Parent epic key (for stories)
    pub parent_key: Option<String>,
    /// Days from sprint start when created
    pub created_day_offset: i32,
    /// Days from sprint start when work started (None = not started)
    pub started_day_offset: Option<i32>,
    /// Days from sprint start when completed (None = not completed)
    pub completed_day_offset: Option<i32>,
    /// Assignee name (optional)
    pub assignee: Option<String>,
}

/// Sprint scenario configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SprintScenario {
    /// Sprint name
    pub sprint_name: String,
    /// Sprint duration in days
    pub duration_days: i32,
    /// Team members
    pub team_members: Vec<String>,
    /// Generated issues
    pub issues: Vec<GeneratedIssue>,
}

/// Trait for AI test data generation
#[async_trait]
pub trait AiTestDataGenerator: Send + Sync {
    /// Generate a sprint scenario with realistic issues
    async fn generate_sprint_scenario(
        &self,
        project_context: &str,
        team_size: usize,
        sprint_duration_days: i32,
    ) -> DomainResult<SprintScenario>;

    /// Generate a single epic with stories and tasks
    async fn generate_epic(
        &self,
        project_context: &str,
        epic_theme: &str,
    ) -> DomainResult<Vec<GeneratedIssue>>;

    /// Generate bugs with realistic lifecycle
    async fn generate_bugs(
        &self,
        project_context: &str,
        count: usize,
        sprint_duration_days: i32,
    ) -> DomainResult<Vec<GeneratedIssue>>;
}

#[async_trait]
impl AiTestDataGenerator for ClaudeClient {
    async fn generate_sprint_scenario(
        &self,
        project_context: &str,
        team_size: usize,
        sprint_duration_days: i32,
    ) -> DomainResult<SprintScenario> {
        let system = r#"You are a JIRA test data generator. Generate realistic sprint data for software development projects.
Your response must be valid JSON matching the expected schema exactly.
Use realistic software development terminology and create believable issue hierarchies."#;

        let prompt = format!(
            r#"Generate a realistic sprint scenario for the following project:

Project Context: {}
Team Size: {} developers
Sprint Duration: {} days

Generate a JSON object with this exact structure:
{{
  "sprint_name": "Sprint N",
  "duration_days": {},
  "team_members": ["Developer Name 1", "Developer Name 2", ...],
  "issues": [
    {{
      "issue_type": "Epic|Story|Task|Bug",
      "summary": "Brief issue title",
      "description": "Detailed description of the issue",
      "priority": "Highest|High|Medium|Low|Lowest",
      "labels": ["label1", "label2"],
      "story_points": 1|2|3|5|8|13|null,
      "parent_key": null,
      "created_day_offset": 0,
      "started_day_offset": null|number,
      "completed_day_offset": null|number,
      "assignee": "Developer Name"|null
    }}
  ]
}}

Requirements:
1. Create 1-2 Epics as containers for the work
2. Create 5-8 Stories under the epics (story_points: 1-8)
3. Create 3-5 Tasks for infrastructure/support work
4. Create 2-4 Bugs that get discovered during the sprint
5. Use realistic day offsets to create a burndown chart pattern:
   - Issues created_day_offset should be 0-3 (sprint planning)
   - started_day_offset should progressively increase (1-{})
   - completed_day_offset should be after started (2-{})
   - Some issues should NOT be completed (completed_day_offset: null)
   - Bugs appear mid-sprint (created_day_offset: 3-{})
6. Assign work to team members realistically
7. Use the project context to create relevant issue content

Generate Japanese issue titles and descriptions if the project context is in Japanese."#,
            project_context,
            team_size,
            sprint_duration_days,
            sprint_duration_days,
            sprint_duration_days,
            sprint_duration_days,
            sprint_duration_days / 2
        );

        self.generate_json(&prompt, Some(system)).await
    }

    async fn generate_epic(
        &self,
        project_context: &str,
        epic_theme: &str,
    ) -> DomainResult<Vec<GeneratedIssue>> {
        let system = r#"You are a JIRA test data generator. Generate realistic epic hierarchies for software development projects.
Your response must be valid JSON array matching the expected schema exactly."#;

        let prompt = format!(
            r#"Generate an epic with related stories and tasks for:

Project Context: {}
Epic Theme: {}

Generate a JSON array with this structure:
[
  {{
    "issue_type": "Epic",
    "summary": "Epic title",
    "description": "Epic description",
    "priority": "High",
    "labels": ["epic-label"],
    "story_points": null,
    "parent_key": null,
    "created_day_offset": 0,
    "started_day_offset": null,
    "completed_day_offset": null,
    "assignee": null
  }},
  {{
    "issue_type": "Story",
    "summary": "Story title",
    "description": "User story description",
    "priority": "Medium",
    "labels": ["feature"],
    "story_points": 5,
    "parent_key": "EPIC",
    "created_day_offset": 0,
    "started_day_offset": 1,
    "completed_day_offset": 5,
    "assignee": "Developer"
  }}
]

Requirements:
1. Create 1 Epic
2. Create 3-5 Stories under the epic
3. Create 1-2 Tasks for each story (technical tasks)
4. Set parent_key to "EPIC" for stories (will be replaced with actual key)
5. Use realistic story points (1, 2, 3, 5, 8)
6. Generate Japanese content if the project context is in Japanese"#,
            project_context, epic_theme
        );

        self.generate_json(&prompt, Some(system)).await
    }

    async fn generate_bugs(
        &self,
        project_context: &str,
        count: usize,
        sprint_duration_days: i32,
    ) -> DomainResult<Vec<GeneratedIssue>> {
        let system = r#"You are a JIRA test data generator. Generate realistic bugs for software development projects.
Your response must be valid JSON array matching the expected schema exactly."#;

        let prompt = format!(
            r#"Generate {} realistic bugs for:

Project Context: {}
Sprint Duration: {} days

Generate a JSON array of bugs:
[
  {{
    "issue_type": "Bug",
    "summary": "Bug title describing the issue",
    "description": "Steps to reproduce, expected vs actual behavior",
    "priority": "Highest|High|Medium|Low",
    "labels": ["bug", "category"],
    "story_points": null,
    "parent_key": null,
    "created_day_offset": number (3-{}),
    "started_day_offset": number|null,
    "completed_day_offset": number|null,
    "assignee": "Developer"|null
  }}
]

Requirements:
1. Bugs appear mid-sprint (day 3+)
2. Mix of priorities (some critical, some minor)
3. Some bugs fixed quickly, some take longer
4. Some bugs not yet fixed (completed_day_offset: null)
5. Realistic bug descriptions with reproduction steps
6. Generate Japanese content if the project context is in Japanese"#,
            count,
            project_context,
            sprint_duration_days,
            sprint_duration_days - 2
        );

        self.generate_json(&prompt, Some(system)).await
    }
}

/// Claude CLI client - uses the `claude` command with `-p` flag
///
/// This allows using Claude Code subscription instead of a separate API key.
/// Requires the `claude` CLI to be installed and authenticated.
pub struct ClaudeCliClient {
    /// Use --dangerously-skip-permissions to bypass permission prompts
    skip_permissions: bool,
}

impl ClaudeCliClient {
    /// Create a new Claude CLI client
    pub fn new() -> Self {
        Self {
            skip_permissions: true,
        }
    }

    /// Check if the claude CLI is available
    pub async fn is_available() -> bool {
        Command::new("claude")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Execute a prompt using the claude CLI
    async fn execute_prompt(&self, prompt: &str) -> DomainResult<String> {
        let mut cmd = Command::new("claude");
        cmd.arg("-p").arg(prompt);

        if self.skip_permissions {
            cmd.arg("--dangerously-skip-permissions");
        }

        // Set output format to plain text
        cmd.arg("--output-format").arg("text");

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| {
                DomainError::ExternalService(format!(
                    "Failed to execute claude command: {}. Is claude CLI installed?",
                    e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DomainError::ExternalService(format!(
                "Claude CLI failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    }

    /// Generate structured JSON response using the CLI
    async fn generate_json<T: for<'de> Deserialize<'de>>(&self, prompt: &str) -> DomainResult<T> {
        let json_prompt = format!(
            "{}\n\nIMPORTANT: Respond with valid JSON only. No markdown code blocks, no explanation, just the JSON.",
            prompt
        );

        let response = self.execute_prompt(&json_prompt).await?;

        // Try to parse directly
        let trimmed = response.trim();

        // Remove markdown code blocks if present
        let json_str = if trimmed.starts_with("```") {
            let lines: Vec<&str> = trimmed.lines().collect();
            if lines.len() > 2 {
                lines[1..lines.len() - 1].join("\n")
            } else {
                trimmed.to_string()
            }
        } else {
            trimmed.to_string()
        };

        serde_json::from_str(&json_str).map_err(|e| {
            DomainError::ExternalService(format!(
                "Failed to parse Claude CLI JSON response: {}. Response was: {}",
                e,
                &json_str[..json_str.len().min(500)]
            ))
        })
    }
}

impl Default for ClaudeCliClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AiTestDataGenerator for ClaudeCliClient {
    async fn generate_sprint_scenario(
        &self,
        project_context: &str,
        team_size: usize,
        sprint_duration_days: i32,
    ) -> DomainResult<SprintScenario> {
        let prompt = format!(
            r#"You are a JIRA test data generator. Generate realistic sprint data for software development projects.

Generate a realistic sprint scenario for the following project:

Project Context: {}
Team Size: {} developers
Sprint Duration: {} days

Generate a JSON object with this exact structure:
{{
  "sprint_name": "Sprint N",
  "duration_days": {},
  "team_members": ["Developer Name 1", "Developer Name 2", ...],
  "issues": [
    {{
      "issue_type": "Epic|Story|Task|Bug",
      "summary": "Brief issue title",
      "description": "Detailed description of the issue",
      "priority": "Highest|High|Medium|Low|Lowest",
      "labels": ["label1", "label2"],
      "story_points": 1|2|3|5|8|13|null,
      "parent_key": null,
      "created_day_offset": 0,
      "started_day_offset": null|number,
      "completed_day_offset": null|number,
      "assignee": "Developer Name"|null
    }}
  ]
}}

Requirements:
1. Create 1-2 Epics as containers for the work
2. Create 5-8 Stories under the epics (story_points: 1-8)
3. Create 3-5 Tasks for infrastructure/support work
4. Create 2-4 Bugs that get discovered during the sprint
5. Use realistic day offsets to create a burndown chart pattern:
   - Issues created_day_offset should be 0-3 (sprint planning)
   - started_day_offset should progressively increase (1-{})
   - completed_day_offset should be after started (2-{})
   - Some issues should NOT be completed (completed_day_offset: null)
   - Bugs appear mid-sprint (created_day_offset: 3-{})
6. Assign work to team members realistically
7. Use the project context to create relevant issue content

Generate Japanese issue titles and descriptions if the project context is in Japanese.

Respond with valid JSON only."#,
            project_context,
            team_size,
            sprint_duration_days,
            sprint_duration_days,
            sprint_duration_days,
            sprint_duration_days,
            sprint_duration_days / 2
        );

        self.generate_json(&prompt).await
    }

    async fn generate_epic(
        &self,
        project_context: &str,
        epic_theme: &str,
    ) -> DomainResult<Vec<GeneratedIssue>> {
        let prompt = format!(
            r#"You are a JIRA test data generator. Generate realistic epic hierarchies for software development projects.

Generate an epic with related stories and tasks for:

Project Context: {}
Epic Theme: {}

Generate a JSON array with this structure:
[
  {{
    "issue_type": "Epic",
    "summary": "Epic title",
    "description": "Epic description",
    "priority": "High",
    "labels": ["epic-label"],
    "story_points": null,
    "parent_key": null,
    "created_day_offset": 0,
    "started_day_offset": null,
    "completed_day_offset": null,
    "assignee": null
  }},
  {{
    "issue_type": "Story",
    "summary": "Story title",
    "description": "User story description",
    "priority": "Medium",
    "labels": ["feature"],
    "story_points": 5,
    "parent_key": "EPIC",
    "created_day_offset": 0,
    "started_day_offset": 1,
    "completed_day_offset": 5,
    "assignee": "Developer"
  }}
]

Requirements:
1. Create 1 Epic
2. Create 3-5 Stories under the epic
3. Create 1-2 Tasks for each story (technical tasks)
4. Set parent_key to "EPIC" for stories (will be replaced with actual key)
5. Use realistic story points (1, 2, 3, 5, 8)
6. Generate Japanese content if the project context is in Japanese

Respond with valid JSON only."#,
            project_context, epic_theme
        );

        self.generate_json(&prompt).await
    }

    async fn generate_bugs(
        &self,
        project_context: &str,
        count: usize,
        sprint_duration_days: i32,
    ) -> DomainResult<Vec<GeneratedIssue>> {
        let prompt = format!(
            r#"You are a JIRA test data generator. Generate realistic bugs for software development projects.

Generate {} realistic bugs for:

Project Context: {}
Sprint Duration: {} days

Generate a JSON array of bugs:
[
  {{
    "issue_type": "Bug",
    "summary": "Bug title describing the issue",
    "description": "Steps to reproduce, expected vs actual behavior",
    "priority": "Highest|High|Medium|Low",
    "labels": ["bug", "category"],
    "story_points": null,
    "parent_key": null,
    "created_day_offset": number (3-{}),
    "started_day_offset": number|null,
    "completed_day_offset": number|null,
    "assignee": "Developer"|null
  }}
]

Requirements:
1. Bugs appear mid-sprint (day 3+)
2. Mix of priorities (some critical, some minor)
3. Some bugs fixed quickly, some take longer
4. Some bugs not yet fixed (completed_day_offset: null)
5. Realistic bug descriptions with reproduction steps
6. Generate Japanese content if the project context is in Japanese

Respond with valid JSON only."#,
            count,
            project_context,
            sprint_duration_days,
            sprint_duration_days - 2
        );

        self.generate_json(&prompt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = ClaudeConfig::new("test-key");
        assert_eq!(config.model, models::CLAUDE_SONNET_4);
        assert_eq!(config.api_key, "test-key");
    }

    #[test]
    fn test_config_with_haiku() {
        let config = ClaudeConfig::new("test-key").with_haiku();
        assert_eq!(config.model, models::CLAUDE_HAIKU);
    }

    #[test]
    fn test_cli_client_default() {
        let client = ClaudeCliClient::new();
        assert!(client.skip_permissions);
    }
}
