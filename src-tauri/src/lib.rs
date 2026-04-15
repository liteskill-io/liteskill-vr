pub mod commands;
pub mod db;
pub mod mcp;

use std::sync::{Arc, Mutex};

use tauri::Manager;

use db::Database;
use mcp::server::McpServer;

const MCP_PORT: u16 = 27182;

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

            let server = McpServer::from_shared(Arc::clone(&db));
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
