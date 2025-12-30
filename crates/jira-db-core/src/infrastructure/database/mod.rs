mod connection;
mod repositories;
mod schema;

pub use connection::{Database, DbConnection};
pub use repositories::{
    DuckDbChangeHistoryRepository, DuckDbIssueRepository, DuckDbMetadataRepository,
    DuckDbProjectRepository, DuckDbSyncHistoryRepository, EmbeddingsRepository, IssueEmbedding,
    SemanticSearchResult,
};
