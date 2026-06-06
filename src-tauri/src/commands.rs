use std::sync::{Arc, Mutex};

use serde_json::Value;
use tauri::{Emitter, State};

use crate::db::Database;
use crate::mcp::handlers;

type DbState = Arc<Mutex<Database>>;

// Human writes are attributed to the OS user; agents use the X-LiteSkill-Author
// header. Identity is caller-controlled, never read from tool params.
fn os_username() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "local-user".to_string())
}

/// The single human write path.
///
/// Runs the SAME tool dispatch the MCP server uses, stamped
/// `author_type = "human"`, so anything an agent can do the UI does through this
/// identical code path (parity). Emits `db-changed` on success so the snapshot
/// refetches.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn mcp_call(
    app: tauri::AppHandle,
    db: State<'_, DbState>,
    tool: String,
    args: Value,
) -> Result<Value, String> {
    let result = {
        // Recover a poisoned lock (a prior panic) instead of wedging all writes;
        // SQLite keeps its own consistency, so the guard is safe to reuse.
        let db = db.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        handlers::dispatch(&db, &tool, &args, &os_username(), "human")
    };
    if result.is_ok() {
        let _ = app.emit("db-changed", ());
    }
    result
}

// Full UI snapshot in one call: replaces the N+1 pattern of listItems() followed
// by a getItem() per item that the frontend previously did.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn project_snapshot(db: State<'_, DbState>) -> Result<Value, String> {
    let db = db.lock().unwrap_or_else(std::sync::PoisonError::into_inner);

    let items = db.item_list(None, None, None).map_err(|e| e.to_string())?;
    let mut details = Vec::with_capacity(items.len());
    for summary in &items {
        let detail = db
            .item_get(&summary.item.item.id)
            .map_err(|e| e.to_string())?;
        details.push(detail);
    }
    let tags = db.tag_list().map_err(|e| e.to_string())?;
    let connection_types = db.connection_type_list().map_err(|e| e.to_string())?;

    // Explanations: list summaries + full detail each (small N, like items).
    let explanations = db.explanation_list(None, None).map_err(|e| e.to_string())?;
    let mut explanation_details = Vec::with_capacity(explanations.len());
    for summary in &explanations {
        let detail = db
            .explanation_get(&summary.explanation.id)
            .map_err(|e| e.to_string())?;
        explanation_details.push(detail);
    }
    drop(db);

    serde_json::to_value(serde_json::json!({
        "items": items,
        "details": details,
        "tags": tags,
        "connection_types": connection_types,
        "explanations": explanations,
        "explanation_details": explanation_details,
        "mcp_port": crate::MCP_PORT,
    }))
    .map_err(|e| e.to_string())
}
