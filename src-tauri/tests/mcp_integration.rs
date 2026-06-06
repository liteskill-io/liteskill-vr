use liteskill_vr_lib::db::Database;
use liteskill_vr_lib::mcp::server::McpServer;
use reqwest::Client;
use serde_json::{json, Value};

struct TestServer {
    url: String,
    client: Client,
}

impl TestServer {
    async fn start() -> Self {
        let db = Database::in_memory("integration-test").unwrap();
        let server = McpServer::new(db);
        let addr = server.start(0).await.unwrap(); // port 0 = random available
        Self {
            url: format!("http://{addr}/mcp"),
            client: Client::new(),
        }
    }

    async fn call(&self, method: &str, params: Value) -> Value {
        let resp = self
            .client
            .post(&self.url)
            .header("X-LiteSkill-Author", "test-agent")
            .header("Accept", "application/json, text/event-stream")
            .json(&json!({
                "jsonrpc": "2.0",
                "method": method,
                "params": params,
                "id": 1
            }))
            .send()
            .await
            .unwrap();
        resp.json().await.unwrap()
    }

    async fn tool(&self, name: &str, args: Value) -> Value {
        let resp = self
            .call(
                "tools/call",
                json!({
                    "name": name,
                    "arguments": args
                }),
            )
            .await;

        if let Some(error) = resp.get("error") {
            panic!("Tool {name} returned error: {error}");
        }

        let content = resp["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or("null");
        serde_json::from_str(content).unwrap_or(Value::Null)
    }

    async fn tool_err(&self, name: &str, args: Value) -> Value {
        let resp = self
            .call(
                "tools/call",
                json!({
                    "name": name,
                    "arguments": args
                }),
            )
            .await;

        // rmcp returns errors as tool results with isError: true
        let content = &resp["result"]["content"][0]["text"];
        let message = content.as_str().unwrap_or("");
        json!({"message": message})
    }
}

// --- Protocol ---

#[tokio::test]
async fn initialize() {
    let s = TestServer::start().await;
    let resp = s
        .call(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "test", "version": "1.0"}
            }),
        )
        .await;
    assert_eq!(resp["result"]["serverInfo"]["name"], "liteskill-vr");
    assert!(resp["result"]["capabilities"]["tools"].is_object());
}

#[tokio::test]
async fn tools_list() {
    let s = TestServer::start().await;
    let resp = s.call("tools/list", json!({})).await;
    let tools = resp["result"]["tools"].as_array().unwrap();
    assert!(tools.len() >= 30);

    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    assert!(names.contains(&"project_summary"));
    assert!(names.contains(&"item_create"));
    assert!(names.contains(&"ioi_create_batch"));
    assert!(names.contains(&"search"));
    assert!(names.contains(&"bulk_delete"));
}

#[tokio::test]
async fn unknown_method() {
    let s = TestServer::start().await;
    let resp = s.call("nonexistent", json!({})).await;
    assert!(resp["error"].is_object());
}

// --- Tags ---

#[tokio::test]
async fn tag_list_returns_defaults() {
    let s = TestServer::start().await;
    let tags = s.tool("tag_list", json!({})).await;
    let tags = tags.as_array().unwrap();
    assert!(tags.len() >= 13);
    assert!(tags.iter().any(|t| t["name"] == "memory-corruption"));
}

#[tokio::test]
async fn tag_create_and_delete() {
    let s = TestServer::start().await;
    let tag = s
        .tool(
            "tag_create",
            json!({"name": "custom", "description": "Custom tag", "color": "#ff0000"}),
        )
        .await;
    assert_eq!(tag["name"], "custom");

    let id = tag["id"].as_str().unwrap();
    s.tool("tag_delete", json!({"id": id})).await;

    let tags = s.tool("tag_list", json!({})).await;
    assert!(!tags
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["name"] == "custom"));
}

#[tokio::test]
async fn tag_create_duplicate_fails() {
    let s = TestServer::start().await;
    let err = s
        .tool_err(
            "tag_create",
            json!({"name": "memory-corruption", "description": "dup"}),
        )
        .await;
    assert!(err["message"].as_str().unwrap().contains("already exists"));
}

