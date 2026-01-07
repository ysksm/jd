//! jira-db-core: Core library for JIRA data synchronization to DuckDB
//!
//! This library provides the core functionality for:
//! - Domain entities and repository interfaces
//! - Application use cases and services
//! - Infrastructure implementations (database, JIRA API, config)
//! - Report generation

pub mod application;
pub mod domain;
pub mod infrastructure;
pub mod report;

// Re-export commonly used types for convenience
pub use application::dto::{CreatedIssueDto, SyncResult, TransitionDto};
pub use application::services::{FetchProgress, JiraService};
pub use application::use_cases::{
    CreateTestTicketUseCase, EmbeddingGenerationConfig, EmbeddingGenerationResult, EmbeddingTiming,
    ExecuteSqlUseCase, GenerateEmbeddingsUseCase, GenerateReportUseCase, GenerateSnapshotsUseCase,
    GetChangeHistoryUseCase, GetProjectMetadataUseCase, ReportData, ResumableSyncResult,
    SearchIssuesUseCase, SnapshotGenerationResult, SqlResult, SyncFieldsResult, SyncFieldsUseCase,
    SyncProjectListUseCase, SyncProjectUseCase, TransitionIssueUseCase, TransitionResult,
};

pub use domain::entities::{
    ChangeHistoryItem, Component, FixVersion, Issue, IssueSnapshot, IssueType, JiraField, Label,
    Priority, Project, Status,
};
pub use domain::error::{DomainError, DomainResult};
pub use domain::repositories::{
    ChangeHistoryRepository, IssueRepository, IssueSnapshotRepository, MetadataRepository,
    ProjectRepository, SearchParams, SyncHistoryRepository,
};

pub use infrastructure::config::{
    DatabaseConfig, EmbeddingsConfig, JiraConfig, LogConfig, ProjectConfig, Settings,
    SyncCheckpoint,
};
pub use infrastructure::database::{
    Database, DatabaseFactory, DbConnection, DuckDbChangeHistoryRepository, DuckDbFieldRepository,
    DuckDbIssueRepository, DuckDbIssueSnapshotRepository, DuckDbIssuesExpandedRepository,
    DuckDbMetadataRepository, DuckDbProjectRepository, DuckDbSyncHistoryRepository,
    EmbeddingsRepository, RawDataRepository, SemanticSearchResult, SharedRawDataRepository,
    checkpoint_connection,
};
pub use infrastructure::external::embeddings::{
    CohereConfig, CohereEmbeddingClient, EmbeddingConfig, EmbeddingProvider, EmbeddingProviderType,
    EmbeddingResult, OllamaConfig, OllamaEmbeddingClient, OpenAIEmbeddingClient, ProviderConfig,
    create_provider,
};
pub use infrastructure::external::jira::JiraApiClient;

pub use report::{generate_interactive_report, generate_static_report};

// Re-export external crates for CLI use
pub use chrono;
pub use indicatif;
