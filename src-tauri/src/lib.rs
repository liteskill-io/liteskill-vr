pub mod commands;
pub mod db;
pub mod mcp;

use std::sync::{Arc, Mutex};

use tauri::{Emitter, Manager};

use db::Database;
use mcp::server::McpServer;

pub const MCP_PORT: u16 = 27182;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_project,
            commands::list_items,
            commands::get_item,
            commands::list_tags,
            commands::list_connection_types,
            commands::project_snapshot,
        ])
        .setup(|app| {
            let db_path = std::env::current_dir()
                .unwrap_or_default()
                .join("project.lsvr");

            let db = if db_path.exists() {
                Database::open(&db_path)
            } else {
                Database::open_and_seed(&db_path, "New Project")
            }
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
