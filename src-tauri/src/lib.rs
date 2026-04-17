pub mod db;
pub mod mcp;

pub const MCP_PORT: u16 = 27182;

#[cfg(feature = "gui")]
pub mod commands;

#[cfg(feature = "gui")]
pub use gui::run;

#[cfg(feature = "gui")]
mod gui {
    use std::sync::{Arc, Mutex};

    use tauri::{Emitter, Manager};

    use crate::db::Database;
    use crate::mcp::server::McpServer;
    use crate::{commands, mcp, MCP_PORT};

    #[cfg_attr(mobile, tauri::mobile_entry_point)]
    pub fn run() {
        #[cfg(target_os = "linux")]
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

        tauri::Builder::default()
            .plugin(tauri_plugin_opener::init())
            .invoke_handler(tauri::generate_handler![commands::project_snapshot])
            .setup(|app| {
                let db_path = std::env::current_dir()
                    .unwrap_or_default()
                    .join("project.lsvr");
                let db = Database::open_or_init(&db_path, "New Project")
                    .expect("Failed to open database");

                let db = Arc::new(Mutex::new(db));

                app.manage(Arc::clone(&db));

                let app_handle = app.handle().clone();
                let on_change: mcp::server::OnChange = Arc::new(move || {
                    let _ = app_handle.emit("db-changed", ());
                });

                let server = McpServer::from_shared(Arc::clone(&db)).with_on_change(on_change);
                tauri::async_runtime::spawn(async move {
                    match server.start(MCP_PORT).await {
                        Ok(addr) => eprintln!("MCP server listening on {addr}"),
                        Err(e) => eprintln!("Failed to start MCP server: {e}"),
                    }
                });

                Ok(())
            })
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    }
}
