use serde_json::{json, Value};

use crate::db::error::DbError;
use crate::db::{Database, NewConnection, NewIoi};

use super::protocol::{INTERNAL_ERROR, INVALID_PARAMS};

pub struct HandlerResult {
    pub value: Option<Value>,
    pub error: Option<(i32, String, Option<Value>)>,
}

impl HandlerResult {
    const fn ok(v: Value) -> Self {
        Self {
            value: Some(v),
            error: None,
        }
    }

    fn err(code: i32, msg: impl Into<String>) -> Self {
        Self {
            value: None,
            error: Some((code, msg.into(), None)),
        }
    }

    fn err_with_data(code: i32, msg: impl Into<String>, data: Value) -> Self {
        Self {
            value: None,
            error: Some((code, msg.into(), Some(data))),
        }
    }
}

fn param_str<'a>(params: &'a Value, key: &str) -> Option<&'a str> {
    params.get(key).and_then(Value::as_str)
}

fn param_str_required<'a>(params: &'a Value, key: &str) -> Result<&'a str, HandlerResult> {
    param_str(params, key).ok_or_else(|| {
        HandlerResult::err(INVALID_PARAMS, format!("Missing required parameter: {key}"))
    })
}

fn param_tags(params: &Value, key: &str) -> Vec<String> {
    params
        .get(key)
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

fn db_err_to_handler(e: DbError) -> HandlerResult {
    match e {
        DbError::NotFound { entity, id } => {
            HandlerResult::err(INVALID_PARAMS, format!("{entity} '{id}' not found"))
        }
        DbError::DuplicateName { entity, name } => {
            HandlerResult::err(INVALID_PARAMS, format!("{entity} '{name}' already exists"))
        }
        DbError::UnregisteredTag(name) => HandlerResult::err_with_data(
            INVALID_PARAMS,
            format!("Tag '{name}' is not registered"),
            json!({"suggestion": "Call tag_list() to see registered tags, or tag_create() to register a new one."}),
        ),
        DbError::UnregisteredConnectionType(name) => HandlerResult::err_with_data(
            INVALID_PARAMS,
            format!("Connection type '{name}' is not registered"),
            json!({"suggestion": "Call connection_type_list() to see registered types, or connection_type_create() to register a new one."}),
        ),
        DbError::InvalidReference { entity, id } => {
            HandlerResult::err(INVALID_PARAMS, format!("{entity} '{id}' does not exist"))
        }
        DbError::BulkDeleteNoFilter => HandlerResult::err(
            INVALID_PARAMS,
            "bulk_delete requires at least one filter (author, since, or entity_type)",
        ),
        DbError::Sqlite(e) => HandlerResult::err(INTERNAL_ERROR, format!("Database error: {e}")),
    }
}

pub fn dispatch(db: &Database, tool_name: &str, params: &Value, author: &str) -> HandlerResult {
    let result = match tool_name {
        // Project
        "project_get" => handle_project_get(db),
        "project_summary" => handle_project_summary(db),
        "changes_since" => handle_changes_since(db, params),

        // Tags
        "tag_list" => handle_tag_list(db),
        "tag_create" => handle_tag_create(db, params),
        "tag_delete" => handle_tag_delete(db, params),

        // Connection Types
        "connection_type_list" => handle_connection_type_list(db),
        "connection_type_create" => handle_connection_type_create(db, params),
        "connection_type_delete" => handle_connection_type_delete(db, params),

        // Items
        "item_list" => handle_item_list(db, params),
        "item_get" => handle_item_get(db, params),
        "item_create" => handle_item_create(db, params),
        "item_create_batch" => handle_item_create_batch(db, params),
        "item_update" => handle_item_update(db, params),
        "item_delete" => handle_item_delete(db, params),

        // Notes
        "note_create" => handle_note_create(db, params, author),
        "note_create_batch" => handle_note_create_batch(db, params, author),
        "note_update" => handle_note_update(db, params),
        "note_delete" => handle_note_delete(db, params),

        // IOI
        "ioi_create" => handle_ioi_create(db, params, author),
        "ioi_create_batch" => handle_ioi_create_batch(db, params, author),
        "ioi_update" => handle_ioi_update(db, params),
        "ioi_delete" => handle_ioi_delete(db, params),

        // Connections
        "connection_create" => handle_connection_create(db, params, author),
        "connection_create_batch" => handle_connection_create_batch(db, params, author),
        "connection_list" => handle_connection_list(db, params),
        "connection_list_all" => handle_connection_list_all(db),
        "connection_delete" => handle_connection_delete(db, params),

        // Search & Filter
        "search" => handle_search(db, params),
        "filter" => handle_filter(db, params),

        // Bulk
        "bulk_delete" => handle_bulk_delete(db, params),

        _ => {
            return HandlerResult::err(
                super::protocol::METHOD_NOT_FOUND,
                format!("Unknown tool: {tool_name}"),
            )
        }
    };
    result
}

// --- Project ---

fn handle_project_get(db: &Database) -> HandlerResult {
    match db.conn().query_row(
        "SELECT id, name, description, created_at, updated_at FROM project LIMIT 1",
        [],
        |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "description": row.get::<_, String>(2)?,
                "created_at": row.get::<_, String>(3)?,
                "updated_at": row.get::<_, String>(4)?,
            }))
        },
    ) {
        Ok(v) => HandlerResult::ok(v),
        Err(e) => HandlerResult::err(INTERNAL_ERROR, format!("Database error: {e}")),
    }
}

