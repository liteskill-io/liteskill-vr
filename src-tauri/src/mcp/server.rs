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

    pub async fn start(self, port: u16) -> Result<SocketAddr, std::io::Error> {
        let db = self.db;
        let on_change = self.on_change;

        let service_factory = move || {
            let db = Arc::clone(&db);
            Ok(LiteSkillHandler {
                db,
                on_change: on_change.clone(),
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
}

fn is_mutation(tool_name: &str) -> bool {
    tool_name.ends_with("_create")
        || tool_name.ends_with("_create_batch")
        || tool_name.ends_with("_update")
        || tool_name.ends_with("_delete")
        || tool_name == "bulk_delete"
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

        let author = context
            .extensions
            .get::<http::request::Parts>()
            .and_then(|parts| {
                parts
                    .headers
                    .get("X-LiteSkill-Author")
                    .and_then(|v| v.to_str().ok())
                    .map(String::from)
            })
            .unwrap_or_else(|| "anonymous-agent".to_string());

        let db = self.db.lock().unwrap();
        let result = handlers::dispatch(&db, &tool_name, &tool_args, &author);
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
