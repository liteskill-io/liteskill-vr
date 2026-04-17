use serde::Serialize;
use serde_json::{json, Value};

use crate::db::error::DbError;
use crate::db::{Database, NewConnection, NewIoi};

pub type HandlerResult = Result<Value, String>;

fn param_str<'a>(params: &'a Value, key: &str) -> Option<&'a str> {
    params.get(key).and_then(Value::as_str)
}

fn param_str_required<'a>(params: &'a Value, key: &str) -> Result<&'a str, String> {
    param_str(params, key).ok_or_else(|| format!("Missing required parameter: {key}"))
}

// Three-way semantics for nullable fields on update:
// None -> field absent (don't change), Some(None) -> set NULL, Some(Some(s)) -> set value.
#[allow(clippy::option_option)]
fn param_str_opt_opt<'a>(params: &'a Value, key: &str) -> Option<Option<&'a str>> {
    params.get(key).map(Value::as_str)
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

fn param_tags_opt(params: &Value, key: &str) -> Option<Vec<String>> {
    params.get(key).and_then(Value::as_array).map(|arr| {
        arr.iter()
            .filter_map(Value::as_str)
            .map(String::from)
            .collect()
    })
}

fn serialize<T: Serialize>(v: T) -> HandlerResult {
    serde_json::to_value(v).map_err(|e| format!("Serialization error: {e}"))
}

fn db_err(e: DbError) -> String {
    match e {
        DbError::NotFound { entity, id } => format!("{entity} '{id}' not found"),
        DbError::DuplicateName { entity, name } => format!("{entity} '{name}' already exists"),
        DbError::UnregisteredTag(name) => format!(
            "Tag '{name}' is not registered. Call tag_list() to see registered tags, or tag_create() to register a new one."
        ),
        DbError::UnregisteredConnectionType(name) => format!(
            "Connection type '{name}' is not registered. Call connection_type_list() to see registered types, or connection_type_create() to register a new one."
        ),
        DbError::InvalidReference { entity, id } => format!("{entity} '{id}' does not exist"),
        DbError::BulkDeleteNoFilter => {
            "bulk_delete requires at least one filter (author, since, or entity_type)".to_string()
        }
        DbError::Sqlite(e) => format!("Database error: {e}"),
    }
}

/// Run a batch operation with all-or-nothing semantics: if any item fails,
/// delete previously-created items and return the error.
fn run_batch<T, C, D>(items: &[Value], mut create: C, delete: D) -> HandlerResult
where
    T: Serialize,
    C: FnMut(&Value) -> Result<(String, T), String>,
    D: Fn(&str),
{
    let mut results: Vec<T> = Vec::with_capacity(items.len());
    let mut created_ids: Vec<String> = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        match create(item) {
            Ok((id, value)) => {
                created_ids.push(id);
                results.push(value);
            }
            Err(msg) => {
                for id in &created_ids {
                    delete(id);
                }
                return Err(format!("Batch failed at index {i}: {msg}"));
            }
        }
    }
    serialize(results)
}