fn handle_project_summary(db: &Database) -> HandlerResult {
    let items = match db.item_list(None, None, None) {
        Ok(v) => v,
        Err(e) => return db_err_to_handler(e),
    };
    let tags = match db.tag_list() {
        Ok(v) => v,
        Err(e) => return db_err_to_handler(e),
    };
    let conn_types = match db.connection_type_list() {
        Ok(v) => v,
        Err(e) => return db_err_to_handler(e),
    };

    let mut severity_counts = json!({"critical": 0, "high": 0, "medium": 0, "low": 0, "info": 0});
    for item in &items {
        let iois = match db.filter_ioi(Some(&item.item.item.id), None, None, None) {
            Ok(v) => v,
            Err(e) => return db_err_to_handler(e),
        };
        for ioi in &iois {
            if let Some(ref sev) = ioi.ioi.severity {
                if let Some(count) = severity_counts.get_mut(sev) {
                    *count = json!(count.as_i64().unwrap_or(0) + 1);
                }
            }
        }
    }

    HandlerResult::ok(json!({
        "items": serde_json::to_value(&items).unwrap_or_default(),
        "severity_summary": severity_counts,
        "tags": serde_json::to_value(&tags).unwrap_or_default(),
        "connection_types": serde_json::to_value(&conn_types).unwrap_or_default(),
    }))
}

fn handle_changes_since(db: &Database, params: &Value) -> HandlerResult {
    let since = match param_str_required(params, "since") {
        Ok(v) => v,
        Err(e) => return e,
    };

    let items = db.conn().prepare(
        "SELECT id, name, item_type, created_at, updated_at FROM items WHERE created_at >= ?1 OR updated_at >= ?1 ORDER BY updated_at",
    ).and_then(|mut stmt| {
        stmt.query_map([since], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "name": row.get::<_, String>(1)?,
                "item_type": row.get::<_, String>(2)?,
                "created_at": row.get::<_, String>(3)?,
                "updated_at": row.get::<_, String>(4)?,
            }))
        }).and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
    });

    let notes = db.conn().prepare(
        "SELECT id, item_id, title, author, created_at FROM notes WHERE created_at >= ?1 OR updated_at >= ?1 ORDER BY updated_at",
    ).and_then(|mut stmt| {
        stmt.query_map([since], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "item_id": row.get::<_, String>(1)?,
                "title": row.get::<_, String>(2)?,
                "author": row.get::<_, String>(3)?,
                "created_at": row.get::<_, String>(4)?,
            }))
        }).and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
    });

    let iois = db.conn().prepare(
        "SELECT id, item_id, title, severity, author, created_at FROM items_of_interest WHERE created_at >= ?1 OR updated_at >= ?1 ORDER BY updated_at",
    ).and_then(|mut stmt| {
        stmt.query_map([since], |row| {
            Ok(json!({
                "id": row.get::<_, String>(0)?,
                "item_id": row.get::<_, String>(1)?,
                "title": row.get::<_, String>(2)?,
                "severity": row.get::<_, Option<String>>(3)?,
                "author": row.get::<_, String>(4)?,
                "created_at": row.get::<_, String>(5)?,
            }))
        }).and_then(|rows| rows.collect::<Result<Vec<_>, _>>())
    });

    match (items, notes, iois) {
        (Ok(items), Ok(notes), Ok(iois)) => HandlerResult::ok(json!({
            "items": items,
            "notes": notes,
            "items_of_interest": iois,
        })),
        _ => HandlerResult::err(INTERNAL_ERROR, "Failed to query changes"),
    }
}

