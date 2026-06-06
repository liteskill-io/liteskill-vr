use serde_json::{json, Value};

fn tool(name: &str, description: &str, properties: &Value, required: &[&str]) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": {
            "type": "object",
            "properties": properties,
            "required": required,
        }
    })
}

fn tool_no_params(name: &str, description: &str) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": { "type": "object", "properties": {} }
    })
}

pub fn list_all() -> Vec<Value> {
    let mut tools = Vec::new();
    tools.extend(project_tools());
    tools.extend(tag_tools());
    tools.extend(connection_type_tools());
    tools.extend(item_tools());
    tools.extend(note_tools());
    tools.extend(ioi_tools());
    tools.extend(connection_tools());
    tools.extend(explanation_tools());
    tools.extend(search_tools());
    tools.extend(bulk_tools());
    tools
}

fn explanation_tools() -> Vec<Value> {
    let claim_item = json!({
        "type": "object",
        "properties": {
            "stable_key": {"type": "string", "description": "Stable id for idempotent re-runs, e.g. claim.auth.uses_rsa"},
            "text": {"type": "string"},
            "claim_type": {"type": "string", "enum": ["behavior", "invariant", "constraint", "assumption", "hypothesis", "security_relevant", "finding_context", "unknown"]},
            "status": {"type": "string", "enum": ["hypothesis", "supported", "refuted"]},
            "confidence": {"type": "string", "enum": ["low", "medium", "high"]}
        },
        "required": ["stable_key", "text"]
    });
    let question_item = json!({
        "type": "object",
        "properties": {
            "stable_key": {"type": "string"},
            "question": {"type": "string"},
            "priority": {"type": "string", "enum": ["low", "medium", "high"]},
            "status": {"type": "string", "enum": ["open", "answered", "blocked", "superseded"]}
        },
        "required": ["stable_key", "question"]
    });
    vec![
        tool(
            "explanation_upsert",
            "Create or update an Explanation (how a system works) by stable_key, with its claims and open questions, all-or-nothing. Keep `summary` a short TL;DR; put the substance in claims (each evidence-backed via evidence_link) and record unknowns as open_questions. Re-running with the same stable_keys updates in place. Returns the explanation plus advisory warnings.",
            &json!({
                "stable_key": {"type": "string", "description": "Stable id, unique per project, e.g. explanation.auth_flow"},
                "title": {"type": "string"},
                "explanation_type": {"type": "string", "enum": ["architecture", "protocol", "packet_format", "state_machine", "control_flow", "data_flow", "memory_layout", "object_lifecycle", "api_surface", "threat_model", "custom"]},
                "summary": {"type": "string", "description": "Short TL;DR — NOT a wall of prose; use claims for substance"},
                "status": {"type": "string", "enum": ["draft", "reviewed"]},
                "confidence": {"type": "string", "enum": ["low", "medium", "high"]},
                "tags": {"type": "array", "items": {"type": "string"}, "description": "Registered tag names"},
                "scope_item_ids": {"type": "array", "items": {"type": "string"}, "description": "Item ids this explanation covers (linked via 'explains' connections)"},
                "claims": {"type": "array", "items": claim_item},
                "open_questions": {"type": "array", "items": question_item}
            }),
            &["stable_key", "title"],
        ),
        tool(
            "explanation_get",
            "Get one explanation with its claims, open questions, evidence, and scope.",
            &json!({"id": {"type": "string"}}),
            &["id"],
        ),
        tool(
            "explanation_list",
            "List explanations with child counts. Optional filters by type and status.",
            &json!({
                "explanation_type": {"type": "string"},
                "status": {"type": "string", "enum": ["draft", "reviewed"]}
            }),
            &[],
        ),
        tool(
            "evidence_link",
            "Attach evidence to an explanation, a claim, or a finding. Source is EITHER an existing entity (source_entity_type + source_entity_id) OR a free-text external_locator (+external_kind) such as a Ghidra symbol, address, or pcap packet.",
            &json!({
                "target_type": {"type": "string", "enum": ["explanation", "claim", "finding"]},
                "target_id": {"type": "string"},
                "source_entity_type": {"type": "string", "enum": ["item", "item_of_interest", "note", "connection", "explanation"]},
                "source_entity_id": {"type": "string"},
                "external_locator": {"type": "string", "description": "e.g. FUN_00401000+0x14, pcap:42"},
                "external_kind": {"type": "string", "enum": ["ghidra", "address", "pcap", "decompilation", "disassembly", "log", "test_case", "other"]},
                "evidence_type": {"type": "string", "enum": ["static_analysis", "dynamic_trace", "decompilation", "disassembly", "packet_capture", "test_case", "runtime_log", "human_observation", "agent_inference"]},
                "strength": {"type": "string", "enum": ["weak", "moderate", "strong"]},
                "excerpt": {"type": "string"}
            }),
            &["target_type", "target_id"],
        ),
    ]
}

