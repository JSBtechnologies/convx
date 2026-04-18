mod commands;

use commands::ConvxState;
use std::sync::Mutex;

/// Launch the desktop GUI (default mode).
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run_gui() {
    let engine = convx::ConvxEngine::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .manage(ConvxState {
            engine,
            cancel_flag: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            commands::convert_file,
            commands::cancel_conversion,
            commands::can_convert,
            commands::get_supported_formats,
            commands::get_conversion_targets,
            commands::check_dependencies,
            commands::get_missing_dependencies,
            commands::install_single_dependency,
            commands::install_dependencies,
            commands::ensure_post_install,
            commands::get_file_info,
            commands::path_exists,
            commands::reveal_in_file_manager,
            commands::get_mcp_config,
            commands::auto_configure_mcp,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Run the MCP (Model Context Protocol) server on stdin/stdout.
pub fn run_mcp() -> anyhow::Result<()> {
    convx::mcp_server::run_stdio_server()
}