#[allow(clippy::too_many_lines)]
pub fn dispatch(db: &Database, tool_name: &str, params: &Value, author: &str) -> HandlerResult {
    match tool_name {
        // Project
        "project_get" => serialize(db.project_get().map_err(db_err)?),
        "project_summary" => handle_project_summary(db),
        "changes_since" => handle_changes_since(db, params),

        // Tags
        "tag_list" => serialize(db.tag_list().map_err(db_err)?),
        "tag_create" => {
            let name = param_str_required(params, "name")?;
            let description = param_str(params, "description").unwrap_or("");
            let color = param_str(params, "color");
            serialize(db.tag_create(name, description, color).map_err(db_err)?)
        }
        "tag_delete" => {
            let id = param_str_required(params, "id")?;
            db.tag_delete(id).map_err(db_err)?;
            Ok(json!({"deleted": true}))
        }

        // Connection Types
        "connection_type_list" => serialize(db.connection_type_list().map_err(db_err)?),
        "connection_type_create" => {
            let name = param_str_required(params, "name")?;
            let description = param_str(params, "description").unwrap_or("");
            serialize(
                db.connection_type_create(name, description)
                    .map_err(db_err)?,
            )
        }
        "connection_type_delete" => {
            let id = param_str_required(params, "id")?;
            db.connection_type_delete(id).map_err(db_err)?;
            Ok(json!({"deleted": true}))
        }

        // Items
        "item_list" => {
            let tags = param_tags_opt(params, "tags");
            serialize(
                db.item_list(
                    param_str(params, "item_type"),
                    param_str(params, "analysis_status"),
                    tags.as_deref(),
                )
                .map_err(db_err)?,
            )
        }
        "item_get" => {
            let id = param_str_required(params, "id")?;
            serialize(db.item_get(id).map_err(db_err)?)
        }
        "item_create" => handle_item_create(db, params),
        "item_create_batch" => handle_item_create_batch(db, params),
        "item_update" => handle_item_update(db, params),
        "item_delete" => {
            let id = param_str_required(params, "id")?;
            db.item_delete(id).map_err(db_err)?;
            Ok(json!({"deleted": true}))
        }

        // Notes
        "note_create" => handle_note_create(db, params, author),
        "note_create_batch" => handle_note_create_batch(db, params, author),
        "note_update" => handle_note_update(db, params),
        "note_delete" => {
            let id = param_str_required(params, "id")?;
            db.note_delete(id).map_err(db_err)?;
            Ok(json!({"deleted": true}))
        }

        // IOI
        "ioi_create" => handle_ioi_create(db, params, author),
        "ioi_create_batch" => handle_ioi_create_batch(db, params, author),
        "ioi_update" => handle_ioi_update(db, params),
        "ioi_delete" => {
            let id = param_str_required(params, "id")?;
            db.ioi_delete(id).map_err(db_err)?;
            Ok(json!({"deleted": true}))
        }

        // Connections
        "connection_create" => {
            let p = parse_new_connection(params, author)?;
            serialize(db.connection_create(&p).map_err(db_err)?)
        }
        "connection_create_batch" => handle_connection_create_batch(db, params, author),
        "connection_list" => {
            let entity_id = param_str_required(params, "entity_id")?;
            serialize(
                db.connection_list(entity_id, param_str(params, "connection_type"))
                    .map_err(db_err)?,
            )
        }
        "connection_list_all" => serialize(db.connection_list_all().map_err(db_err)?),
        "connection_delete" => {
            let id = param_str_required(params, "id")?;
            db.connection_delete(id).map_err(db_err)?;
            Ok(json!({"deleted": true}))
        }

        // Search & Filter
        "search" => {
            let query = param_str_required(params, "query")?;
            serialize(
                db.search(query, param_str(params, "entity_type"))
                    .map_err(db_err)?,
            )
        }
        "filter" => handle_filter(db, params),

        // Bulk
        "bulk_delete" => {
            let count = db
                .bulk_delete(
                    param_str(params, "author"),
                    param_str(params, "since"),
                    param_str(params, "entity_type"),
                )
                .map_err(db_err)?;
            Ok(json!({"deleted_count": count}))
        }

        _ => Err(format!("Unknown tool: {tool_name}")),
    }
}

// --- Project ---

fn handle_project_summary(db: &Database) -> HandlerResult {
    let items = db.item_list(None, None, None).map_err(db_err)?;
    let tags = db.tag_list().map_err(db_err)?;
    let conn_types = db.connection_type_list().map_err(db_err)?;

    let mut severity_counts = json!({"critical": 0, "high": 0, "medium": 0, "low": 0, "info": 0});
    for item in &items {
        let iois = db
            .filter_ioi(Some(&item.item.item.id), None, None, None)
            .map_err(db_err)?;
        for ioi in &iois {
            if let Some(ref sev) = ioi.ioi.severity {
                if let Some(count) = severity_counts.get_mut(sev) {
                    *count = json!(count.as_i64().unwrap_or(0) + 1);
                }
            }
        }
    }

    Ok(json!({
        "items": serde_json::to_value(&items).unwrap_or_default(),
        "severity_summary": severity_counts,
        "tags": serde_json::to_value(&tags).unwrap_or_default(),
        "connection_types": serde_json::to_value(&conn_types).unwrap_or_default(),
    }))
}