fn project_tools() -> Vec<Value> {
    vec![
        tool_no_params("project_get", "Get project metadata"),
        tool_no_params("project_summary", "High-level overview: all items with status/counts, severity breakdown, recent activity, registered tags, registered connection types"),
        tool("changes_since", "All entities created or updated after the given timestamp, grouped by type",
            &json!({"since": {"type": "string", "description": "ISO 8601 timestamp"}}),
            &["since"]),
    ]
}

fn tag_tools() -> Vec<Value> {
    vec![
        tool_no_params(
            "tag_list",
            "List all registered tags. Call before tagging anything.",
        ),
        tool(
            "tag_create",
            "Register a new tag. Fails if name already exists.",
            &json!({
                "name": {"type": "string"},
                "description": {"type": "string"},
                "color": {"type": "string", "description": "Hex color for UI display"}
            }),
            &["name", "description"],
        ),
        tool(
            "tag_delete",
            "Delete a registered tag. Removes it from all entities.",
            &json!({"id": {"type": "string"}}),
            &["id"],
        ),
    ]
}

fn connection_type_tools() -> Vec<Value> {
    vec![
        tool_no_params(
            "connection_type_list",
            "List all registered connection types. Call before creating connections.",
        ),
        tool(
            "connection_type_create",
            "Register a new connection type. Fails if name already exists.",
            &json!({
                "name": {"type": "string"},
                "description": {"type": "string"}
            }),
            &["name", "description"],
        ),
        tool(
            "connection_type_delete",
            "Delete a registered connection type. Removes all connections of that type.",
            &json!({"id": {"type": "string"}}),
            &["id"],
        ),
    ]
}

fn item_tools() -> Vec<Value> {
    vec![
        tool(
            "item_list",
            "List items with note/ioi/connection counts. All filters optional.",
            &json!({
                "item_type": {"type": "string"},
                "analysis_status": {"type": "string", "enum": ["untouched", "in_progress", "reviewed"]},
                "tags": {"type": "array", "items": {"type": "string"}}
            }),
            &[],
        ),
        tool(
            "item_get",
            "Get full item details including all notes, items of interest, and connections.",
            &json!({"id": {"type": "string"}}),
            &["id"],
        ),
        tool(
            "item_create",
            "Add a new item. Tags must be registered.",
            &json!({
                "name": {"type": "string"},
                "item_type": {"type": "string"},
                "path": {"type": "string"},
                "architecture": {"type": "string"},
                "description": {"type": "string"},
                "tags": {"type": "array", "items": {"type": "string"}}
            }),
            &["name", "item_type"],
        ),
        tool(
            "item_create_batch",
            "Create multiple items. All-or-nothing transaction.",
            &json!({
                "items": {"type": "array", "items": {"type": "object", "properties": {
                    "name": {"type": "string"},
                    "item_type": {"type": "string"},
                    "path": {"type": "string"},
                    "architecture": {"type": "string"},
                    "description": {"type": "string"},
                    "tags": {"type": "array", "items": {"type": "string"}}
                }, "required": ["name", "item_type"]}}
            }),
            &["items"],
        ),
        tool(
            "item_update",
            "Update item metadata.",
            &json!({
                "id": {"type": "string"},
                "name": {"type": "string"},
                "description": {"type": "string"},
                "analysis_status": {"type": "string", "enum": ["untouched", "in_progress", "reviewed"]},
                "tags": {"type": "array", "items": {"type": "string"}}
            }),
            &["id"],
        ),
        tool(
            "item_delete",
            "Delete an item. Cascades to its notes, ioi, and connections.",
            &json!({"id": {"type": "string"}}),
            &["id"],
        ),
    ]
}

fn note_tools() -> Vec<Value> {
    vec![
        tool(
            "note_create",
            "Add a note to an item.",
            &json!({
                "item_id": {"type": "string"},
                "title": {"type": "string"},
                "content": {"type": "string"},
                "tags": {"type": "array", "items": {"type": "string"}}
            }),
            &["item_id", "title", "content"],
        ),
        tool(
            "note_create_batch",
            "Create multiple notes. All-or-nothing. Notes can span multiple items.",
            &json!({
                "notes": {"type": "array", "items": {"type": "object", "properties": {
                    "item_id": {"type": "string"},
                    "title": {"type": "string"},
                    "content": {"type": "string"},
                    "tags": {"type": "array", "items": {"type": "string"}}
                }, "required": ["item_id", "title", "content"]}}
            }),
            &["notes"],
        ),
        tool(
            "note_update",
            "Update a note.",
            &json!({
                "id": {"type": "string"},
                "title": {"type": "string"},
                "content": {"type": "string"},
                "tags": {"type": "array", "items": {"type": "string"}}
            }),
            &["id"],
        ),
        tool(
            "note_delete",
            "Delete a note.",
            &json!({"id": {"type": "string"}}),
            &["id"],
        ),
    ]
}