// --- Tags ---

fn handle_tag_list(db: &Database) -> HandlerResult {
    match db.tag_list() {
        Ok(tags) => HandlerResult::ok(serde_json::to_value(tags).unwrap_or_default()),
        Err(e) => db_err_to_handler(e),
    }
}

fn handle_tag_create(db: &Database, params: &Value) -> HandlerResult {
    let name = match param_str_required(params, "name") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let description = param_str(params, "description").unwrap_or("");
    let color = param_str(params, "color");

    match db.tag_create(name, description, color) {
        Ok(tag) => HandlerResult::ok(serde_json::to_value(tag).unwrap_or_default()),
        Err(e) => db_err_to_handler(e),
    }
}

fn handle_tag_delete(db: &Database, params: &Value) -> HandlerResult {
    let id = match param_str_required(params, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };
    match db.tag_delete(id) {
        Ok(()) => HandlerResult::ok(json!({"deleted": true})),
        Err(e) => db_err_to_handler(e),
    }
}

// --- Connection Types ---

fn handle_connection_type_list(db: &Database) -> HandlerResult {
    match db.connection_type_list() {
        Ok(types) => HandlerResult::ok(serde_json::to_value(types).unwrap_or_default()),
        Err(e) => db_err_to_handler(e),
    }
}

fn handle_connection_type_create(db: &Database, params: &Value) -> HandlerResult {
    let name = match param_str_required(params, "name") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let description = param_str(params, "description").unwrap_or("");
    match db.connection_type_create(name, description) {
        Ok(ct) => HandlerResult::ok(serde_json::to_value(ct).unwrap_or_default()),
        Err(e) => db_err_to_handler(e),
    }
}

fn handle_connection_type_delete(db: &Database, params: &Value) -> HandlerResult {
    let id = match param_str_required(params, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };
    match db.connection_type_delete(id) {
        Ok(()) => HandlerResult::ok(json!({"deleted": true})),
        Err(e) => db_err_to_handler(e),
    }
}

// --- Items ---

fn handle_item_list(db: &Database, params: &Value) -> HandlerResult {
    let item_type = param_str(params, "item_type");
    let status = param_str(params, "analysis_status");
    let tags = params.get("tags").and_then(Value::as_array).map(|arr| {
        arr.iter()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect::<Vec<_>>()
    });

    match db.item_list(item_type, status, tags.as_deref()) {
        Ok(items) => HandlerResult::ok(serde_json::to_value(items).unwrap_or_default()),
        Err(e) => db_err_to_handler(e),
    }
}

fn handle_item_get(db: &Database, params: &Value) -> HandlerResult {
    let id = match param_str_required(params, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };
    match db.item_get(id) {
        Ok(detail) => HandlerResult::ok(serde_json::to_value(detail).unwrap_or_default()),
        Err(e) => db_err_to_handler(e),
    }
}

