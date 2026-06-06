use serde::Serialize;
use serde_json::{json, Value};

use crate::db::error::DbError;
use crate::db::{
    ClaimInput, Database, ExplanationInput, FieldInput, NewConnection, NewEvidence, NewIoi,
    QuestionInput, SearchFilters, StateInput, TransitionInput,
};

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
pub fn dispatch(
    db: &Database,
    tool_name: &str,
    params: &Value,
    author: &str,
    author_type: &str,
) -> HandlerResult {
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
        "note_create" => handle_note_create(db, params, author, author_type),
        "note_create_batch" => handle_note_create_batch(db, params, author, author_type),
        "note_update" => handle_note_update(db, params),
        "note_delete" => {
            let id = param_str_required(params, "id")?;
            db.note_delete(id).map_err(db_err)?;
            Ok(json!({"deleted": true}))
        }

        // IOI
        "ioi_create" => handle_ioi_create(db, params, author, author_type),
        "ioi_create_batch" => handle_ioi_create_batch(db, params, author, author_type),
        "ioi_update" => handle_ioi_update(db, params),
        "ioi_delete" => {
            let id = param_str_required(params, "id")?;
            db.ioi_delete(id).map_err(db_err)?;
            Ok(json!({"deleted": true}))
        }

        // Connections
        "connection_create" => {
            let p = parse_new_connection(params, author, author_type)?;
            serialize(db.connection_create(&p).map_err(db_err)?)
        }
        "connection_create_batch" => {
            handle_connection_create_batch(db, params, author, author_type)
        }
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

        // Explanations
        "explanation_upsert" => handle_explanation_upsert(db, params, author, author_type),
        "explanation_update" => handle_explanation_update(db, params),
        "explanation_delete" => {
            let id = param_str_required(params, "id")?;
            db.explanation_delete(id).map_err(db_err)?;
            Ok(json!({"deleted": true}))
        }
        "claim_create" => handle_claim_create(db, params, author, author_type),
        "claim_update" => handle_claim_update(db, params),
        "claim_delete" => {
            let id = param_str_required(params, "id")?;
            db.claim_delete(id).map_err(db_err)?;
            Ok(json!({"deleted": true}))
        }
        "open_question_create" => handle_question_create(db, params, author, author_type),
        "open_question_update" => handle_question_update(db, params),
        "open_question_delete" => {
            let id = param_str_required(params, "id")?;
            db.open_question_delete(id).map_err(db_err)?;
            Ok(json!({"deleted": true}))
        }
        "evidence_delete" => {
            let id = param_str_required(params, "id")?;
            db.evidence_delete(id).map_err(db_err)?;
            Ok(json!({"deleted": true}))
        }

        // State machine content
        "state_create" => handle_state_create(db, params, author, author_type),
        "state_update" => handle_state_update(db, params),
        "state_delete" => {
            let id = param_str_required(params, "id")?;
            db.state_delete(id).map_err(db_err)?;
            Ok(json!({"deleted": true}))
        }
        "transition_create" => handle_transition_create(db, params, author, author_type),
        "transition_update" => handle_transition_update(db, params),
        "transition_delete" => {
            let id = param_str_required(params, "id")?;
            db.transition_delete(id).map_err(db_err)?;
            Ok(json!({"deleted": true}))
        }
        "field_create" => handle_field_create(db, params, author, author_type),
        "field_update" => handle_field_update(db, params),
        "field_delete" => {
            let id = param_str_required(params, "id")?;
            db.field_delete(id).map_err(db_err)?;
            Ok(json!({"deleted": true}))
        }
        "explanation_get" => {
            let id = param_str_required(params, "id")?;
            serialize(db.explanation_get(id).map_err(db_err)?)
        }
        "explanation_list" => serialize(
            db.explanation_list(
                param_str(params, "explanation_type"),
                param_str(params, "status"),
            )
            .map_err(db_err)?,
        ),
        "evidence_link" => handle_evidence_link(db, params, author, author_type),

        // Search & Filter
        "search" => {
            let query = param_str_required(params, "query")?;
            let tags = param_tags_opt(params, "tags");
            let filters = SearchFilters {
                tags: tags.as_deref(),
                severity: param_str(params, "severity"),
                connection_type: param_str(params, "connection_type"),
                author_type: param_str(params, "author_type"),
            };
            serialize(
                db.search(query, param_str(params, "entity_type"), &filters)
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

    let recent = db.recent_activity(10).map_err(db_err)?;
    let explanations = db.explanation_list(None, None).map_err(db_err)?;
    let open_questions = db.open_questions_list(None, Some("open")).map_err(db_err)?;

    Ok(json!({
        "items": serde_json::to_value(&items).unwrap_or_default(),
        "severity_summary": severity_counts,
        "tags": serde_json::to_value(&tags).unwrap_or_default(),
        "connection_types": serde_json::to_value(&conn_types).unwrap_or_default(),
        "recent_activity": serde_json::to_value(&recent).unwrap_or_default(),
        "explanations": serde_json::to_value(&explanations).unwrap_or_default(),
        "open_questions": serde_json::to_value(&open_questions).unwrap_or_default(),
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

fn handle_note_create(
    db: &Database,
    params: &Value,
    author: &str,
    author_type: &str,
) -> HandlerResult {
    let item_id = param_str_required(params, "item_id")?;
    let title = param_str_required(params, "title")?;
    let content = param_str_required(params, "content")?;
    let tags = param_tags(params, "tags");
    serialize(
        db.note_create(Some(item_id), title, content, author, author_type, &tags)
            .map_err(db_err)?,
    )
}

fn handle_note_create_batch(
    db: &Database,
    params: &Value,
    author: &str,
    author_type: &str,
) -> HandlerResult {
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
                .note_create(Some(item_id), title, content, author, author_type, &tags)
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

fn handle_ioi_create(
    db: &Database,
    params: &Value,
    author: &str,
    author_type: &str,
) -> HandlerResult {
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
            author_type,
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

fn handle_ioi_create_batch(
    db: &Database,
    params: &Value,
    author: &str,
    author_type: &str,
) -> HandlerResult {
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
                    author_type,
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

fn handle_connection_create_batch(
    db: &Database,
    params: &Value,
    author: &str,
    author_type: &str,
) -> HandlerResult {
    let connections = params
        .get("connections")
        .and_then(Value::as_array)
        .ok_or("Missing required parameter: connections")?;

    run_batch(
        connections,
        |item| {
            let p = parse_new_connection(item, author, author_type)?;
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
    author_type: &'a str,
) -> Result<NewConnection<'a>, String> {
    Ok(NewConnection {
        source_id: param_str_required(params, "source_id")?,
        source_type: param_str_required(params, "source_type")?,
        target_id: param_str_required(params, "target_id")?,
        target_type: param_str_required(params, "target_type")?,
        connection_type: param_str_required(params, "connection_type")?,
        description: param_str(params, "description").unwrap_or(""),
        author,
        author_type,
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
        "explanation" => serialize(
            db.explanation_list(
                param_str(params, "explanation_type"),
                param_str(params, "status"),
            )
            .map_err(db_err)?,
        ),
        "open_question" => serialize(
            db.open_questions_list(param_str(params, "priority"), param_str(params, "status"))
                .map_err(db_err)?,
        ),
        other => Err(format!("Unknown entity_type '{other}'")),
    }
}

// --- Explanations ---

fn parse_claims(params: &Value) -> Result<Vec<ClaimInput>, String> {
    let Some(arr) = params.get("claims").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    arr.iter()
        .enumerate()
        .map(|(i, c)| {
            Ok(ClaimInput {
                stable_key: param_str(c, "stable_key")
                    .ok_or_else(|| format!("claims[{i}]: missing 'stable_key'"))?
                    .to_string(),
                text: param_str(c, "text")
                    .ok_or_else(|| format!("claims[{i}]: missing 'text'"))?
                    .to_string(),
                claim_type: param_str(c, "claim_type").map(String::from),
                status: param_str(c, "status").map(String::from),
                confidence: param_str(c, "confidence").map(String::from),
            })
        })
        .collect()
}

fn parse_questions(params: &Value) -> Result<Vec<QuestionInput>, String> {
    let Some(arr) = params.get("open_questions").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    arr.iter()
        .enumerate()
        .map(|(i, q)| {
            Ok(QuestionInput {
                stable_key: param_str(q, "stable_key")
                    .ok_or_else(|| format!("open_questions[{i}]: missing 'stable_key'"))?
                    .to_string(),
                question: param_str(q, "question")
                    .ok_or_else(|| format!("open_questions[{i}]: missing 'question'"))?
                    .to_string(),
                priority: param_str(q, "priority").map(String::from),
                status: param_str(q, "status").map(String::from),
            })
        })
        .collect()
}

fn param_bool(v: &Value, key: &str) -> bool {
    v.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn param_i64(v: &Value, key: &str) -> Option<i64> {
    v.get(key).and_then(Value::as_i64)
}

fn parse_fields(params: &Value) -> Result<Vec<FieldInput>, String> {
    let Some(arr) = params.get("fields").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    arr.iter()
        .enumerate()
        .map(|(i, f)| {
            Ok(FieldInput {
                stable_key: param_str(f, "stable_key")
                    .ok_or_else(|| format!("fields[{i}]: missing 'stable_key'"))?
                    .to_string(),
                name: param_str(f, "name")
                    .ok_or_else(|| format!("fields[{i}]: missing 'name'"))?
                    .to_string(),
                field_type: param_str(f, "field_type").map(String::from),
                offset: param_i64(f, "offset"),
                size: param_i64(f, "size"),
                description: param_str(f, "description").map(String::from),
            })
        })
        .collect()
}

fn parse_states(params: &Value) -> Result<Vec<StateInput>, String> {
    let Some(arr) = params.get("states").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    arr.iter()
        .enumerate()
        .map(|(i, s)| {
            Ok(StateInput {
                stable_key: param_str(s, "stable_key")
                    .ok_or_else(|| format!("states[{i}]: missing 'stable_key'"))?
                    .to_string(),
                name: param_str(s, "name")
                    .ok_or_else(|| format!("states[{i}]: missing 'name'"))?
                    .to_string(),
                description: param_str(s, "description").map(String::from),
                is_initial: param_bool(s, "is_initial"),
                is_terminal: param_bool(s, "is_terminal"),
            })
        })
        .collect()
}

fn parse_transitions(params: &Value) -> Result<Vec<TransitionInput>, String> {
    let Some(arr) = params.get("transitions").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    arr.iter()
        .enumerate()
        .map(|(i, t)| {
            Ok(TransitionInput {
                stable_key: param_str(t, "stable_key")
                    .ok_or_else(|| format!("transitions[{i}]: missing 'stable_key'"))?
                    .to_string(),
                from_state: param_str(t, "from_state")
                    .ok_or_else(|| format!("transitions[{i}]: missing 'from_state'"))?
                    .to_string(),
                to_state: param_str(t, "to_state")
                    .ok_or_else(|| format!("transitions[{i}]: missing 'to_state'"))?
                    .to_string(),
                event: param_str(t, "event").map(String::from),
                guard: param_str(t, "guard").map(String::from),
                action: param_str(t, "action").map(String::from),
                description: param_str(t, "description").map(String::from),
            })
        })
        .collect()
}

fn handle_explanation_upsert(
    db: &Database,
    params: &Value,
    author: &str,
    author_type: &str,
) -> HandlerResult {
    let input = ExplanationInput {
        stable_key: param_str_required(params, "stable_key")?.to_string(),
        title: param_str_required(params, "title")?.to_string(),
        explanation_type: param_str(params, "explanation_type")
            .unwrap_or("custom")
            .to_string(),
        summary: param_str(params, "summary").unwrap_or("").to_string(),
        status: param_str(params, "status").map(String::from),
        confidence: param_str(params, "confidence").map(String::from),
        diagram_html: param_str(params, "diagram_html").map(String::from),
        tags: param_tags(params, "tags"),
        scope_item_ids: param_tags(params, "scope_item_ids"),
        claims: parse_claims(params)?,
        open_questions: parse_questions(params)?,
        states: parse_states(params)?,
        transitions: parse_transitions(params)?,
        fields: parse_fields(params)?,
        author: author.to_string(),
        author_type: author_type.to_string(),
    };
    let res = db.explanation_upsert(&input).map_err(db_err)?;
    Ok(json!({
        "explanation": serde_json::to_value(&res.detail).unwrap_or_default(),
        "warnings": res.warnings,
    }))
}

fn handle_explanation_update(db: &Database, params: &Value) -> HandlerResult {
    let id = param_str_required(params, "id")?;
    serialize(
        db.explanation_update(
            id,
            param_str(params, "title"),
            param_str(params, "explanation_type"),
            param_str(params, "summary"),
            param_str(params, "status"),
            param_str(params, "confidence"),
            param_str(params, "diagram_html"),
        )
        .map_err(db_err)?,
    )
}

fn handle_claim_create(
    db: &Database,
    params: &Value,
    author: &str,
    author_type: &str,
) -> HandlerResult {
    serialize(
        db.claim_create(
            param_str_required(params, "explanation_id")?,
            param_str_required(params, "text")?,
            param_str(params, "claim_type"),
            param_str(params, "status"),
            param_str(params, "confidence"),
            author,
            author_type,
        )
        .map_err(db_err)?,
    )
}

fn handle_claim_update(db: &Database, params: &Value) -> HandlerResult {
    serialize(
        db.claim_update(
            param_str_required(params, "id")?,
            param_str(params, "text"),
            param_str(params, "claim_type"),
            param_str(params, "status"),
            param_str(params, "confidence"),
        )
        .map_err(db_err)?,
    )
}

fn handle_question_create(
    db: &Database,
    params: &Value,
    author: &str,
    author_type: &str,
) -> HandlerResult {
    serialize(
        db.open_question_create(
            param_str_required(params, "explanation_id")?,
            param_str_required(params, "question")?,
            param_str(params, "priority"),
            param_str(params, "status"),
            author,
            author_type,
        )
        .map_err(db_err)?,
    )
}

fn handle_question_update(db: &Database, params: &Value) -> HandlerResult {
    serialize(
        db.open_question_update(
            param_str_required(params, "id")?,
            param_str(params, "question"),
            param_str(params, "priority"),
            param_str(params, "status"),
        )
        .map_err(db_err)?,
    )
}

fn handle_state_create(
    db: &Database,
    params: &Value,
    author: &str,
    author_type: &str,
) -> HandlerResult {
    serialize(
        db.state_create(
            param_str_required(params, "explanation_id")?,
            param_str_required(params, "name")?,
            param_str(params, "description"),
            param_bool(params, "is_initial"),
            param_bool(params, "is_terminal"),
            author,
            author_type,
        )
        .map_err(db_err)?,
    )
}

fn handle_state_update(db: &Database, params: &Value) -> HandlerResult {
    serialize(
        db.state_update(
            param_str_required(params, "id")?,
            param_str(params, "name"),
            param_str(params, "description"),
            params.get("is_initial").and_then(Value::as_bool),
            params.get("is_terminal").and_then(Value::as_bool),
        )
        .map_err(db_err)?,
    )
}

fn handle_transition_create(
    db: &Database,
    params: &Value,
    author: &str,
    author_type: &str,
) -> HandlerResult {
    serialize(
        db.transition_create(
            param_str_required(params, "explanation_id")?,
            param_str_required(params, "from_state")?,
            param_str_required(params, "to_state")?,
            param_str(params, "event"),
            param_str(params, "guard"),
            param_str(params, "action"),
            param_str(params, "description"),
            author,
            author_type,
        )
        .map_err(db_err)?,
    )
}

fn handle_transition_update(db: &Database, params: &Value) -> HandlerResult {
    serialize(
        db.transition_update(
            param_str_required(params, "id")?,
            param_str(params, "from_state"),
            param_str(params, "to_state"),
            param_str(params, "event"),
            param_str(params, "guard"),
            param_str(params, "action"),
            param_str(params, "description"),
        )
        .map_err(db_err)?,
    )
}

fn handle_field_create(
    db: &Database,
    params: &Value,
    author: &str,
    author_type: &str,
) -> HandlerResult {
    serialize(
        db.field_create(
            param_str_required(params, "explanation_id")?,
            param_str_required(params, "name")?,
            param_str(params, "field_type"),
            param_i64(params, "offset"),
            param_i64(params, "size"),
            param_str(params, "description"),
            author,
            author_type,
        )
        .map_err(db_err)?,
    )
}

fn handle_field_update(db: &Database, params: &Value) -> HandlerResult {
    serialize(
        db.field_update(
            param_str_required(params, "id")?,
            param_str(params, "name"),
            param_str(params, "field_type"),
            param_i64(params, "offset"),
            param_i64(params, "size"),
            param_str(params, "description"),
        )
        .map_err(db_err)?,
    )
}

fn handle_evidence_link(
    db: &Database,
    params: &Value,
    author: &str,
    author_type: &str,
) -> HandlerResult {
    let target_type = param_str_required(params, "target_type")?;
    let target_id = param_str_required(params, "target_id")?;
    let evidence = NewEvidence {
        target_type,
        target_id,
        source_entity_type: param_str(params, "source_entity_type"),
        source_entity_id: param_str(params, "source_entity_id"),
        external_locator: param_str(params, "external_locator"),
        external_kind: param_str(params, "external_kind"),
        evidence_type: param_str(params, "evidence_type").unwrap_or("agent_inference"),
        strength: param_str(params, "strength").unwrap_or("moderate"),
        excerpt: param_str(params, "excerpt"),
        author,
        author_type,
    };
    serialize(db.evidence_link(&evidence).map_err(db_err)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn db() -> Database {
        Database::in_memory("t").unwrap()
    }

    fn call(db: &Database, tool: &str, args: &Value, who: &str, kind: &str) -> Value {
        dispatch(db, tool, args, who, kind).expect("dispatch ok")
    }

    #[test]
    fn author_type_is_caller_controlled_not_hardcoded() {
        let db = db();
        let item = call(
            &db,
            "item_create",
            &json!({"name": "httpd", "item_type": "elf"}),
            "alice",
            "human",
        );
        let item_id = item["id"].as_str().unwrap().to_string();

        // Same tool, two callers → two author_types. This is the parity guarantee:
        // a human write and an agent write share one code path.
        let human = call(
            &db,
            "note_create",
            &json!({"item_id": item_id, "title": "t", "content": "c"}),
            "alice",
            "human",
        );
        assert_eq!(human["author_type"], "human");
        assert_eq!(human["author"], "alice");

        let agent = call(
            &db,
            "note_create",
            &json!({"item_id": item_id, "title": "t2", "content": "c"}),
            "claude",
            "agent",
        );
        assert_eq!(agent["author_type"], "agent");
    }

    #[test]
    fn granular_child_tools_give_full_crud_via_dispatch() {
        let db = db();
        let expl = call(
            &db,
            "explanation_upsert",
            &json!({"stable_key": "e.1", "title": "E"}),
            "alice",
            "human",
        );
        let expl_id = expl["explanation"]["id"].as_str().unwrap().to_string();

        // claim: create (human-stamped) → update → delete
        let claim = call(
            &db,
            "claim_create",
            &json!({"explanation_id": expl_id, "text": "C"}),
            "alice",
            "human",
        );
        assert_eq!(claim["author_type"], "human");
        let cid = claim["id"].as_str().unwrap().to_string();
        let upd = call(
            &db,
            "claim_update",
            &json!({"id": cid, "status": "supported"}),
            "alice",
            "human",
        );
        assert_eq!(upd["status"], "supported");
        call(&db, "claim_delete", &json!({"id": cid}), "alice", "human");

        // open question: create → delete
        let q = call(
            &db,
            "open_question_create",
            &json!({"explanation_id": expl_id, "question": "Q?"}),
            "alice",
            "human",
        );
        let qid = q["id"].as_str().unwrap().to_string();
        call(
            &db,
            "open_question_delete",
            &json!({"id": qid}),
            "alice",
            "human",
        );

        // evidence: create → delete
        let claim2 = call(
            &db,
            "claim_create",
            &json!({"explanation_id": expl_id, "text": "C2"}),
            "alice",
            "human",
        );
        let c2 = claim2["id"].as_str().unwrap().to_string();
        let ev = call(
            &db,
            "evidence_link",
            &json!({"target_type": "claim", "target_id": c2, "external_locator": "FUN_x", "external_kind": "ghidra"}),
            "alice",
            "human",
        );
        assert_eq!(ev["author_type"], "human");
        let evid = ev["id"].as_str().unwrap().to_string();
        call(
            &db,
            "evidence_delete",
            &json!({"id": evid}),
            "alice",
            "human",
        );

        // explanation: update envelope → delete
        let eu = call(
            &db,
            "explanation_update",
            &json!({"id": expl_id, "status": "reviewed"}),
            "alice",
            "human",
        );
        assert_eq!(eu["status"], "reviewed");
        call(
            &db,
            "explanation_delete",
            &json!({"id": expl_id}),
            "alice",
            "human",
        );
        assert!(db.explanation_list(None, None).unwrap().is_empty());
    }

    #[test]
    fn state_machine_tools_and_generated_text() {
        let db = db();
        let expl = call(
            &db,
            "explanation_upsert",
            &json!({"stable_key": "sm.1", "title": "Auth", "explanation_type": "state_machine"}),
            "alice",
            "human",
        );
        let expl_id = expl["explanation"]["id"].as_str().unwrap().to_string();

        for (name, init) in [("UNAUTH", true), ("AUTHED", false)] {
            call(
                &db,
                "state_create",
                &json!({"explanation_id": expl_id, "name": name, "is_initial": init}),
                "alice",
                "human",
            );
        }
        let detail = call(
            &db,
            "explanation_get",
            &json!({"id": expl_id}),
            "alice",
            "human",
        );
        let states = detail["states"].as_array().unwrap().clone();
        let from = states[0]["stable_key"].as_str().unwrap().to_string();
        let to = states[1]["stable_key"].as_str().unwrap().to_string();

        let t = call(
            &db,
            "transition_create",
            &json!({"explanation_id": expl_id, "from_state": from, "to_state": to, "event": "LOGIN", "guard": "ok"}),
            "alice",
            "human",
        );
        assert_eq!(t["author_type"], "human");

        // A transition to an unknown state is rejected.
        assert!(dispatch(
            &db,
            "transition_create",
            &json!({"explanation_id": expl_id, "from_state": from, "to_state": "nope"}),
            "alice",
            "human",
        )
        .is_err());

        // explanation_get carries the on-the-fly text diagram.
        let detail = call(
            &db,
            "explanation_get",
            &json!({"id": expl_id}),
            "alice",
            "human",
        );
        let text = detail["diagram_text"].as_str().unwrap();
        assert!(text.contains("UNAUTH --LOGIN [ok]--> AUTHED"));

        // Deleting a state prunes its transitions.
        let state_id = states[0]["id"].as_str().unwrap().to_string();
        call(
            &db,
            "state_delete",
            &json!({"id": state_id}),
            "alice",
            "human",
        );
        let detail = call(
            &db,
            "explanation_get",
            &json!({"id": expl_id}),
            "alice",
            "human",
        );
        assert!(detail["transitions"].as_array().unwrap().is_empty());
    }
}
