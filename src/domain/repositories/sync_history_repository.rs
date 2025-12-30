use chrono::{DateTime, Utc};
use crate::domain::error::DomainResult;

/// Repository trait for SyncHistory entity
/// Infrastructure layer will implement this trait
#[allow(dead_code)]
pub trait SyncHistoryRepository: Send + Sync {
    fn insert(
        &self,
        project_id: &str,
        sync_type: &str,
        started_at: DateTime<Utc>,
    ) -> DomainResult<i64>;

    fn update_completed(
        &self,
        id: i64,
        items_synced: usize,
        completed_at: DateTime<Utc>,
    ) -> DomainResult<()>;

    fn update_failed(
        &self,
        id: i64,
        error_message: &str,
        completed_at: DateTime<Utc>,
    ) -> DomainResult<()>;

    fn find_latest_by_project(&self, project_id: &str) -> DomainResult<Option<(DateTime<Utc>, String)>>;
}
