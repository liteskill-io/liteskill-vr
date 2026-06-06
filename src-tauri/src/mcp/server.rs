use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use serde_json::json;

use crate::db::Database;

use super::handlers;
use super::tools;

pub type OnChange = Arc<dyn Fn() + Send + Sync>;

pub struct McpServer {
    db: Arc<Mutex<Database>>,
    on_change: Option<OnChange>,
}

impl McpServer {
    pub fn new(db: Database) -> Self {
        Self {
            db: Arc::new(Mutex::new(db)),
            on_change: None,
        }
    }

    pub const fn from_shared(db: Arc<Mutex<Database>>) -> Self {
        Self {
            db,
            on_change: None,
        }
    }

    #[must_use]
    pub fn with_on_change(mut self, on_change: OnChange) -> Self {
        self.on_change = Some(on_change);
        self
    }

    /// Serve over stdio (JSON-RPC on stdin/stdout). Intended for MCP clients that
    /// spawn the server as a subprocess. The author header trick used by HTTP
    /// isn't available here, so all writes are attributed to `"stdio-agent"`.
    /// Blocks until the client closes stdin.
    pub async fn serve_stdio(self) -> Result<(), std::io::Error> {
        use rmcp::transport::stdio;
        use rmcp::ServiceExt;

        let handler = LiteSkillHandler {
            db: self.db,
            on_change: self.on_change,
            author_override: Some("stdio-agent".to_string()),
        };

        let running = handler
            .serve(stdio())
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        running
            .waiting()
            .await
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        Ok(())
    }

    pub async fn start(self, port: u16) -> Result<SocketAddr, std::io::Error> {
        let db = self.db;
        let on_change = self.on_change;

        let service_factory = move || {
            let db = Arc::clone(&db);
            Ok(LiteSkillHandler {
                db,
                on_change: on_change.clone(),
                author_override: None,
            })
        };

        let config = rmcp::transport::StreamableHttpServerConfig::default()
            .with_stateful_mode(false)
            .with_json_response(true);

        let session_manager = Arc::new(
            rmcp::transport::streamable_http_server::session::never::NeverSessionManager::default(),
        );

        let mcp_service =
            rmcp::transport::StreamableHttpService::new(service_factory, session_manager, config);

        let app = axum::Router::new().route("/mcp", axum::routing::any_service(mcp_service));

        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let listener = tokio::net::TcpListener::bind(addr).await?;
        let local_addr = listener.local_addr()?;

        tokio::spawn(async move {
            axum::serve(listener, app).await.ok();
        });

        Ok(local_addr)
    }
}

struct LiteSkillHandler {
    db: Arc<Mutex<Database>>,
    on_change: Option<OnChange>,
    author_override: Option<String>,
}

// Explicit list so adding a non-CRUD-named mutation (e.g. "archive_item") forces
// an update here rather than silently failing to notify listeners.
const MUTATION_TOOLS: &[&str] = &[
    "tag_create",
    "tag_delete",
    "connection_type_create",
    "connection_type_delete",
    "item_create",
    "item_create_batch",
    "item_update",
    "item_delete",
    "note_create",
    "note_create_batch",
    "note_update",
    "note_delete",
    "ioi_create",
    "ioi_create_batch",
    "ioi_update",
    "ioi_delete",
    "connection_create",
    "connection_create_batch",
    "connection_delete",
    "explanation_upsert",
    "explanation_update",
    "explanation_delete",
    "claim_create",
    "claim_update",
    "claim_delete",
    "open_question_create",
    "open_question_update",
    "open_question_delete",
    "evidence_link",
    "evidence_delete",
    "state_create",
    "state_update",
    "state_delete",
    "transition_create",
    "transition_update",
    "transition_delete",
    "field_create",
    "field_update",
    "field_delete",
    "bulk_delete",
];

fn is_mutation(tool_name: &str) -> bool {
    MUTATION_TOOLS.contains(&tool_name)
}

impl rmcp::ServerHandler for LiteSkillHandler {
    fn get_info(&self) -> rmcp::model::ServerInfo {
        let caps = rmcp::model::ServerCapabilities::builder()
            .enable_tools()
            .build();
        rmcp::model::InitializeResult::new(caps).with_server_info(rmcp::model::Implementation::new(
            "liteskill-vr",
            env!("CARGO_PKG_VERSION"),
        ))
    }

    fn list_tools(
        &self,
        _request: Option<rmcp::model::PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl std::future::Future<Output = Result<rmcp::model::ListToolsResult, rmcp::ErrorData>>
           + Send
           + '_ {
        let tool_defs = tools::list_all();
        let tools: Vec<rmcp::model::Tool> = tool_defs
            .into_iter()
            .filter_map(|def| {
                let name = def.get("name")?.as_str()?.to_string();
                let description = def.get("description")?.as_str()?.to_string();
                let input_schema = def
                    .get("inputSchema")
                    .and_then(|v| v.as_object())
                    .cloned()
                    .unwrap_or_default();
                Some(rmcp::model::Tool::new(name, description, input_schema))
            })
            .collect();

        std::future::ready(Ok(rmcp::model::ListToolsResult {
            tools,
            meta: None,
            next_cursor: None,
        }))
    }