fn handle_changes_since(db: &Database, params: &Value) -> HandlerResult {
    let since = param_str_required(params, "since")?;
    serialize(db.changes_since(since).map_err(db_err)?)
}

// --- Items ---

fn handle_item_create(db: &Database, params: &Value) -> HandlerResult {
    let name = param_str_required(params, "name")?;
    let item_type = param_str_required(params, "item_type")?;
    let tags = param_tags(params, "tags");
    serialize(
        db.item_create(
            name,
            item_type,
            param_str(params, "path"),
            param_str(params, "architecture"),
            param_str(params, "description").unwrap_or(""),
            &tags,
        )
        .map_err(db_err)?,
    )
}

fn handle_item_create_batch(db: &Database, params: &Value) -> HandlerResult {
    let items = params
        .get("items")
        .and_then(Value::as_array)
        .ok_or("Missing required parameter: items")?;

    run_batch(
        items,
        |item| {
            let name = param_str(item, "name").ok_or_else(|| "missing 'name'".to_string())?;
            let item_type =
                param_str(item, "item_type").ok_or_else(|| "missing 'item_type'".to_string())?;
            let tags = param_tags(item, "tags");
            let created = db
                .item_create(
                    name,
                    item_type,
                    param_str(item, "path"),
                    param_str(item, "architecture"),
                    param_str(item, "description").unwrap_or(""),
                    &tags,
                )
                .map_err(db_err)?;
            Ok((created.item.id.clone(), created))
        },
        |id| {
            let _ = db.item_delete(id);
        },
    )
}

fn handle_item_update(db: &Database, params: &Value) -> HandlerResult {
    let id = param_str_required(params, "id")?;
    let tags = param_tags_opt(params, "tags");
    serialize(
        db.item_update(
            id,
            param_str(params, "name"),
            param_str(params, "description"),
            param_str(params, "analysis_status"),
            tags.as_deref(),
        )
        .map_err(db_err)?,
    )
}

// --- Notes ---

fn handle_note_create(db: &Database, params: &Value, author: &str) -> HandlerResult {
    let item_id = param_str_required(params, "item_id")?;
    let title = param_str_required(params, "title")?;
    let content = param_str_required(params, "content")?;
    let tags = param_tags(params, "tags");
    serialize(
        db.note_create(Some(item_id), title, content, author, "agent", &tags)
            .map_err(db_err)?,
    )
}

fn handle_note_create_batch(db: &Database, params: &Value, author: &str) -> HandlerResult {
    let notes = params
        .get("notes")
        .and_then(Value::as_array)
        .ok_or("Missing required parameter: notes")?;

    run_batch(
        notes,
        |note| {
            let item_id =
                param_str(note, "item_id").ok_or_else(|| "missing 'item_id'".to_string())?;
            let title = param_str(note, "title").ok_or_else(|| "missing 'title'".to_string())?;
            let content =
                param_str(note, "content").ok_or_else(|| "missing 'content'".to_string())?;
            let tags = param_tags(note, "tags");
            let created = db
                .note_create(Some(item_id), title, content, author, "agent", &tags)
                .map_err(db_err)?;
            Ok((created.note.id.clone(), created))
        },
        |id| {
            let _ = db.note_delete(id);
        },
    )
}

fn handle_note_update(db: &Database, params: &Value) -> HandlerResult {
    let id = param_str_required(params, "id")?;
    let tags = param_tags_opt(params, "tags");
    serialize(
        db.note_update(
            id,
            param_str(params, "title"),
            param_str(params, "content"),
            tags.as_deref(),
        )
        .map_err(db_err)?,
    )
}

// --- IOI ---

