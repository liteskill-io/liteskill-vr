use std::sync::{Arc, Mutex};

use serde_json::Value;
use tauri::State;

use crate::db::Database;

type DbState = Arc<Mutex<Database>>;

// Full UI snapshot in one call: replaces the N+1 pattern of listItems() followed
// by a getItem() per item that the frontend previously did.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn project_snapshot(db: State<'_, DbState>) -> Result<Value, String> {
    let db = db.lock().map_err(|e| e.to_string())?;

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
    drop(db);

    serde_json::to_value(serde_json::json!({
        "items": items,
        "details": details,
        "tags": tags,
        "connection_types": connection_types,
        "mcp_port": crate::MCP_PORT,
    }))
    .map_err(|e| e.to_string())
}
