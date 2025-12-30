//! Transport layer for MCP communication
//!
//! This module provides abstractions for different transport mechanisms.
//! Currently supports:
//! - stdio: Standard input/output (for CLI tools like Claude Desktop)
//!
//! Future support planned for:
//! - HTTP with SSE (for remote access)

mod stdio;
mod traits;

pub use stdio::*;
pub use traits::*;
