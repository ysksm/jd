use crate::domain::error::{DomainError, DomainResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub jira: JiraConfig,
    pub projects: Vec<ProjectConfig>,
    pub database: DatabaseConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeddings: Option<EmbeddingsConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraConfig {
    pub endpoint: String,
    pub username: String,
    pub api_key: String,
}

/// Checkpoint for resumable sync
/// Stored in settings.json to allow sync to resume after interruption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncCheckpoint {
    /// Timestamp of the last successfully synced issue's updated field
    pub last_issue_updated_at: DateTime<Utc>,
    /// Key of the last successfully synced issue (for tie-breaking)
    pub last_issue_key: String,
    /// Number of issues processed so far in this sync session
    pub items_processed: usize,
    /// Total number of issues expected (from JIRA)
    pub total_items: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub id: String,
    pub key: String,
    pub name: String,
    pub sync_enabled: bool,
    pub last_synced: Option<DateTime<Utc>>,
    /// Checkpoint for resuming interrupted sync
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sync_checkpoint: Option<SyncCheckpoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Legacy single database path (deprecated, kept for backward compatibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<PathBuf>,
    /// Directory containing per-project database files
    /// Each project will have its own database at {database_dir}/{project_key}.duckdb
    #[serde(default = "default_database_dir")]
    pub database_dir: PathBuf,
}

fn default_database_dir() -> PathBuf {
    PathBuf::from("./data")
}

/// Configuration for embedding generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsConfig {
    /// Embedding provider: "openai", "ollama", or "cohere"
    #[serde(default = "default_provider")]
    pub provider: String,
    /// API key (for OpenAI: OPENAI_API_KEY, for Cohere: COHERE_API_KEY)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// OpenAI API key (deprecated, use api_key instead)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai_api_key: Option<String>,
    /// Embedding model to use
    #[serde(default = "default_embedding_model")]
    pub model: String,
    /// Endpoint URL (for Ollama, default: http://localhost:11434)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    /// Whether to generate embeddings during sync
    #[serde(default)]
    pub auto_generate: bool,
}

fn default_provider() -> String {
    "openai".to_string()
}

fn default_embedding_model() -> String {
    "text-embedding-3-small".to_string()
}

impl Default for EmbeddingsConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            api_key: None,
            openai_api_key: None,
            model: default_embedding_model(),
            endpoint: None,
            auto_generate: false,
        }
    }
}

impl EmbeddingsConfig {
    /// Get the effective API key (prefers api_key over openai_api_key)
    pub fn get_api_key(&self) -> Option<&String> {
        self.api_key.as_ref().or(self.openai_api_key.as_ref())
    }
}

impl Settings {
    pub fn load<P: AsRef<Path>>(path: P) -> DomainResult<Self> {
        let content = fs::read_to_string(&path)
            .map_err(|e| DomainError::Repository(format!("Failed to read settings file: {}", e)))?;
        let settings: Settings = serde_json::from_str(&content)
            .map_err(|e| DomainError::Repository(format!("Failed to parse settings: {}", e)))?;
        Ok(settings)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> DomainResult<()> {
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent).map_err(|e| {
                DomainError::Repository(format!("Failed to create directory: {}", e))
            })?;
        }

