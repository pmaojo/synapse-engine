use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, sse::{Event, Sse}},
    routing::{get, post},
    Router,
};
use futures::stream::Stream;
use serde_json::Value;
use std::convert::Infallible;
use std::sync::Arc;
use tokio_stream::StreamExt;
use tower_http::cors::CorsLayer;

use crate::store::SynapseStore;
use super::protocol::{JsonRpcRequest, JsonRpcResponse};
use super::tools::handle_tool_call;

#[derive(Clone)]
struct AppState {
    store: Arc<SynapseStore>,
}

pub async fn start_mcp_server(port: u16, store: Arc<SynapseStore>) -> Result<(), Box<dyn std::error::Error>> {
    let state = AppState { store };

    let app = Router::new()
        .route("/mcp", post(handle_mcp_request))
        // Endpoint for SSE or Ext-Apps to fetch real-time updates directly if needed outside of standard MCP calls
        .route("/mcp/sse", get(handle_sse_request))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("🚀 MCP HTTP/SSE Server listening on {}", addr);

    axum::serve(listener, app).await?;
    Ok(())
}

async fn handle_mcp_request(
    State(state): State<AppState>,
    Json(req): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let method = req.method.as_str();
    let id = req.id.unwrap_or(Value::Null);

    match handle_tool_call(method, req.params, state.store.clone()).await {
        Ok(res) => (
            StatusCode::OK,
            Json(JsonRpcResponse::success(id, res)),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(JsonRpcResponse::error(id, -32000, &e)),
        ),
    }
}

async fn handle_sse_request() -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Basic placeholder for Server-Sent Events stream for Ext-Apps
    // E.g., streaming inference logs or acting as the notification channel for MCP
    let stream = tokio_stream::wrappers::IntervalStream::new(tokio::time::interval(
        std::time::Duration::from_secs(15),
    ))
    .map(|_| {
        Ok(Event::default()
            .event("ping")
            .data("keep-alive"))
    });

    Sse::new(stream).keep_alive(axum::response::sse::KeepAlive::new())
}
