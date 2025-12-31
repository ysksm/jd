//! MCP Protocol implementation
//!
//! This module implements the Model Context Protocol (MCP) over JSON-RPC 2.0.
//! It provides types and utilities for handling MCP messages.

mod error;
mod jsonrpc;
mod mcp_types;

pub use error::*;
pub use jsonrpc::*;
pub use mcp_types::*;