fn handle_item_create(db: &Database, params: &Value) -> HandlerResult {
    let name = match param_str_required(params, "name") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let item_type = match param_str_required(params, "item_type") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let path = param_str(params, "path");
    let arch = param_str(params, "architecture");
    let desc = param_str(params, "description").unwrap_or("");
    let tags = param_tags(params, "tags");

    match db.item_create(name, item_type, path, arch, desc, &tags) {
        Ok(item) => HandlerResult::ok(serde_json::to_value(item).unwrap_or_default()),
        Err(e) => db_err_to_handler(e),
    }
}

fn handle_item_create_batch(db: &Database, params: &Value) -> HandlerResult {
    let Some(items) = params.get("items").and_then(Value::as_array) else {
        return HandlerResult::err(INVALID_PARAMS, "Missing required parameter: items");
    };

    let mut results = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        let Some(name) = param_str(item, "name") else {
            return HandlerResult::err_with_data(
                INVALID_PARAMS,
                format!("Item at index {i} missing 'name'"),
                json!({"index": i}),
            );
        };
        let Some(item_type) = param_str(item, "item_type") else {
            return HandlerResult::err_with_data(
                INVALID_PARAMS,
                format!("Item at index {i} missing 'item_type'"),
                json!({"index": i}),
            );
        };
        let path = param_str(item, "path");
        let arch = param_str(item, "architecture");
        let desc = param_str(item, "description").unwrap_or("");
        let tags = param_tags(item, "tags");

        match db.item_create(name, item_type, path, arch, desc, &tags) {
            Ok(created) => results.push(created),
            Err(e) => {
                // Rollback: delete everything we created
                for r in &results {
                    let _ = db.item_delete(&r.item.id);
                }
                return HandlerResult::err_with_data(
                    INVALID_PARAMS,
                    format!("Batch failed at index {i}: {e}"),
                    json!({"index": i}),
                );
            }
        }
    }
    HandlerResult::ok(serde_json::to_value(results).unwrap_or_default())
}

