//! stdio transport implementation
//!
//! Implements the MCP transport over standard input/output.
//! Messages are newline-delimited JSON.

use async_trait::async_trait;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Stdin, Stdout};

use crate::protocol::{JsonRpcRequest, ProtocolError, ProtocolResult};

use super::Transport;

/// stdio transport for MCP communication
///
/// Uses stdin for reading requests and stdout for sending responses.
/// Each message is a single line of JSON.
pub struct StdioTransport {
    reader: BufReader<Stdin>,
    writer: Stdout,
}

impl StdioTransport {
    /// Create a new stdio transport
    pub fn new() -> Self {
        Self {
            reader: BufReader::new(tokio::io::stdin()),
            writer: tokio::io::stdout(),
        }
    }
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn read_request(&mut self) -> ProtocolResult<Option<JsonRpcRequest>> {
        let mut line = String::new();

        match self.reader.read_line(&mut line).await {
            Ok(0) => {
                // EOF reached
                Ok(None)
            }
            Ok(_) => {
                let line = line.trim();
                if line.is_empty() {
                    // Empty line, try to read the next one
                    return Box::pin(self.read_request()).await;
                }

                tracing::debug!("Received: {}", line);

                let request: JsonRpcRequest = serde_json::from_str(line).map_err(|e| {
                    tracing::error!("Failed to parse JSON: {}", e);
                    ProtocolError::JsonParse(e)
                })?;

                Ok(Some(request))
            }
            Err(e) => Err(ProtocolError::Io(e)),
        }
    }

    async fn send_response(&mut self, response: Value) -> ProtocolResult<()> {
        let json = serde_json::to_string(&response)?;
        tracing::debug!("Sending: {}", json);

        self.writer.write_all(json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;

        Ok(())
    }

    async fn close(&mut self) -> ProtocolResult<()> {
        self.writer.flush().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stdio_transport_creation() {
        // Just verify we can create the transport
        // Actual I/O testing would require mocking stdin/stdout
        let _transport = StdioTransport::new();
    }
}
