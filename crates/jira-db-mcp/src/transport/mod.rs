//! Transport layer for MCP communication
//!
//! This module provides abstractions for different transport mechanisms.
//! Supports:
//! - stdio: Standard input/output (for CLI tools like Claude Desktop)
//! - HTTP: HTTP server with JSON-RPC endpoint (for remote access)

pub mod http;
mod stdio;
mod traits;

pub use stdio::*;
pub use traits::*;
