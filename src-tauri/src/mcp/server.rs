use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::HeaderMap;
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{json, Value};

use crate::db::Database;

use super::handlers;
use super::protocol::{JsonRpcRequest, JsonRpcResponse, INTERNAL_ERROR, METHOD_NOT_FOUND};
use super::tools;

pub struct McpServer {
    db: Arc<Mutex<Database>>,
}

#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<Database>>,
}

const DEFAULT_AUTHOR: &str = "anonymous-agent";

impl McpServer {
    pub fn new(db: Database) -> Self {
        Self {
            db: Arc::new(Mutex::new(db)),
        }
    }

    pub const fn from_shared(db: Arc<Mutex<Database>>) -> Self {
        Self { db }
    }

    pub async fn start(self, port: u16) -> Result<SocketAddr, std::io::Error> {
        let state = AppState { db: self.db };
        let app = Router::new()
            .route("/mcp", post(handle_mcp))
            .with_state(state);

        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;

        tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });

        Ok(local_addr)
    }
}

async fn handle_mcp(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    let author = headers
        .get("X-LiteSkill-Author")
        .and_then(|v| v.to_str().ok())
        .unwrap_or(DEFAULT_AUTHOR);

    let response = match request.method.as_str() {
        "initialize" => JsonRpcResponse::success(
            request.id,
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {
                    "name": "liteskill-vr",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }),
        ),

        "tools/list" => JsonRpcResponse::success(request.id, json!({"tools": tools::list_all()})),

        "tools/call" => {
            let params = request.params.unwrap_or_else(|| json!({}));
            let Some(tool_name) = params.get("name").and_then(Value::as_str) else {
                return Json(JsonRpcResponse::error(
                    request.id,
                    METHOD_NOT_FOUND,
                    "Missing tool name in params.name".to_string(),
                ));
            };
            let tool_args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));

            let Ok(db) = state.db.lock() else {
                return Json(JsonRpcResponse::error(
                    request.id,
                    INTERNAL_ERROR,
                    "Database lock poisoned".to_string(),
                ));
            };

            let result = handlers::dispatch(&db, tool_name, &tool_args, author);
            drop(db);
            match result.error {
                Some((code, msg, data)) => {
                    if let Some(d) = data {
                        JsonRpcResponse::error_with_data(request.id, code, msg, d)
                    } else {
                        JsonRpcResponse::error(request.id, code, msg)
                    }
                }
                None => JsonRpcResponse::success(
                    request.id,
                    json!({"content": [{"type": "text", "text": serde_json::to_string(&result.value).unwrap_or_default()}]}),
                ),
            }
        }

        _ => JsonRpcResponse::error(
            request.id,
            METHOD_NOT_FOUND,
            format!("Unknown method: {}", request.method),
        ),
    };

    Json(response)
}
