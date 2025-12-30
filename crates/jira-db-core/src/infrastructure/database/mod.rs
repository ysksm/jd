mod connection;
mod repositories;
mod schema;

pub use connection::Database;
pub use repositories::{
    DuckDbChangeHistoryRepository, DuckDbIssueRepository, DuckDbMetadataRepository,
    DuckDbProjectRepository, DuckDbSyncHistoryRepository, EmbeddingsRepository, IssueEmbedding,
    SemanticSearchResult,
};
