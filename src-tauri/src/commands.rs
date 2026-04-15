use std::sync::{Arc, Mutex};

use serde_json::Value;
use tauri::State;

use crate::db::Database;

type DbState = Arc<Mutex<Database>>;

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn get_project(db: State<'_, DbState>) -> Result<Value, String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    let result = db.conn().query_row(
        "SELECT id, name, description, created_at, updated_at FROM project LIMIT 1",
        [],
        |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "description": row.get::<_, String>(2)?,
                "created_at": row.get::<_, String>(3)?,
                "updated_at": row.get::<_, String>(4)?,
            }))
        },
    );
    drop(db);
    result.map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn list_items(db: State<'_, DbState>) -> Result<Value, String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    let items = db.item_list(None, None, None).map_err(|e| e.to_string())?;
    drop(db);
    serde_json::to_value(items).map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn get_item(db: State<'_, DbState>, id: String) -> Result<Value, String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    let detail = db.item_get(&id).map_err(|e| e.to_string())?;
    drop(db);
    serde_json::to_value(detail).map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn list_tags(db: State<'_, DbState>) -> Result<Value, String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    let tags = db.tag_list().map_err(|e| e.to_string())?;
    drop(db);
    serde_json::to_value(tags).map_err(|e| e.to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn list_connection_types(db: State<'_, DbState>) -> Result<Value, String> {
    let db = db.lock().map_err(|e| e.to_string())?;
    let types = db.connection_type_list().map_err(|e| e.to_string())?;
    drop(db);
    serde_json::to_value(types).map_err(|e| e.to_string())
}
