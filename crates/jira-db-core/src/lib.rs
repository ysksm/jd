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
pub use application::dto::{CreatedIssueDto, SyncResult};
pub use application::services::JiraService;
pub use application::use_cases::{
    CreateTestTicketUseCase, EmbeddingGenerationConfig, EmbeddingGenerationResult,
    EmbeddingTiming, GenerateEmbeddingsUseCase, GenerateReportUseCase, GetChangeHistoryUseCase,
    GetProjectMetadataUseCase, ReportData, SearchIssuesUseCase, SyncProjectListUseCase,
    SyncProjectUseCase,
};

pub use domain::entities::{
    ChangeHistoryItem, Component, FixVersion, Issue, IssueType, Label, Priority, Project, Status,
};
pub use domain::error::{DomainError, DomainResult};
pub use domain::repositories::{
    ChangeHistoryRepository, IssueRepository, MetadataRepository, ProjectRepository, SearchParams,
    SyncHistoryRepository,
};

pub use infrastructure::config::{DatabaseConfig, JiraConfig, ProjectConfig, Settings};
pub use infrastructure::database::{
    Database, DbConnection, DuckDbChangeHistoryRepository, DuckDbIssueRepository,
    DuckDbMetadataRepository, DuckDbProjectRepository, DuckDbSyncHistoryRepository,
    EmbeddingsRepository, SemanticSearchResult,
};
pub use infrastructure::external::embeddings::{
    EmbeddingConfig, EmbeddingProvider, EmbeddingResult, OpenAIEmbeddingClient,
};
pub use infrastructure::external::jira::JiraApiClient;

pub use report::{generate_interactive_report, generate_static_report};

// Re-export external crates for CLI use
pub use chrono;
pub use indicatif;
