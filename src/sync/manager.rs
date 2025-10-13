use crate::config::{ProjectConfig, Settings};
use crate::db::{Database, IssueRepository, MetadataRepository, ProjectRepository, SyncHistoryRepository};
use crate::error::Result;
use crate::jira::JiraClient;
use chrono::Utc;
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, warn};

pub struct SyncManager {
    client: JiraClient,
    db: Database,
}

impl SyncManager {
    pub fn new(client: JiraClient, db: Database) -> Self {
        Self { client, db }
    }

    /// Sync all projects from JIRA and update settings
    pub async fn sync_project_list(&self, settings: &mut Settings) -> Result<()> {
        info!("Fetching project list from JIRA...");

        let projects = self.client.fetch_projects().await?;
        info!("Found {} projects", projects.len());

        let project_repo = ProjectRepository::new(self.db.connection());

        for project in &projects {
            project_repo.insert(project)?;

            let project_config = ProjectConfig {
                id: project.id.clone(),
                key: project.key.clone(),
                name: project.name.clone(),
                sync_enabled: false,
                last_synced: None,
            };

            settings.upsert_project(project_config);
        }

        Ok(())
    }

    /// Sync issues for a specific project
    pub async fn sync_project(&self, project_key: &str, settings: &mut Settings) -> Result<()> {
        info!("Syncing project: {}", project_key);

        let project_config = settings
            .find_project(project_key)
            .ok_or_else(|| crate::error::JiraDbError::ProjectNotFound(project_key.to_string()))?;

        let sync_history_repo = SyncHistoryRepository::new(self.db.connection());
        let started_at = Utc::now();
        let history_id = sync_history_repo.insert(&project_config.id, "full", started_at)?;

        match self.sync_project_internal(project_key, &project_config.id).await {
            Ok(count) => {
                let completed_at = Utc::now();
                sync_history_repo.update_completed(history_id, count, completed_at)?;

                if let Some(project) = settings.find_project_mut(project_key) {
                    project.last_synced = Some(completed_at);
                }

                info!("Successfully synced {} issues for project {}", count, project_key);
                Ok(())
            }
            Err(e) => {
                let completed_at = Utc::now();
                sync_history_repo.update_failed(history_id, &e.to_string(), completed_at)?;
                Err(e)
            }
        }
    }

    async fn sync_project_internal(&self, project_key: &str, project_id: &str) -> Result<usize> {
        info!("Fetching issues for project: {}", project_key);

        let issues = self.client.fetch_project_issues(project_key).await?;
        let count = issues.len();

        info!("Fetched {} issues, saving to database...", count);

        // Create progress bar
        let pb = ProgressBar::new(count as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos}/{len} {msg}")
                .expect("Failed to create progress style")
                .progress_chars("█▓▒░ "),
        );
        pb.set_message("Saving issues...");

        // Save issues with progress
        let issue_repo = IssueRepository::new(self.db.connection());

        // Split into chunks for better progress feedback
        let chunk_size = 50;
        for chunk in issues.chunks(chunk_size) {
            issue_repo.batch_insert(chunk)?;
            pb.inc(chunk.len() as u64);
        }

        pb.finish_with_message("Completed!");

        // Fetch and save metadata from JIRA API
        info!("Fetching and saving project metadata...");
        let metadata_repo = MetadataRepository::new(self.db.connection());

        // Fetch statuses
        match self.client.fetch_project_statuses(project_key).await {
            Ok(statuses) => {
                if !statuses.is_empty() {
                    metadata_repo.upsert_statuses(project_id, &statuses)?;
                    info!("Saved {} statuses", statuses.len());
                }
            }
            Err(e) => warn!("Failed to fetch statuses: {}", e),
        }

        // Fetch priorities
        match self.client.fetch_priorities().await {
            Ok(priorities) => {
                if !priorities.is_empty() {
                    metadata_repo.upsert_priorities(project_id, &priorities)?;
                    info!("Saved {} priorities", priorities.len());
                }
            }
            Err(e) => warn!("Failed to fetch priorities: {}", e),
        }

        // Fetch issue types
        match self.client.fetch_project_issue_types(project_id).await {
            Ok(issue_types) => {
                if !issue_types.is_empty() {
                    metadata_repo.upsert_issue_types(project_id, &issue_types)?;
                    info!("Saved {} issue types", issue_types.len());
                }
            }
            Err(e) => warn!("Failed to fetch issue types: {}", e),
        }

        // Fetch labels
        match self.client.fetch_project_labels(project_key).await {
            Ok(labels) => {
                if !labels.is_empty() {
                    metadata_repo.upsert_labels(project_id, &labels)?;
                    info!("Saved {} labels", labels.len());
                }
            }
            Err(e) => warn!("Failed to fetch labels: {}", e),
        }

        // Fetch components
        match self.client.fetch_project_components(project_key).await {
            Ok(components) => {
                if !components.is_empty() {
                    metadata_repo.upsert_components(project_id, &components)?;
                    info!("Saved {} components", components.len());
                }
            }
            Err(e) => warn!("Failed to fetch components: {}", e),
        }

        // Fetch versions
        match self.client.fetch_project_versions(project_key).await {
            Ok(fix_versions) => {
                if !fix_versions.is_empty() {
                    metadata_repo.upsert_fix_versions(project_id, &fix_versions)?;
                    info!("Saved {} fix versions", fix_versions.len());
                }
            }
            Err(e) => warn!("Failed to fetch versions: {}", e),
        }

        Ok(count)
    }

    /// Sync all enabled projects
    pub async fn sync_all(&self, settings: &mut Settings) -> Result<()> {
        let enabled_projects: Vec<String> = settings
            .sync_enabled_projects()
            .iter()
            .map(|p| p.key.clone())
            .collect();

        if enabled_projects.is_empty() {
            warn!("No projects enabled for sync");
            return Ok(());
        }

        info!("Syncing {} projects", enabled_projects.len());

        for project_key in enabled_projects {
            match self.sync_project(&project_key, settings).await {
                Ok(_) => {}
                Err(e) => {
                    warn!("Failed to sync project {}: {}", project_key, e);
                }
            }
        }

        Ok(())
    }
}
