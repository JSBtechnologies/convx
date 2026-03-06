/// Known CLI subcommands from convx::cli (clap Commands enum).
const CLI_SUBCOMMANDS: &[&str] = &[
    "convert", "presets", "formats", "check", "info", "watch",
    "mcp", "activate", "deactivate", "license", "fingerprint", "version",
];

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Use args[0] for exe name detection (current_exe() resolves symlinks)
    let exe_name = args
        .first()
        .and_then(|a| std::path::Path::new(a).file_name())
        .and_then(|n| n.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // --- MCP mode ---
    // Triggered by --mcp flag or exe name convx-mcp
    let is_mcp = args.iter().any(|a| a == "--mcp")
        || exe_name == "convx-mcp"
        || exe_name == "convx-mcp.exe";

    if is_mcp {
        if let Err(e) = convx_app_lib::run_mcp() {
            eprintln!("MCP server error: {}", e);
            std::process::exit(1);
        }
        return;
    }

    // --- CLI mode ---
    // Triggered by: exe name convx-cli, or first positional arg is a known subcommand,
    // or --help/-h/--version/-V flags (show CLI help instead of launching GUI)
    let first_positional = args.iter().skip(1).find(|a| !a.starts_with('-'));
    let has_cli_subcommand = first_positional
        .map(|s| CLI_SUBCOMMANDS.contains(&s.as_str()))
        .unwrap_or(false);
    let has_help_or_version = args.iter().skip(1).any(|a| {
        a == "--help" || a == "-h" || a == "--version" || a == "-V"
    });

    let is_cli = exe_name == "convx-cli"
        || exe_name == "convx-cli.exe"
        || has_cli_subcommand
        || has_help_or_version;

    if is_cli {
        if let Err(e) = convx::cli::cli_main() {
            eprintln!("Error: {:#}", e);
            std::process::exit(1);
        }
        return;
    }

    // --- GUI mode (default) ---
    #[cfg(all(windows, not(debug_assertions)))]
    unsafe {
        windows_sys::Win32::System::Console::FreeConsole();
    }

    convx_app_lib::run_gui()
}