fn handle_item_update(db: &Database, params: &Value) -> HandlerResult {
    let id = match param_str_required(params, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let name = param_str(params, "name");
    let desc = param_str(params, "description");
    let status = param_str(params, "analysis_status");
    let tags = params.get("tags").and_then(Value::as_array).map(|arr| {
        arr.iter()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect::<Vec<_>>()
    });

    match db.item_update(id, name, desc, status, tags.as_deref()) {
        Ok(item) => HandlerResult::ok(serde_json::to_value(item).unwrap_or_default()),
        Err(e) => db_err_to_handler(e),
    }
}

fn handle_item_delete(db: &Database, params: &Value) -> HandlerResult {
    let id = match param_str_required(params, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };
    match db.item_delete(id) {
        Ok(()) => HandlerResult::ok(json!({"deleted": true})),
        Err(e) => db_err_to_handler(e),
    }
}

// --- Notes ---

fn handle_note_create(db: &Database, params: &Value, author: &str) -> HandlerResult {
    let item_id = match param_str_required(params, "item_id") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let title = match param_str_required(params, "title") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let content = match param_str_required(params, "content") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let tags = param_tags(params, "tags");

    match db.note_create(item_id, title, content, author, "agent", &tags) {
        Ok(note) => HandlerResult::ok(serde_json::to_value(note).unwrap_or_default()),
        Err(e) => db_err_to_handler(e),
    }
}

fn handle_note_create_batch(db: &Database, params: &Value, author: &str) -> HandlerResult {
    let Some(notes) = params.get("notes").and_then(Value::as_array) else {
        return HandlerResult::err(INVALID_PARAMS, "Missing required parameter: notes");
    };

    let mut results = Vec::with_capacity(notes.len());
    for (i, note) in notes.iter().enumerate() {
        let Some(item_id) = param_str(note, "item_id") else {
            return batch_rollback_notes(db, &results, i, "'item_id'");
        };
        let Some(title) = param_str(note, "title") else {
            return batch_rollback_notes(db, &results, i, "'title'");
        };
        let Some(content) = param_str(note, "content") else {
            return batch_rollback_notes(db, &results, i, "'content'");
        };
        let tags = param_tags(note, "tags");

        match db.note_create(item_id, title, content, author, "agent", &tags) {
            Ok(created) => results.push(created),
            Err(e) => {
                for r in &results {
                    let _ = db.note_delete(&r.note.id);
                }
                return HandlerResult::err_with_data(
                    INVALID_PARAMS,
                    format!("Batch failed at index {i}: {e}"),
                    json!({"index": i}),
                );
            }
        }
    }
    HandlerResult::ok(serde_json::to_value(results).unwrap_or_default())
}

fn batch_rollback_notes(
    db: &Database,
    results: &[crate::db::models::NoteWithTags],
    index: usize,
    field: &str,
) -> HandlerResult {
    for r in results {
        let _ = db.note_delete(&r.note.id);
    }
    HandlerResult::err_with_data(
        INVALID_PARAMS,
        format!("Note at index {index} missing {field}"),
        json!({"index": index}),
    )
}

fn handle_note_update(db: &Database, params: &Value) -> HandlerResult {
    let id = match param_str_required(params, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let title = param_str(params, "title");
    let content = param_str(params, "content");
    let tags = params.get("tags").and_then(Value::as_array).map(|arr| {
        arr.iter()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect::<Vec<_>>()
    });

    match db.note_update(id, title, content, tags.as_deref()) {
        Ok(note) => HandlerResult::ok(serde_json::to_value(note).unwrap_or_default()),
        Err(e) => db_err_to_handler(e),
    }
}

fn handle_note_delete(db: &Database, params: &Value) -> HandlerResult {
    let id = match param_str_required(params, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };
    match db.note_delete(id) {
        Ok(()) => HandlerResult::ok(json!({"deleted": true})),
        Err(e) => db_err_to_handler(e),
    }
}

// --- IOI ---

fn handle_ioi_create(db: &Database, params: &Value, author: &str) -> HandlerResult {
    let item_id = match param_str_required(params, "item_id") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let title = match param_str_required(params, "title") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let description = match param_str_required(params, "description") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let location = param_str(params, "location");
    let severity = param_str(params, "severity");
    let tags = param_tags(params, "tags");

    match db.ioi_create(&NewIoi {
        item_id,
        title,
        description,
        location,
        severity,
        author,
        author_type: "agent",
        tags: &tags,
    }) {
        Ok((ioi, warning)) => {
            let mut result = serde_json::to_value(ioi).unwrap_or_default();
            if let Some(w) = warning {
                result["duplicate_warning"] = json!({
                    "existing_id": w.existing_id,
                    "existing_title": w.existing_title,
                });
            }
            HandlerResult::ok(result)
        }
        Err(e) => db_err_to_handler(e),
    }
}

fn handle_ioi_create_batch(db: &Database, params: &Value, author: &str) -> HandlerResult {
    let item_id = match param_str_required(params, "item_id") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let Some(items) = params.get("items").and_then(Value::as_array) else {
        return HandlerResult::err(INVALID_PARAMS, "Missing required parameter: items");
    };

    let mut results = Vec::with_capacity(items.len());
    let mut created_ids: Vec<String> = Vec::with_capacity(items.len());

    for (i, item) in items.iter().enumerate() {
        let Some(title) = param_str(item, "title") else {
            for id in &created_ids {
                let _ = db.ioi_delete(id);
            }
            return HandlerResult::err_with_data(
                INVALID_PARAMS,
                format!("Item at index {i} missing 'title'"),
                json!({"index": i}),
            );
        };
        let Some(description) = param_str(item, "description") else {
            for id in &created_ids {
                let _ = db.ioi_delete(id);
            }
            return HandlerResult::err_with_data(
                INVALID_PARAMS,
                format!("Item at index {i} missing 'description'"),
                json!({"index": i}),
            );
        };
        let location = param_str(item, "location");
        let severity = param_str(item, "severity");
        let tags = param_tags(item, "tags");

        match db.ioi_create(&NewIoi {
            item_id,
            title,
            description,
            location,
            severity,
            author,
            author_type: "agent",
            tags: &tags,
        }) {
            Ok((ioi, warning)) => {
                created_ids.push(ioi.ioi.id.clone());
                let mut val = serde_json::to_value(&ioi).unwrap_or_default();
                if let Some(w) = warning {
                    val["duplicate_warning"] = json!({
                        "existing_id": w.existing_id,
                        "existing_title": w.existing_title,
                    });
                }
                results.push(val);
            }
            Err(e) => {
                for id in &created_ids {
                    let _ = db.ioi_delete(id);
                }
                return HandlerResult::err_with_data(
                    INVALID_PARAMS,
                    format!("Batch failed at index {i}: {e}"),
                    json!({"index": i}),
                );
            }
        }
    }
    HandlerResult::ok(json!(results))
}

fn handle_ioi_update(db: &Database, params: &Value) -> HandlerResult {
    let id = match param_str_required(params, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let title = param_str(params, "title");
    let description = param_str(params, "description");
    let location = params.get("location").map(|v| v.as_str());
    let severity = params.get("severity").map(|v| v.as_str());
    let tags = params.get("tags").and_then(Value::as_array).map(|arr| {
        arr.iter()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect::<Vec<_>>()
    });

    match db.ioi_update(id, title, description, location, severity, tags.as_deref()) {
        Ok(ioi) => HandlerResult::ok(serde_json::to_value(ioi).unwrap_or_default()),
        Err(e) => db_err_to_handler(e),
    }
}

fn handle_ioi_delete(db: &Database, params: &Value) -> HandlerResult {
    let id = match param_str_required(params, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };
    match db.ioi_delete(id) {
        Ok(()) => HandlerResult::ok(json!({"deleted": true})),
        Err(e) => db_err_to_handler(e),
    }
}

// --- Connections ---

fn handle_connection_create(db: &Database, params: &Value, author: &str) -> HandlerResult {
    let p = match parse_new_connection(params, author) {
        Ok(v) => v,
        Err(e) => return e,
    };
    match db.connection_create(&p) {
        Ok(conn) => HandlerResult::ok(serde_json::to_value(conn).unwrap_or_default()),
        Err(e) => db_err_to_handler(e),
    }
}

fn handle_connection_create_batch(db: &Database, params: &Value, author: &str) -> HandlerResult {
    let Some(connections) = params.get("connections").and_then(Value::as_array) else {
        return HandlerResult::err(INVALID_PARAMS, "Missing required parameter: connections");
    };

    let mut results = Vec::with_capacity(connections.len());
    for (i, conn_params) in connections.iter().enumerate() {
        let Ok(p) = parse_new_connection(conn_params, author) else {
            for r in &results {
                let r: &crate::db::models::Connection = r;
                let _ = db.connection_delete(&r.id);
            }
            return HandlerResult::err_with_data(
                INVALID_PARAMS,
                format!("Connection at index {i} has missing fields"),
                json!({"index": i}),
            );
        };
        match db.connection_create(&p) {
            Ok(created) => results.push(created),
            Err(e) => {
                for r in &results {
                    let _ = db.connection_delete(&r.id);
                }
                return HandlerResult::err_with_data(
                    INVALID_PARAMS,
                    format!("Batch failed at index {i}: {e}"),
                    json!({"index": i}),
                );
            }
        }
    }
    HandlerResult::ok(serde_json::to_value(results).unwrap_or_default())
}

fn parse_new_connection<'a>(
    params: &'a Value,
    author: &'a str,
) -> Result<NewConnection<'a>, HandlerResult> {
    Ok(NewConnection {
        source_id: param_str_required(params, "source_id")?,
        source_type: param_str_required(params, "source_type")?,
        target_id: param_str_required(params, "target_id")?,
        target_type: param_str_required(params, "target_type")?,
        connection_type: param_str_required(params, "connection_type")?,
        description: param_str(params, "description").unwrap_or(""),
        author,
        author_type: "agent",
    })
}

fn handle_connection_list(db: &Database, params: &Value) -> HandlerResult {
    let entity_id = match param_str_required(params, "entity_id") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let connection_type = param_str(params, "connection_type");
    match db.connection_list(entity_id, connection_type) {
        Ok(conns) => HandlerResult::ok(serde_json::to_value(conns).unwrap_or_default()),
        Err(e) => db_err_to_handler(e),
    }
}

fn handle_connection_list_all(db: &Database) -> HandlerResult {
    match db.connection_list_all() {
        Ok(conns) => HandlerResult::ok(serde_json::to_value(conns).unwrap_or_default()),
        Err(e) => db_err_to_handler(e),
    }
}

fn handle_connection_delete(db: &Database, params: &Value) -> HandlerResult {
    let id = match param_str_required(params, "id") {
        Ok(v) => v,
        Err(e) => return e,
    };
    match db.connection_delete(id) {
        Ok(()) => HandlerResult::ok(json!({"deleted": true})),
        Err(e) => db_err_to_handler(e),
    }
}

// --- Search & Filter ---

fn handle_search(db: &Database, params: &Value) -> HandlerResult {
    let query = match param_str_required(params, "query") {
        Ok(v) => v,
        Err(e) => return e,
    };
    let entity_type = param_str(params, "entity_type");
    match db.search(query, entity_type) {
        Ok(results) => HandlerResult::ok(serde_json::to_value(results).unwrap_or_default()),
        Err(e) => db_err_to_handler(e),
    }
}

fn handle_filter(db: &Database, params: &Value) -> HandlerResult {
    let entity_type = match param_str_required(params, "entity_type") {
        Ok(v) => v,
        Err(e) => return e,
    };

    match entity_type {
        "item_of_interest" => {
            let item_id = param_str(params, "item_id");
            let severity = param_str(params, "severity");
            let author_type = param_str(params, "author_type");
            let tags = params.get("tags").and_then(Value::as_array).map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect::<Vec<_>>()
            });
            match db.filter_ioi(item_id, severity, tags.as_deref(), author_type) {
                Ok(results) => HandlerResult::ok(serde_json::to_value(results).unwrap_or_default()),
                Err(e) => db_err_to_handler(e),
            }
        }
        "item" => {
            let item_type = param_str(params, "item_type");
            let status = param_str(params, "analysis_status");
            let tags = params.get("tags").and_then(Value::as_array).map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect::<Vec<_>>()
            });
            match db.item_list(item_type, status, tags.as_deref()) {
                Ok(items) => HandlerResult::ok(serde_json::to_value(items).unwrap_or_default()),
                Err(e) => db_err_to_handler(e),
            }
        }
        "note" => {
            let item_id = param_str(params, "item_id");
            let author_type = param_str(params, "author_type");
            let tags = params.get("tags").and_then(Value::as_array).map(|arr| {
                arr.iter()
                    .filter_map(Value::as_str)
                    .map(String::from)
                    .collect::<Vec<_>>()
            });
            match db.filter_notes(item_id, tags.as_deref(), author_type) {
                Ok(results) => HandlerResult::ok(serde_json::to_value(results).unwrap_or_default()),
                Err(e) => db_err_to_handler(e),
            }
        }
        "connection" => {
            let connection_type = param_str(params, "connection_type");
            let author_type = param_str(params, "author_type");
            match db.filter_connections(connection_type, author_type) {
                Ok(results) => HandlerResult::ok(serde_json::to_value(results).unwrap_or_default()),
                Err(e) => db_err_to_handler(e),
            }
        }
        other => HandlerResult::err(INVALID_PARAMS, format!("Unknown entity_type '{other}'")),
    }
}

// --- Bulk ---

fn handle_bulk_delete(db: &Database, params: &Value) -> HandlerResult {
    let author = param_str(params, "author");
    let since = param_str(params, "since");
    let entity_type = param_str(params, "entity_type");

    match db.bulk_delete(author, since, entity_type) {
        Ok(count) => HandlerResult::ok(json!({"deleted_count": count})),
        Err(e) => db_err_to_handler(e),
    }
}