fn handle_ioi_create(db: &Database, params: &Value, author: &str) -> HandlerResult {
    let item_id = param_str_required(params, "item_id")?;
    let title = param_str_required(params, "title")?;
    let description = param_str_required(params, "description")?;
    let tags = param_tags(params, "tags");
    let (ioi, warning) = db
        .ioi_create(&NewIoi {
            item_id,
            title,
            description,
            location: param_str(params, "location"),
            severity: param_str(params, "severity"),
            status: param_str(params, "status"),
            author,
            author_type: "agent",
            tags: &tags,
        })
        .map_err(db_err)?;

    let mut result = serialize(ioi)?;
    if let Some(w) = warning {
        result["duplicate_warning"] = json!({
            "existing_id": w.existing_id,
            "existing_title": w.existing_title,
        });
    }
    Ok(result)
}

fn handle_ioi_create_batch(db: &Database, params: &Value, author: &str) -> HandlerResult {
    let item_id = param_str_required(params, "item_id")?;
    let items = params
        .get("items")
        .and_then(Value::as_array)
        .ok_or("Missing required parameter: items")?;

    run_batch(
        items,
        |item| {
            let title = param_str(item, "title").ok_or_else(|| "missing 'title'".to_string())?;
            let description = param_str(item, "description")
                .ok_or_else(|| "missing 'description'".to_string())?;
            let tags = param_tags(item, "tags");
            let (ioi, warning) = db
                .ioi_create(&NewIoi {
                    item_id,
                    title,
                    description,
                    location: param_str(item, "location"),
                    severity: param_str(item, "severity"),
                    status: param_str(item, "status"),
                    author,
                    author_type: "agent",
                    tags: &tags,
                })
                .map_err(db_err)?;
            let id = ioi.ioi.id.clone();
            let mut val = serde_json::to_value(&ioi).unwrap_or_default();
            if let Some(w) = warning {
                val["duplicate_warning"] = json!({
                    "existing_id": w.existing_id,
                    "existing_title": w.existing_title,
                });
            }
            Ok((id, val))
        },
        |id| {
            let _ = db.ioi_delete(id);
        },
    )
}

fn handle_ioi_update(db: &Database, params: &Value) -> HandlerResult {
    let id = param_str_required(params, "id")?;
    let tags = param_tags_opt(params, "tags");
    serialize(
        db.ioi_update(
            id,
            param_str(params, "title"),
            param_str(params, "description"),
            param_str_opt_opt(params, "location"),
            param_str_opt_opt(params, "severity"),
            param_str(params, "status"),
            tags.as_deref(),
        )
        .map_err(db_err)?,
    )
}

// --- Connections ---

fn handle_connection_create_batch(db: &Database, params: &Value, author: &str) -> HandlerResult {
    let connections = params
        .get("connections")
        .and_then(Value::as_array)
        .ok_or("Missing required parameter: connections")?;

    run_batch(
        connections,
        |item| {
            let p = parse_new_connection(item, author)?;
            let created = db.connection_create(&p).map_err(db_err)?;
            Ok((created.id.clone(), created))
        },
        |id| {
            let _ = db.connection_delete(id);
        },
    )
}

fn parse_new_connection<'a>(
    params: &'a Value,
    author: &'a str,
) -> Result<NewConnection<'a>, String> {
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

// --- Filter ---

fn handle_filter(db: &Database, params: &Value) -> HandlerResult {
    let entity_type = param_str_required(params, "entity_type")?;
    let tags = param_tags_opt(params, "tags");
    let author_type = param_str(params, "author_type");

    match entity_type {
        "item_of_interest" => serialize(
            db.filter_ioi(
                param_str(params, "item_id"),
                param_str(params, "severity"),
                tags.as_deref(),
                author_type,
            )
            .map_err(db_err)?,
        ),
        "item" => serialize(
            db.item_list(
                param_str(params, "item_type"),
                param_str(params, "analysis_status"),
                tags.as_deref(),
            )
            .map_err(db_err)?,
        ),
        "note" => serialize(
            db.filter_notes(param_str(params, "item_id"), tags.as_deref(), author_type)
                .map_err(db_err)?,
        ),
        "connection" => serialize(
            db.filter_connections(param_str(params, "connection_type"), author_type)
                .map_err(db_err)?,
        ),
        other => Err(format!("Unknown entity_type '{other}'")),
    }
}
