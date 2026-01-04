//! JiraDb Service Layer
//!
//! This crate provides a shared service layer that can be used by both
//! Tauri commands and HTTP handlers (ActixWeb, Axum, etc.).
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────┐
//! │  Presentation Layer                                        │
//! │  ┌──────────────┬──────────────┬──────────────────────────┐│
//! │  │ Tauri        │ ActixWeb     │ Axum                     ││
//! │  │ (commands)   │ (handlers)   │ (handlers)               ││
//! │  └──────────────┴──────────────┴──────────────────────────┘│
//! └────────────────────────────────────────────────────────────┘
//!                           │
//!                           ▼
//! ┌────────────────────────────────────────────────────────────┐
//! │  jira-db-service (This Crate)                              │
//! │  ┌────────────────────────────────────────────────────────┐│
//! │  │ Services: config, projects, issues, sync, etc.         ││
//! │  │ Types: Request/Response DTOs                           ││
//! │  │ State: AppState for shared state management            ││
//! │  └────────────────────────────────────────────────────────┘│
//! └────────────────────────────────────────────────────────────┘
//!                           │
//!                           ▼
//! ┌────────────────────────────────────────────────────────────┐
//! │  jira-db-core (Business Logic)                             │
//! └────────────────────────────────────────────────────────────┘
//! ```

pub mod error;
pub mod services;
pub mod state;
pub mod types;

// Re-export main types and functions for convenience
pub use error::{ServiceError, ServiceResult};
pub use state::AppState;
pub use types::*;

// Re-export service modules
pub use services::config;
pub use services::embeddings;
pub use services::issues;
pub use services::metadata;
pub use services::projects;
pub use services::reports;
pub use services::sql;
pub use services::sync;
