use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use crate::domain::error::{DomainError, DomainResult};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub id: String,
    pub key: String,
    pub name: String,
    pub sync_enabled: bool,
    pub last_synced: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: PathBuf,
}

/// Configuration for embedding generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingsConfig {
    /// OpenAI API key (can also use OPENAI_API_KEY env var)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai_api_key: Option<String>,
    /// Embedding model to use (default: text-embedding-3-small)
    #[serde(default = "default_embedding_model")]
    pub model: String,
    /// Whether to generate embeddings during sync
    #[serde(default)]
    pub auto_generate: bool,
}

fn default_embedding_model() -> String {
    "text-embedding-3-small".to_string()
}

impl Default for EmbeddingsConfig {
    fn default() -> Self {
        Self {
            openai_api_key: None,
            model: default_embedding_model(),
            auto_generate: false,
        }
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
            fs::create_dir_all(parent)
                .map_err(|e| DomainError::Repository(format!("Failed to create directory: {}", e)))?;
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
            fs::set_permissions(&path, perms)
                .map_err(|e| DomainError::Repository(format!("Failed to set permissions: {}", e)))?;
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
                path: PathBuf::from("./data/jira.duckdb"),
            },
            embeddings: None,
        };

        settings.save(&path)?;
        Ok(settings)
    }

    pub fn default_path() -> DomainResult<PathBuf> {
        Ok(PathBuf::from("./settings.json"))
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
                path: PathBuf::from("./test.db"),
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
                path: PathBuf::from("./test.db"),
            },
            embeddings: None,
        };

        assert!(settings.validate().is_ok());

        settings.jira.api_key = "your-api-key-here".into();
        assert!(settings.validate().is_err());
    }
}
