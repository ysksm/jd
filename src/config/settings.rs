use crate::error::{JiraDbError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub jira: JiraConfig,
    pub projects: Vec<ProjectConfig>,
    pub database: DatabaseConfig,
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

impl Settings {
    /// Load settings from file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)?;
        let settings: Settings = serde_json::from_str(&content)?;
        Ok(settings)
    }

    /// Save settings to file
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(self)?;
        fs::write(&path, content)?;

        // Set file permissions to 600 (read/write for owner only) on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path)?.permissions();
            perms.set_mode(0o600);
            fs::set_permissions(&path, perms)?;
        }

        Ok(())
    }

    /// Create default settings file
    pub fn create_default<P: AsRef<Path>>(path: P) -> Result<Self> {
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
        };

        settings.save(&path)?;
        Ok(settings)
    }

    /// Get the default settings file path (current directory)
    pub fn default_path() -> Result<PathBuf> {
        Ok(PathBuf::from("./settings.json"))
    }

    /// Check if settings file exists
    pub fn exists<P: AsRef<Path>>(path: P) -> bool {
        path.as_ref().exists()
    }

    /// Validate settings
    pub fn validate(&self) -> Result<()> {
        if self.jira.endpoint.is_empty() {
            return Err(JiraDbError::InvalidConfig(
                "JIRA endpoint cannot be empty".into(),
            ));
        }

        if self.jira.username.is_empty() {
            return Err(JiraDbError::InvalidConfig(
                "JIRA username cannot be empty".into(),
            ));
        }

        if self.jira.api_key.is_empty() || self.jira.api_key == "your-api-key-here" {
            return Err(JiraDbError::InvalidConfig(
                "JIRA API key must be configured".into(),
            ));
        }

        Ok(())
    }

    /// Find a project by key
    pub fn find_project(&self, key: &str) -> Option<&ProjectConfig> {
        self.projects.iter().find(|p| p.key == key)
    }

    /// Find a mutable project by key
    pub fn find_project_mut(&mut self, key: &str) -> Option<&mut ProjectConfig> {
        self.projects.iter_mut().find(|p| p.key == key)
    }

    /// Add or update a project
    pub fn upsert_project(&mut self, project: ProjectConfig) {
        if let Some(existing) = self.find_project_mut(&project.key) {
            *existing = project;
        } else {
            self.projects.push(project);
        }
    }

    /// Get all projects that should be synced
    pub fn sync_enabled_projects(&self) -> Vec<&ProjectConfig> {
        self.projects
            .iter()
            .filter(|p| p.sync_enabled)
            .collect()
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
        };

        assert!(settings.validate().is_ok());

        settings.jira.api_key = "your-api-key-here".into();
        assert!(settings.validate().is_err());
    }
}
