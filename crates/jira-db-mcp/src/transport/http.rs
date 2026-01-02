//! HTTP transport implementation with SSE support
//!
//! Provides an HTTP server for MCP communication.
//! - POST /mcp - JSON-RPC endpoint for MCP requests
//! - GET /mcp/sse - Server-Sent Events for streaming (future)

use std::sync::Arc;

use actix_web::{HttpResponse, Responder, web};

use crate::handlers::RequestHandler;
use crate::protocol::{JsonRpcRequest, ProtocolError};
use crate::tools::ToolRegistry;

/// State shared across HTTP handlers
pub struct HttpState {
    handler: tokio::sync::Mutex<RequestHandler>,
}

impl HttpState {
    /// Create new HTTP state with the given tool registry
    pub fn new(tool_registry: ToolRegistry) -> Self {
        Self {
            handler: tokio::sync::Mutex::new(RequestHandler::new(tool_registry)),
        }
    }
}

/// Handle raw JSON-RPC requests
pub async fn handle_mcp_raw(state: web::Data<Arc<HttpState>>, body: web::Bytes) -> impl Responder {
    // Parse the request
    let request: JsonRpcRequest = match serde_json::from_slice(&body) {
        Ok(req) => req,
        Err(e) => {
            let error_response = ProtocolError::JsonParse(e).to_error_response(None);
            return HttpResponse::Ok()
                .content_type("application/json")
                .json(error_response);
        }
    };

    let id = request.id.clone();
    let mut handler = state.handler.lock().await;

    match handler.handle(request).await {
        Ok(Some(response)) => HttpResponse::Ok()
            .content_type("application/json")
            .json(response),
        Ok(None) => HttpResponse::NoContent().finish(),
        Err(e) => {
            let error_response = e.to_error_response(id);
            HttpResponse::Ok()
                .content_type("application/json")
                .json(error_response)
        }
    }
}

/// Health check endpoint
pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "service": "jira-db-mcp"
    }))
}

/// Get server info
pub async fn server_info() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "name": "jira-db-mcp",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": "MCP",
        "transport": "HTTP"
    }))
}

/// Configure HTTP routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/mcp")
            .route("", web::post().to(handle_mcp_raw))
            .route("/", web::post().to(handle_mcp_raw)),
    )
    .route("/health", web::get().to(health_check))
    .route("/info", web::get().to(server_info));
}
