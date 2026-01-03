mod change_history_repository;
mod embeddings_repository;
mod field_repository;
mod issue_repository;
mod issue_snapshot_repository;
mod issues_expanded_repository;
mod metadata_repository;
mod project_repository;
mod sync_history_repository;

pub use change_history_repository::DuckDbChangeHistoryRepository;
pub use embeddings_repository::{EmbeddingsRepository, IssueEmbedding, SemanticSearchResult};
pub use field_repository::DuckDbFieldRepository;
pub use issue_repository::DuckDbIssueRepository;
pub use issue_snapshot_repository::DuckDbIssueSnapshotRepository;
pub use issues_expanded_repository::DuckDbIssuesExpandedRepository;
pub use metadata_repository::DuckDbMetadataRepository;
pub use project_repository::DuckDbProjectRepository;
pub use sync_history_repository::DuckDbSyncHistoryRepository;