        let content = serde_json::to_string_pretty(self)
            .map_err(|e| DomainError::Repository(format!("Failed to serialize settings: {}", e)))?;
        fs::write(&path, content)
            .map_err(|e| DomainError::Repository(format!("Failed to write settings: {}", e)))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path)
                .map_err(|e| DomainError::Repository(format!("Failed to get metadata: {}", e)))?
                .permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&path, perms).map_err(|e| {
                DomainError::Repository(format!("Failed to set permissions: {}", e))
            })?;
        }

        Ok(())
    }

    pub fn create_default<P: AsRef<Path>>(path: P) -> DomainResult<Self> {
        let settings = Settings {
            jira: JiraConfig {
                endpoint: String::from("https://your-domain.atlassian.net"),
                username: String::from("user@example.com"),
                api_key: String::from("your-api-key-here"),
            },
            projects: Vec::new(),
            database: DatabaseConfig {
                path: None,
                database_dir: PathBuf::from("./data"),
            },
            embeddings: None,
        };

        settings.save(&path)?;
        Ok(settings)
    }

    pub fn default_path() -> DomainResult<PathBuf> {
        Ok(PathBuf::from("./data/settings.json"))
    }

    pub fn exists<P: AsRef<Path>>(path: P) -> bool {
        path.as_ref().exists()
    }

    pub fn validate(&self) -> DomainResult<()> {
        if self.jira.endpoint.is_empty() {
            return Err(DomainError::Validation(
                "JIRA endpoint cannot be empty".into(),
            ));
        }

        if self.jira.username.is_empty() {
            return Err(DomainError::Validation(
                "JIRA username cannot be empty".into(),
            ));
        }

        if self.jira.api_key.is_empty() || self.jira.api_key == "your-api-key-here" {
            return Err(DomainError::Validation(
                "JIRA API key must be configured".into(),
            ));
        }

        Ok(())
    }

    pub fn find_project(&self, key: &str) -> Option<&ProjectConfig> {
        self.projects.iter().find(|p| p.key == key)
    }

    pub fn find_project_mut(&mut self, key: &str) -> Option<&mut ProjectConfig> {
        self.projects.iter_mut().find(|p| p.key == key)
    }

    pub fn upsert_project(&mut self, project: ProjectConfig) {
        if let Some(existing) = self.find_project_mut(&project.key) {
            *existing = project;
        } else {
            self.projects.push(project);
        }
    }

    pub fn sync_enabled_projects(&self) -> Vec<&ProjectConfig> {
        self.projects.iter().filter(|p| p.sync_enabled).collect()
    }

    /// Get the database path for a specific project
    /// Returns {database_dir}/{project_key}.duckdb
    pub fn get_database_path_for_project(&self, project_key: &str) -> PathBuf {
        self.database
            .database_dir
            .join(format!("{}.duckdb", project_key))
    }

    /// Get the database directory
    pub fn get_database_dir(&self) -> &Path {
        &self.database.database_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_serialization() {
        let settings = Settings {
            jira: JiraConfig {
                endpoint: "https://test.atlassian.net".into(),
                username: "test@example.com".into(),
                api_key: "test-key".into(),
            },
            projects: vec![],
            database: DatabaseConfig {
                path: None,
                database_dir: PathBuf::from("./data"),
            },
            embeddings: None,
        };

        let json = serde_json::to_string(&settings).unwrap();
        let deserialized: Settings = serde_json::from_str(&json).unwrap();

        assert_eq!(settings.jira.endpoint, deserialized.jira.endpoint);
    }

    #[test]
    fn test_validate() {
        let mut settings = Settings {
            jira: JiraConfig {
                endpoint: "https://test.atlassian.net".into(),
                username: "test@example.com".into(),
                api_key: "test-key".into(),
            },
            projects: vec![],
            database: DatabaseConfig {
                path: None,
                database_dir: PathBuf::from("./data"),
            },
            embeddings: None,
        };

        assert!(settings.validate().is_ok());

        settings.jira.api_key = "your-api-key-here".into();
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_get_database_path_for_project() {
        let settings = Settings {
            jira: JiraConfig {
                endpoint: "https://test.atlassian.net".into(),
                username: "test@example.com".into(),
                api_key: "test-key".into(),
            },
            projects: vec![],
            database: DatabaseConfig {
                path: None,
                database_dir: PathBuf::from("./data"),
            },
            embeddings: None,
        };

        let path = settings.get_database_path_for_project("MYPROJ");
        assert_eq!(path, PathBuf::from("./data/MYPROJ.duckdb"));
    }
}
