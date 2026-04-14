#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Work around WebKitGTK DMA-BUF renderer crashes on Wayland
    #[cfg(target_os = "linux")]
    std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