    fn call_tool(
        &self,
        request: rmcp::model::CallToolRequestParams,
        context: rmcp::service::RequestContext<rmcp::RoleServer>,
    ) -> impl std::future::Future<Output = Result<rmcp::model::CallToolResult, rmcp::ErrorData>>
           + Send
           + '_ {
        let tool_name = request.name.to_string();
        let tool_args = request
            .arguments
            .map_or_else(|| json!({}), serde_json::Value::Object);

        let author = self.author_override.clone().unwrap_or_else(|| {
            context
                .extensions
                .get::<http::request::Parts>()
                .and_then(|parts| {
                    parts
                        .headers
                        .get("X-LiteSkill-Author")
                        .and_then(|v| v.to_str().ok())
                        .map(String::from)
                })
                .unwrap_or_else(|| "anonymous-agent".to_string())
        });

        // Recover from a poisoned lock rather than panicking the worker: a prior
        // handler panic doesn't corrupt the DB (SQLite rolls back mid-op writes),
        // so keep serving instead of wedging the server.
        let db = self
            .db
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let result = handlers::dispatch(&db, &tool_name, &tool_args, &author, "agent");
        drop(db);

        if result.is_ok() && is_mutation(&tool_name) {
            if let Some(ref on_change) = self.on_change {
                on_change();
            }
        }

        let call_result = match result {
            Ok(value) => {
                let text = serde_json::to_string(&value).unwrap_or_default();
                rmcp::model::CallToolResult::success(vec![rmcp::model::Content::text(text)])
            }
            Err(msg) => rmcp::model::CallToolResult::error(vec![rmcp::model::Content::text(msg)]),
        };

        std::future::ready(Ok(call_result))
    }
}

#[cfg(test)]
mod tests {
    use super::{is_mutation, MUTATION_TOOLS};
    use crate::db::Database;
    use crate::mcp::{handlers, tools};
    use serde_json::json;

    fn advertised_names() -> Vec<String> {
        tools::list_all()
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()).map(String::from))
            .collect()
    }

    // Every tool we advertise in `tools/list` must have a `dispatch` arm —
    // otherwise an agent calls a listed tool and gets "Unknown tool".
    #[test]
    fn every_advertised_tool_is_dispatchable() {
        let db = Database::in_memory("t").unwrap();
        for name in advertised_names() {
            // Empty args may fail validation, but must never be "Unknown tool".
            if let Err(e) = handlers::dispatch(&db, &name, &json!({}), "t", "agent") {
                assert!(
                    !e.contains("Unknown tool"),
                    "advertised tool `{name}` has no dispatch arm: {e}"
                );
            }
        }
    }

    // Every name in MUTATION_TOOLS must be a real, advertised tool (catches typos
    // and renames that would silently stop emitting `db-changed`).
    #[test]
    fn mutation_tools_are_all_advertised() {
        let advertised = advertised_names();
        for m in MUTATION_TOOLS {
            assert!(
                advertised.contains(&(*m).to_string()),
                "MUTATION_TOOLS entry `{m}` is not in tools::list_all()"
            );
        }
    }

    // Any tool whose name follows the CRUD/mutation convention must be classified
    // as a mutation, or the UI never gets a `db-changed` event after the write.
    #[test]
    fn crud_named_tools_are_classified_as_mutations() {
        const MARKERS: &[&str] = &["_create", "_update", "_delete", "_upsert", "_link"];
        for name in advertised_names() {
            let looks_mutating = MARKERS.iter().any(|m| name.contains(m));
            if looks_mutating {
                assert!(
                    is_mutation(&name),
                    "`{name}` looks like a mutation but is missing from MUTATION_TOOLS \
                     (writes via it won't emit db-changed → stale UI)"
                );
            }
        }
    }

    // A handler panic poisons the shared Mutex; the server must recover the guard
    // and keep dispatching instead of wedging every subsequent call.
    #[test]
    fn dispatch_survives_a_poisoned_lock() {
        use std::sync::{Arc, Mutex};

        let db = Arc::new(Mutex::new(Database::in_memory("t").unwrap()));

        // Poison the mutex: panic while holding the guard on another thread.
        // Silence the panic backtrace it would otherwise print to stderr.
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let poisoner = Arc::clone(&db);
        let joined = std::thread::spawn(move || {
            let _guard = poisoner.lock().unwrap();
            panic!("boom");
        })
        .join();
        std::panic::set_hook(prev_hook);
        assert!(joined.is_err(), "the poisoning thread should have panicked");
        assert!(db.lock().is_err(), "the mutex should now be poisoned");

        // The recovery pattern used in call_tool still yields a usable DB.
        let guard = db.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let res = handlers::dispatch(&guard, "project_get", &json!({}), "t", "agent");
        drop(guard);
        assert!(
            res.is_ok(),
            "dispatch should work through a recovered guard"
        );
    }
}
