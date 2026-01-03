mod connection;
mod repositories;
mod schema;

pub use connection::{Database, DbConnection};
pub use repositories::{
    DuckDbChangeHistoryRepository, DuckDbFieldRepository, DuckDbIssueRepository,
    DuckDbIssueSnapshotRepository, DuckDbIssuesExpandedRepository, DuckDbMetadataRepository,
    DuckDbProjectRepository, DuckDbSyncHistoryRepository, EmbeddingsRepository, IssueEmbedding,
    SemanticSearchResult,
};