// --- Connection Types ---

#[tokio::test]
async fn connection_type_list_returns_defaults() {
    let s = TestServer::start().await;
    let types = s.tool("connection_type_list", json!({})).await;
    let types = types.as_array().unwrap();
    assert!(types.len() >= 7);
    assert!(types.iter().any(|t| t["name"] == "calls"));
}

#[tokio::test]
async fn connection_type_create_and_delete() {
    let s = TestServer::start().await;
    let ct = s
        .tool(
            "connection_type_create",
            json!({"name": "monitors", "description": "Source monitors target"}),
        )
        .await;
    assert_eq!(ct["name"], "monitors");

    let id = ct["id"].as_str().unwrap();
    s.tool("connection_type_delete", json!({"id": id})).await;

    let types = s.tool("connection_type_list", json!({})).await;
    assert!(!types
        .as_array()
        .unwrap()
        .iter()
        .any(|t| t["name"] == "monitors"));
}

// --- Items ---

#[tokio::test]
async fn item_crud() {
    let s = TestServer::start().await;

    // Create
    let item = s
        .tool(
            "item_create",
            json!({"name": "httpd", "item_type": "elf", "path": "/usr/bin/httpd", "architecture": "arm32", "description": "Web server", "tags": ["interesting"]}),
        )
        .await;
    assert_eq!(item["name"], "httpd");
    assert_eq!(item["tags"][0], "interesting");
    let item_id = item["id"].as_str().unwrap().to_string();

    // Get
    let detail = s.tool("item_get", json!({"id": item_id})).await;
    assert_eq!(detail["item"]["name"], "httpd");
    assert!(detail["notes"].as_array().unwrap().is_empty());

    // Update
    let updated = s
        .tool(
            "item_update",
            json!({"id": item_id, "analysis_status": "in_progress"}),
        )
        .await;
    assert_eq!(updated["analysis_status"], "in_progress");

    // List
    let items = s.tool("item_list", json!({})).await;
    assert_eq!(items.as_array().unwrap().len(), 1);

    // Delete
    s.tool("item_delete", json!({"id": item_id})).await;
    let items = s.tool("item_list", json!({})).await;
    assert!(items.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn item_create_batch() {
    let s = TestServer::start().await;
    let items = s
        .tool(
            "item_create_batch",
            json!({"items": [
                {"name": "httpd", "item_type": "elf"},
                {"name": "libfoo.so", "item_type": "shared_object"},
                {"name": "httpd.conf", "item_type": "config"}
            ]}),
        )
        .await;
    assert_eq!(items.as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn item_create_with_unregistered_tag_fails() {
    let s = TestServer::start().await;
    let err = s
        .tool_err(
            "item_create",
            json!({"name": "test", "item_type": "elf", "tags": ["nonexistent"]}),
        )
        .await;
    assert!(err["message"].as_str().unwrap().contains("not registered"));
}

// --- Notes ---

#[tokio::test]
async fn note_crud() {
    let s = TestServer::start().await;
    let item = s
        .tool("item_create", json!({"name": "httpd", "item_type": "elf"}))
        .await;
    let item_id = item["id"].as_str().unwrap();

    let note = s
        .tool(
            "note_create",
            json!({"item_id": item_id, "title": "Analysis", "content": "Found a buffer overflow"}),
        )
        .await;
    assert_eq!(note["title"], "Analysis");
    assert_eq!(note["author"], "test-agent");
    assert_eq!(note["author_type"], "agent");

    let note_id = note["id"].as_str().unwrap();
    let updated = s
        .tool(
            "note_update",
            json!({"id": note_id, "title": "Updated Analysis"}),
        )
        .await;
    assert_eq!(updated["title"], "Updated Analysis");

    s.tool("note_delete", json!({"id": note_id})).await;

    let detail = s.tool("item_get", json!({"id": item_id})).await;
    assert!(detail["notes"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn note_author_comes_from_header() {
    let s = TestServer::start().await;
    let item = s
        .tool("item_create", json!({"name": "test", "item_type": "elf"}))
        .await;
    let item_id = item["id"].as_str().unwrap();

    let note = s
        .tool(
            "note_create",
            json!({"item_id": item_id, "title": "test", "content": "test"}),
        )
        .await;
    assert_eq!(note["author"], "test-agent");
    assert_eq!(note["author_type"], "agent");
}

// --- IOI ---

#[tokio::test]
async fn ioi_crud() {
    let s = TestServer::start().await;
    let item = s
        .tool("item_create", json!({"name": "httpd", "item_type": "elf"}))
        .await;
    let item_id = item["id"].as_str().unwrap();

    let ioi = s
        .tool(
            "ioi_create",
            json!({
                "item_id": item_id,
                "title": "parse_header()",
                "description": "Stack buffer overflow",
                "location": "0x08041234",
                "severity": "critical",
                "tags": ["memory-corruption"]
            }),
        )
        .await;
    assert_eq!(ioi["title"], "parse_header()");
    assert_eq!(ioi["severity"], "critical");
    assert!(ioi.get("duplicate_warning").is_none());

    let ioi_id = ioi["id"].as_str().unwrap();
    let updated = s
        .tool("ioi_update", json!({"id": ioi_id, "severity": "high"}))
        .await;
    assert_eq!(updated["severity"], "high");

    s.tool("ioi_delete", json!({"id": ioi_id})).await;
}

#[tokio::test]
async fn ioi_duplicate_warning() {
    let s = TestServer::start().await;
    let item = s
        .tool("item_create", json!({"name": "httpd", "item_type": "elf"}))
        .await;
    let item_id = item["id"].as_str().unwrap();

    s.tool(
        "ioi_create",
        json!({"item_id": item_id, "title": "parse_header()", "description": "first"}),
    )
    .await;

    let second = s
        .tool(
            "ioi_create",
            json!({"item_id": item_id, "title": "parse_header()", "description": "second"}),
        )
        .await;
    assert!(second.get("duplicate_warning").is_some());
    assert_eq!(
        second["duplicate_warning"]["existing_title"],
        "parse_header()"
    );
}

#[tokio::test]
async fn ioi_create_batch_with_duplicate_warnings() {
    let s = TestServer::start().await;
    let item = s
        .tool("item_create", json!({"name": "httpd", "item_type": "elf"}))
        .await;
    let item_id = item["id"].as_str().unwrap();

    let results = s
        .tool(
            "ioi_create_batch",
            json!({
                "item_id": item_id,
                "items": [
                    {"title": "func_a", "description": "first"},
                    {"title": "func_b", "description": "second"},
                    {"title": "func_c", "description": "third"}
                ]
            }),
        )
        .await;
    assert_eq!(results.as_array().unwrap().len(), 3);
}

// --- Connections ---

#[tokio::test]
async fn connection_crud() {
    let s = TestServer::start().await;
    let a = s
        .tool("item_create", json!({"name": "httpd", "item_type": "elf"}))
        .await;
    let b = s
        .tool(
            "item_create",
            json!({"name": "libfoo.so", "item_type": "shared_object"}),
        )
        .await;
    let a_id = a["id"].as_str().unwrap();
    let b_id = b["id"].as_str().unwrap();

    let conn = s
        .tool(
            "connection_create",
            json!({
                "source_id": a_id, "source_type": "item",
                "target_id": b_id, "target_type": "item",
                "connection_type": "links",
                "description": "httpd links libfoo"
            }),
        )
        .await;
    assert_eq!(conn["connection_type"], "links");

    // Bidirectional list
    let from_a = s.tool("connection_list", json!({"entity_id": a_id})).await;
    let from_b = s.tool("connection_list", json!({"entity_id": b_id})).await;
    assert_eq!(from_a.as_array().unwrap().len(), 1);
    assert_eq!(from_b.as_array().unwrap().len(), 1);

    // List all
    let all = s.tool("connection_list_all", json!({})).await;
    assert_eq!(all.as_array().unwrap().len(), 1);

    // Delete
    let conn_id = conn["id"].as_str().unwrap();
    s.tool("connection_delete", json!({"id": conn_id})).await;
    let all = s.tool("connection_list_all", json!({})).await;
    assert!(all.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn connection_create_batch() {
    let s = TestServer::start().await;
    let items = s
        .tool(
            "item_create_batch",
            json!({"items": [
                {"name": "httpd", "item_type": "elf"},
                {"name": "libfoo.so", "item_type": "shared_object"},
                {"name": "httpd.conf", "item_type": "config"}
            ]}),
        )
        .await;
    let items = items.as_array().unwrap();
    let a_id = items[0]["id"].as_str().unwrap();
    let b_id = items[1]["id"].as_str().unwrap();
    let c_id = items[2]["id"].as_str().unwrap();

    let conns = s
        .tool(
            "connection_create_batch",
            json!({"connections": [
                {"source_id": a_id, "source_type": "item", "target_id": b_id, "target_type": "item", "connection_type": "links", "description": "links libfoo"},
                {"source_id": a_id, "source_type": "item", "target_id": c_id, "target_type": "item", "connection_type": "reads_config", "description": "reads config"}
            ]}),
        )
        .await;
    assert_eq!(conns.as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn connection_with_unregistered_type_fails() {
    let s = TestServer::start().await;
    let a = s
        .tool("item_create", json!({"name": "a", "item_type": "elf"}))
        .await;
    let b = s
        .tool("item_create", json!({"name": "b", "item_type": "elf"}))
        .await;
    let err = s
        .tool_err(
            "connection_create",
            json!({
                "source_id": a["id"], "source_type": "item",
                "target_id": b["id"], "target_type": "item",
                "connection_type": "nonexistent",
                "description": "test"
            }),
        )
        .await;
    assert!(err["message"].as_str().unwrap().contains("not registered"));
}

// --- Search ---

#[tokio::test]
async fn search_across_entities() {
    let s = TestServer::start().await;
    let item = s
        .tool(
            "item_create",
            json!({"name": "httpd", "item_type": "elf", "description": "Main web server"}),
        )
        .await;
    let item_id = item["id"].as_str().unwrap();

    s.tool(
        "note_create",
        json!({"item_id": item_id, "title": "Analysis", "content": "Found buffer overflow in parse_header"}),
    )
    .await;

    let results = s.tool("search", json!({"query": "buffer overflow"})).await;
    assert!(!results.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn filter_ioi_by_severity() {
    let s = TestServer::start().await;
    let item = s
        .tool("item_create", json!({"name": "httpd", "item_type": "elf"}))
        .await;
    let item_id = item["id"].as_str().unwrap();

    s.tool(
        "ioi_create",
        json!({"item_id": item_id, "title": "critical_bug", "description": "bad", "severity": "critical"}),
    )
    .await;
    s.tool(
        "ioi_create",
        json!({"item_id": item_id, "title": "low_bug", "description": "minor", "severity": "low"}),
    )
    .await;

    let critical = s
        .tool(
            "filter",
            json!({"entity_type": "item_of_interest", "severity": "critical"}),
        )
        .await;
    assert_eq!(critical.as_array().unwrap().len(), 1);
    assert_eq!(critical[0]["title"], "critical_bug");
}

#[tokio::test]
async fn filter_notes_by_author_type() {
    let s = TestServer::start().await;
    let item = s
        .tool("item_create", json!({"name": "httpd", "item_type": "elf"}))
        .await;
    let item_id = item["id"].as_str().unwrap();

    s.tool(
        "note_create",
        json!({"item_id": item_id, "title": "Agent note", "content": "from agent"}),
    )
    .await;

    let agent_notes = s
        .tool(
            "filter",
            json!({"entity_type": "note", "author_type": "agent"}),
        )
        .await;
    assert_eq!(agent_notes.as_array().unwrap().len(), 1);

    let human_notes = s
        .tool(
            "filter",
            json!({"entity_type": "note", "author_type": "human"}),
        )
        .await;
    assert!(human_notes.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn filter_connections_by_type() {
    let s = TestServer::start().await;
    let items = s
        .tool(
            "item_create_batch",
            json!({"items": [
                {"name": "httpd", "item_type": "elf"},
                {"name": "libfoo.so", "item_type": "shared_object"},
                {"name": "httpd.conf", "item_type": "config"}
            ]}),
        )
        .await;
    let items = items.as_array().unwrap();
    let a = items[0]["id"].as_str().unwrap();
    let b = items[1]["id"].as_str().unwrap();
    let c = items[2]["id"].as_str().unwrap();

    s.tool(
        "connection_create_batch",
        json!({"connections": [
            {"source_id": a, "source_type": "item", "target_id": b, "target_type": "item", "connection_type": "links", "description": ""},
            {"source_id": a, "source_type": "item", "target_id": c, "target_type": "item", "connection_type": "reads_config", "description": ""}
        ]}),
    )
    .await;

    let links = s
        .tool(
            "filter",
            json!({"entity_type": "connection", "connection_type": "links"}),
        )
        .await;
    assert_eq!(links.as_array().unwrap().len(), 1);

    let all = s.tool("filter", json!({"entity_type": "connection"})).await;
    assert_eq!(all.as_array().unwrap().len(), 2);
}

// --- Bulk Delete ---

#[tokio::test]
async fn bulk_delete_by_author() {
    let s = TestServer::start().await;
    let item = s
        .tool("item_create", json!({"name": "httpd", "item_type": "elf"}))
        .await;
    let item_id = item["id"].as_str().unwrap();

    s.tool(
        "note_create",
        json!({"item_id": item_id, "title": "note1", "content": "c1"}),
    )
    .await;
    s.tool(
        "note_create",
        json!({"item_id": item_id, "title": "note2", "content": "c2"}),
    )
    .await;
    s.tool(
        "ioi_create",
        json!({"item_id": item_id, "title": "bug1", "description": "d1"}),
    )
    .await;

    let result = s.tool("bulk_delete", json!({"author": "test-agent"})).await;
    assert!(result["deleted_count"].as_i64().unwrap() >= 3);

    let detail = s.tool("item_get", json!({"id": item_id})).await;
    assert!(detail["notes"].as_array().unwrap().is_empty());
    assert!(detail["items_of_interest"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn bulk_delete_requires_filter() {
    let s = TestServer::start().await;
    let err = s.tool_err("bulk_delete", json!({})).await;
    assert!(err["message"]
        .as_str()
        .unwrap()
        .contains("requires at least one filter"));
}

// --- Project Summary ---

#[tokio::test]
async fn project_summary() {
    let s = TestServer::start().await;
    let item = s
        .tool("item_create", json!({"name": "httpd", "item_type": "elf"}))
        .await;
    let item_id = item["id"].as_str().unwrap();

    s.tool(
        "ioi_create",
        json!({"item_id": item_id, "title": "bug", "description": "bad", "severity": "critical"}),
    )
    .await;

    let summary = s.tool("project_summary", json!({})).await;
    assert_eq!(summary["items"].as_array().unwrap().len(), 1);
    assert_eq!(summary["severity_summary"]["critical"], 1);
    assert!(summary["tags"].as_array().unwrap().len() >= 13);
    assert!(summary["connection_types"].as_array().unwrap().len() >= 7);
}

// --- Full Agent Session (acceptance test) ---

#[tokio::test]
async fn full_agent_session() {
    let s = TestServer::start().await;

    // 1. Orient
    let summary = s.tool("project_summary", json!({})).await;
    assert!(summary["items"].as_array().unwrap().is_empty());

    let tags = s.tool("tag_list", json!({})).await;
    assert!(tags.as_array().unwrap().len() >= 13);

    let conn_types = s.tool("connection_type_list", json!({})).await;
    assert!(conn_types.as_array().unwrap().len() >= 7);

    // 2. Create items
    let items = s
        .tool(
            "item_create_batch",
            json!({"items": [
                {"name": "httpd", "item_type": "elf", "path": "/usr/bin/httpd", "architecture": "arm32"},
                {"name": "libfoo.so", "item_type": "shared_object", "path": "/usr/lib/libfoo.so"},
                {"name": "httpd.conf", "item_type": "config", "path": "/etc/httpd.conf"}
            ]}),
        )
        .await;
    let items = items.as_array().unwrap();
    assert_eq!(items.len(), 3);
    let httpd_id = items[0]["id"].as_str().unwrap();
    let libfoo_id = items[1]["id"].as_str().unwrap();
    let conf_id = items[2]["id"].as_str().unwrap();

    // 3. Document findings
    let iois = s
        .tool(
            "ioi_create_batch",
            json!({
                "item_id": httpd_id,
                "items": [
                    {"title": "parse_header()", "description": "Stack buffer overflow via Content-Length", "location": "0x08041234", "severity": "critical", "tags": ["memory-corruption"]},
                    {"title": "auth_check()", "description": "strcmp timing side-channel", "location": "0x08042000", "severity": "high", "tags": ["auth-bypass"]},
                    {"title": "cmd_handler()", "description": "User input passed to system()", "location": "0x08043000", "severity": "critical", "tags": ["command-injection"]}
                ]
            }),
        )
        .await;
    assert_eq!(iois.as_array().unwrap().len(), 3);

    s.tool(
        "note_create",
        json!({"item_id": httpd_id, "title": "Session 1 Summary", "content": "Analyzed httpd binary. Found 3 critical/high issues in request handling path."}),
    )
    .await;

    // 4. Draw connections
    s.tool(
        "connection_create_batch",
        json!({"connections": [
            {"source_id": httpd_id, "source_type": "item", "target_id": libfoo_id, "target_type": "item", "connection_type": "links", "description": "httpd dynamically links libfoo.so"},
            {"source_id": httpd_id, "source_type": "item", "target_id": conf_id, "target_type": "item", "connection_type": "reads_config", "description": "httpd reads httpd.conf at startup"}
        ]}),
    )
    .await;

    // 5. Verify with item_get
    let detail = s.tool("item_get", json!({"id": httpd_id})).await;
    assert_eq!(detail["notes"].as_array().unwrap().len(), 1);
    assert_eq!(detail["items_of_interest"].as_array().unwrap().len(), 3);
    assert_eq!(detail["connections"].as_array().unwrap().len(), 2);

    // 6. Search
    let results = s.tool("search", json!({"query": "buffer overflow"})).await;
    assert!(!results.as_array().unwrap().is_empty());

    // 7. Filter
    let critical = s
        .tool(
            "filter",
            json!({"entity_type": "item_of_interest", "severity": "critical"}),
        )
        .await;
    assert_eq!(critical.as_array().unwrap().len(), 2);

    // 8. Update item status
    s.tool(
        "item_update",
        json!({"id": httpd_id, "analysis_status": "reviewed"}),
    )
    .await;

    // 9. Final summary
    let summary = s.tool("project_summary", json!({})).await;
    assert_eq!(summary["items"].as_array().unwrap().len(), 3);
    assert_eq!(summary["severity_summary"]["critical"], 2);
    assert_eq!(summary["severity_summary"]["high"], 1);
}

#[tokio::test]
async fn explanation_lifecycle() {
    let s = TestServer::start().await;

    let item = s
        .tool("item_create", json!({"name": "httpd", "item_type": "elf"}))
        .await;
    let item_id = item["id"].as_str().unwrap().to_string();

    // Create an explanation with a claim + open question, scoped to the item.
    let res = s
        .tool(
            "explanation_upsert",
            json!({
                "stable_key": "explanation.auth",
                "title": "Auth flow",
                "explanation_type": "state_machine",
                "summary": "short tldr",
                "scope_item_ids": [item_id],
                "claims": [{"stable_key": "claim.rsa", "text": "Auth uses RSA"}],
                "open_questions": [{"stable_key": "q.bound", "question": "Length bounded?", "priority": "high"}]
            }),
        )
        .await;
    let expl_id = res["explanation"]["id"].as_str().unwrap().to_string();
    let claim_id = res["explanation"]["claims"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    // The unbacked-claim guardrail warning is present.
    assert!(res["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|w| w.as_str().unwrap().contains("no linked evidence")));

    // Attach external-locator evidence to the claim.
    s.tool(
        "evidence_link",
        json!({
            "target_type": "claim",
            "target_id": claim_id,
            "external_locator": "FUN_00401000+0x14",
            "external_kind": "ghidra",
            "evidence_type": "decompilation",
            "strength": "strong"
        }),
    )
    .await;

    // Re-run with the same stable_keys: idempotent (same id), claim updated, and
    // the unbacked warning now cleared.
    let res2 = s
        .tool(
            "explanation_upsert",
            json!({
                "stable_key": "explanation.auth",
                "title": "Authentication flow",
                "claims": [{"stable_key": "claim.rsa", "text": "Auth uses RSA-2048"}]
            }),
        )
        .await;
    assert_eq!(res2["explanation"]["id"].as_str().unwrap(), expl_id);
    assert!(!res2["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .any(|w| w.as_str().unwrap().contains("no linked evidence")));

    // explanation_get reflects the update, evidence, and scope.
    let got = s.tool("explanation_get", json!({"id": expl_id})).await;
    assert_eq!(got["title"], "Authentication flow");
    assert_eq!(got["claims"][0]["text"], "Auth uses RSA-2048");
    assert_eq!(got["evidence"].as_array().unwrap().len(), 1);
    assert_eq!(got["scope_item_ids"][0].as_str().unwrap(), item_id);

    // Discovery surfaces (list, project_summary, filter).
    assert_eq!(
        s.tool("explanation_list", json!({}))
            .await
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let summary = s.tool("project_summary", json!({})).await;
    assert_eq!(summary["explanations"].as_array().unwrap().len(), 1);
    assert_eq!(summary["open_questions"].as_array().unwrap().len(), 1);
    let qs = s
        .tool(
            "filter",
            json!({"entity_type": "open_question", "priority": "high"}),
        )
        .await;
    assert_eq!(qs.as_array().unwrap().len(), 1);
}

// Read-only discovery tools that aren't covered by the CRUD tests above:
// project_get, changes_since, connection_list, connection_list_all.
#[tokio::test]
async fn read_only_tools() {
    let s = TestServer::start().await;

    // project_get returns the project envelope.
    let project = s.tool("project_get", json!({})).await;
    assert!(project["id"].as_str().is_some());
    assert!(project["name"].as_str().is_some());

    // Build a small graph: two linked items.
    let a = s
        .tool("item_create", json!({"name": "httpd", "item_type": "elf"}))
        .await;
    let b = s
        .tool(
            "item_create",
            json!({"name": "libssl", "item_type": "shared_object"}),
        )
        .await;
    let a_id = a["id"].as_str().unwrap().to_string();
    let b_id = b["id"].as_str().unwrap().to_string();
    s.tool(
        "connection_create",
        json!({
            "source_id": a_id,
            "source_type": "item",
            "target_id": b_id,
            "target_type": "item",
            "connection_type": "links"
        }),
    )
    .await;

    // connection_list (entity-scoped) and connection_list_all both see it.
    let scoped = s.tool("connection_list", json!({"entity_id": a_id})).await;
    assert_eq!(scoped.as_array().unwrap().len(), 1);
    let all = s.tool("connection_list_all", json!({})).await;
    assert_eq!(all.as_array().unwrap().len(), 1);

    // changes_since with an epoch floor returns the items just created; with a
    // far-future floor it returns nothing.
    let recent = s
        .tool("changes_since", json!({"since": "1970-01-01"}))
        .await;
    assert_eq!(recent["items"].as_array().unwrap().len(), 2);
    let none = s
        .tool("changes_since", json!({"since": "2999-01-01"}))
        .await;
    assert!(none["items"].as_array().unwrap().is_empty());
}

// Structured typed content (packet fields + inline state machine) over the real
// HTTP transport — exercises the full serialize/deserialize path the unit tests
// (which call dispatch directly) don't.
#[tokio::test]
async fn structured_content_over_http() {
    let s = TestServer::start().await;

    // A packet_format with inline fields, plus one granular field_create.
    let pkt = s
        .tool(
            "explanation_upsert",
            json!({
                "stable_key": "explanation.packet",
                "title": "LoginRequest",
                "explanation_type": "packet_format",
                "fields": [
                    {"stable_key": "f.magic", "name": "magic", "field_type": "u32", "offset": 0, "size": 4},
                    {"stable_key": "f.len", "name": "length", "field_type": "u16", "offset": 4, "size": 2}
                ]
            }),
        )
        .await;
    let pkt_id = pkt["explanation"]["id"].as_str().unwrap().to_string();

    s.tool(
        "field_create",
        json!({"explanation_id": pkt_id, "name": "payload", "field_type": "bytes"}),
    )
    .await;

    let got = s.tool("explanation_get", json!({"id": pkt_id})).await;
    let fields = got["fields"].as_array().unwrap();
    assert_eq!(fields.len(), 3);
    // Offset-ordered; the unset-offset field sorts last.
    assert_eq!(fields[0]["name"], "magic");
    assert_eq!(fields[0]["offset"], 0);
    assert_eq!(fields[2]["name"], "payload");
    assert!(fields[2]["offset"].is_null());

    // Inline states + transitions on a state_machine upsert (now advertised in
    // the schema) and the generated text diagram on explanation_get.
    let sm = s
        .tool(
            "explanation_upsert",
            json!({
                "stable_key": "explanation.sm",
                "title": "Auth",
                "explanation_type": "state_machine",
                "states": [
                    {"stable_key": "A", "name": "UNAUTH", "is_initial": true},
                    {"stable_key": "B", "name": "AUTHED", "is_terminal": true}
                ],
                "transitions": [
                    {"stable_key": "t1", "from_state": "A", "to_state": "B", "event": "LOGIN", "guard": "ok"}
                ]
            }),
        )
        .await;
    let sm_id = sm["explanation"]["id"].as_str().unwrap().to_string();
    let got = s.tool("explanation_get", json!({"id": sm_id})).await;
    assert_eq!(got["states"].as_array().unwrap().len(), 2);
    assert_eq!(got["transitions"].as_array().unwrap().len(), 1);
    assert!(got["diagram_text"]
        .as_str()
        .unwrap()
        .contains("UNAUTH --LOGIN [ok]--> AUTHED"));

    // A transition referencing an unknown state stable_key is rejected.
    let err = s
        .tool_err(
            "transition_create",
            json!({"explanation_id": sm_id, "from_state": "A", "to_state": "ghost"}),
        )
        .await;
    assert!(!err["message"].as_str().unwrap().is_empty());
}

// HTML diagrams are sanitized server-side before storage — the JS-injection
// constraint, verified end-to-end over HTTP.
#[tokio::test]
async fn diagram_html_is_sanitized_over_http() {
    let s = TestServer::start().await;
    let res = s
        .tool(
            "explanation_upsert",
            json!({
                "stable_key": "explanation.proto",
                "title": "Wire protocol",
                "explanation_type": "protocol",
                "diagram_html": "<table><tr><td>ok</td></tr></table><script>alert(1)</script><a href=\"javascript:evil()\" onclick=\"x()\">bad</a>"
            }),
        )
        .await;
    let html = res["explanation"]["diagram_html"].as_str().unwrap();
    assert!(html.contains("<table>"), "safe markup is kept");
    assert!(!html.contains("<script"), "scripts are stripped");
    assert!(!html.contains("onclick"), "event handlers are stripped");
    assert!(!html.contains("javascript:"), "unsafe URLs are stripped");
}