fn ioi_tools() -> Vec<Value> {
    vec![
        tool("ioi_create", "Add an item of interest. Returns duplicate_warning if similar title/location exists.",
            &json!({
                "item_id": {"type": "string"},
                "title": {"type": "string"},
                "description": {"type": "string"},
                "location": {"type": "string"},
                "severity": {"type": "string", "enum": ["critical", "high", "medium", "low", "info"]},
                "status": {"type": "string", "enum": ["draft", "confirmed", "false_positive", "reported", "fixed"]},
                "tags": {"type": "array", "items": {"type": "string"}}
            }),
            &["item_id", "title", "description"]),
        tool("ioi_create_batch", "Create multiple items of interest. All-or-nothing. Each entry includes duplicate_warning if applicable.",
            &json!({
                "item_id": {"type": "string"},
                "items": {"type": "array", "items": {"type": "object", "properties": {
                    "title": {"type": "string"},
                    "description": {"type": "string"},
                    "location": {"type": "string"},
                    "severity": {"type": "string", "enum": ["critical", "high", "medium", "low", "info"]},
                    "tags": {"type": "array", "items": {"type": "string"}}
                }, "required": ["title", "description"]}}
            }),
            &["item_id", "items"]),
        tool("ioi_update", "Update an item of interest.",
            &json!({
                "id": {"type": "string"},
                "title": {"type": "string"},
                "description": {"type": "string"},
                "location": {"type": "string"},
                "severity": {"type": "string", "enum": ["critical", "high", "medium", "low", "info"]},
                "status": {"type": "string", "enum": ["draft", "confirmed", "false_positive", "reported", "fixed"]},
                "tags": {"type": "array", "items": {"type": "string"}}
            }),
            &["id"]),
        tool("ioi_delete", "Delete an item of interest. Cascades to its connections.",
            &json!({"id": {"type": "string"}}),
            &["id"]),
    ]
}

fn connection_tools() -> Vec<Value> {
    vec![
        tool(
            "connection_create",
            "Create a connection between two entities. connection_type must be registered.",
            &json!({
                "source_id": {"type": "string"},
                "source_type": {"type": "string", "enum": ["item", "item_of_interest"]},
                "target_id": {"type": "string"},
                "target_type": {"type": "string", "enum": ["item", "item_of_interest"]},
                "connection_type": {"type": "string"},
                "description": {"type": "string"}
            }),
            &[
                "source_id",
                "source_type",
                "target_id",
                "target_type",
                "connection_type",
                "description",
            ],
        ),
        tool(
            "connection_create_batch",
            "Create multiple connections. All-or-nothing.",
            &json!({
                "connections": {"type": "array", "items": {"type": "object", "properties": {
                    "source_id": {"type": "string"},
                    "source_type": {"type": "string", "enum": ["item", "item_of_interest"]},
                    "target_id": {"type": "string"},
                    "target_type": {"type": "string", "enum": ["item", "item_of_interest"]},
                    "connection_type": {"type": "string"},
                    "description": {"type": "string"}
                }, "required": ["source_id", "source_type", "target_id", "target_type", "connection_type", "description"]}}
            }),
            &["connections"],
        ),
        tool(
            "connection_list",
            "List connections where entity is source or target.",
            &json!({
                "entity_id": {"type": "string"},
                "connection_type": {"type": "string"}
            }),
            &["entity_id"],
        ),
        tool_no_params("connection_list_all", "All connections in the project."),
        tool(
            "connection_delete",
            "Delete a connection.",
            &json!({"id": {"type": "string"}}),
            &["id"],
        ),
    ]
}

fn search_tools() -> Vec<Value> {
    vec![
        tool(
            "search",
            "Full-text search across all entities. Returns matches with snippets. Optional filters narrow results; a filter that can't apply to an entity kind (e.g. severity on items) drops that kind from the results.",
            &json!({
                "query": {"type": "string"},
                "entity_type": {"type": "string", "enum": ["item", "note", "item_of_interest", "connection"]},
                "tags": {"type": "array", "items": {"type": "string"}},
                "severity": {"type": "string", "enum": ["critical", "high", "medium", "low", "info"]},
                "connection_type": {"type": "string"},
                "author_type": {"type": "string", "enum": ["human", "agent"]}
            }),
            &["query"],
        ),
        tool(
            "filter",
            "Structured query without text search. Returns matching entities.",
            &json!({
                "entity_type": {"type": "string", "enum": ["item", "note", "item_of_interest", "connection"]},
                "tags": {"type": "array", "items": {"type": "string"}},
                "severity": {"type": "string", "enum": ["critical", "high", "medium", "low", "info"]},
                "connection_type": {"type": "string"},
                "author_type": {"type": "string", "enum": ["human", "agent"]},
                "item_id": {"type": "string"},
                "analysis_status": {"type": "string", "enum": ["untouched", "in_progress", "reviewed"]}
            }),
            &["entity_type"],
        ),
    ]
}

fn bulk_tools() -> Vec<Value> {
    vec![tool(
        "bulk_delete",
        "Delete all matching entities. At least one filter required.",
        &json!({
            "author": {"type": "string"},
            "since": {"type": "string", "description": "ISO 8601 timestamp"},
            "entity_type": {"type": "string", "enum": ["note", "item_of_interest", "connection", "item"]}
        }),
        &[],
    )]
}
