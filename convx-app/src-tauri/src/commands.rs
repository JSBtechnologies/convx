use convx::{
    license, license::enterprise::ConversionAuditEvent, silent_command, ConversionOptions,
    ConversionStatus, ConvxEngine, DependencyChecker, Format, FormatCategory,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, State, Window};

pub struct ConvxState {
    pub engine: ConvxEngine,
    pub cancel_flag: Arc<AtomicBool>,
}

fn sensitive_roots() -> Vec<PathBuf> {
    if cfg!(target_os = "windows") {
        vec![
            PathBuf::from(r"C:\Windows"),
            PathBuf::from(r"C:\Program Files"),
            PathBuf::from(r"C:\Program Files (x86)"),
        ]
    } else {
        vec![
            PathBuf::from("/System"),
            PathBuf::from("/Library"),
            PathBuf::from("/private/etc"),
            PathBuf::from("/etc"),
            PathBuf::from("/bin"),
            PathBuf::from("/sbin"),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/usr/sbin"),
            PathBuf::from("/var/db"),
            PathBuf::from("/dev"),
        ]
    }
}

fn allowed_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();

    // HOME (macOS/Linux) or USERPROFILE (Windows)
    for var in &["HOME", "USERPROFILE"] {
        if let Ok(home) = std::env::var(var) {
            if let Ok(home_canon) = dunce::canonicalize(&home) {
                roots.push(home_canon);
            }
        }
    }

    if let Ok(temp_canon) = dunce::canonicalize(std::env::temp_dir()) {
        roots.push(temp_canon);
    }

    if cfg!(target_os = "windows") {
        // Allow all drive roots (D:\, E:\, etc.) so users can access files anywhere
        for letter in b'A'..=b'Z' {
            let drive = PathBuf::from(format!("{}:\\", letter as char));
            if drive.exists() {
                roots.push(drive);
            }
        }
    } else {
        let volumes = PathBuf::from("/Volumes");
        if volumes.exists() {
            roots.push(volumes);
        }
    }

    roots
}

fn ensure_allowed_path(path: PathBuf) -> Result<PathBuf, String> {
    if sensitive_roots().iter().any(|root| path.starts_with(root)) {
        return Err(format!(
            "Access to sensitive path is not allowed: {}",
            path.display()
        ));
    }

    let roots = allowed_roots();
    if roots.is_empty() {
        return Err("No allowed filesystem roots configured".to_string());
    }

    if roots.iter().any(|root| path.starts_with(root)) {
        Ok(path)
    } else {
        Err(format!(
            "Path is outside allowed directories: {}",
            path.display()
        ))
    }
}

fn resolve_existing_path(path: &str) -> Result<PathBuf, String> {
    let raw = PathBuf::from(path);
    if !raw.is_absolute() {
        return Err(format!("Path must be absolute: {}", path));
    }

    let canonical = dunce::canonicalize(&raw)
        .map_err(|e| format!("Invalid path {}: {}", raw.display(), e))?;
    ensure_allowed_path(canonical)
}

fn resolve_output_path(path: &str) -> Result<PathBuf, String> {
    let raw = PathBuf::from(path);
    if !raw.is_absolute() {
        return Err(format!("Path must be absolute: {}", path));
    }

    if raw.exists() {
        let canonical = dunce::canonicalize(&raw)
            .map_err(|e| format!("Invalid output path {}: {}", raw.display(), e))?;
        return ensure_allowed_path(canonical);
    }

    let file_name = raw
        .file_name()
        .ok_or_else(|| format!("Invalid output path: {}", raw.display()))?
        .to_owned();

    let parent = raw
        .parent()
        .ok_or_else(|| format!("Output path has no parent: {}", raw.display()))?;

    let canonical_parent = dunce::canonicalize(parent)
        .map_err(|e| format!("Output parent directory is invalid: {}", e))?;
    let canonical_output = canonical_parent.join(file_name);

    ensure_allowed_path(canonical_output)
}

#[derive(Deserialize)]
pub struct JsConversionOptions {
    pub output_format: String,
    pub quality: Option<u8>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub overwrite: Option<bool>,
}

#[derive(Clone, Serialize)]
pub struct JsConversionProgress {
    pub stage: String,
    pub percent: u8,
    pub message: Option<String>,
}

#[derive(Serialize)]
pub struct JsConversionResult {
    pub id: String,
    pub status: String,
    #[serde(rename = "inputPath")]
    pub input_path: String,
    #[serde(rename = "outputPath")]
    pub output_path: Option<String>,
    #[serde(rename = "inputFormat")]
    pub input_format: String,
    #[serde(rename = "outputFormat")]
    pub output_format: String,
    #[serde(rename = "inputSize")]
    pub input_size: u64,
    #[serde(rename = "outputSize")]
    pub output_size: Option<u64>,
    #[serde(rename = "spaceSaved")]
    pub space_saved: Option<i64>,
    #[serde(rename = "durationMs")]
    pub duration_ms: u64,
    pub error: Option<String>,
    pub timestamp: String,
}

#[derive(Serialize)]
pub struct JsFileInfo {
    pub name: String,
    pub size: u64,
    pub extension: String,
}

#[derive(Serialize)]
pub struct JsDependencyStatus {
    pub ok: bool,
    pub message: String,
}

#[derive(Serialize)]
pub struct JsPostInstallStatus {
    pub ok: bool,
    pub repairs: Vec<String>,
}

#[tauri::command]
pub async fn convert_file(
    window: Window,
    state: State<'_, ConvxState>,
    input: String,
    output: String,
    options: JsConversionOptions,
) -> Result<JsConversionResult, String> {
    state.cancel_flag.store(false, Ordering::SeqCst);

    let _ = window.emit(
        "conversion-progress",
        JsConversionProgress {
            stage: "reading".to_string(),
            percent: 5,
            message: Some("Reading input file".to_string()),
        },
    );

    let input_path = resolve_existing_path(&input)?;
    let output_path = resolve_output_path(&output)?;

    let output_format = Format::from_extension(&options.output_format)
        .ok_or_else(|| format!("Unknown format: {}", options.output_format))?;

    let conv_options = ConversionOptions {
        output_format,
        quality: options.quality,
        image: if options.width.is_some() || options.height.is_some() {
            Some(convx::ImageOptions {
                width: options.width,
                height: options.height,
                ..Default::default()
            })
        } else {
            None
        },
        overwrite: options.overwrite.unwrap_or(false),
        ..Default::default()
    };

    let mut on_progress = |progress: f32| {
        let pct = (progress.clamp(0.0, 1.0) * 100.0).round() as u8;
        let _ = window.emit(
            "conversion-progress",
            JsConversionProgress {
                stage: "converting".to_string(),
                percent: pct,
                message: Some(format!("Converting... {}%", pct)),
            },
        );
    };

    let result = state
        .engine
        .convert_with_progress(
            input_path.as_path(),
            output_path.as_path(),
            conv_options,
            &mut on_progress,
            Some(state.cancel_flag.as_ref()),
        )
        .map_err(|e| {
            let _ = window.emit(
                "conversion-progress",
                JsConversionProgress {
                    stage: "error".to_string(),
                    percent: 0,
                    message: Some(e.to_string()),
                },
            );
            e.to_string()
        })?;

    let _ = window.emit(
        "conversion-progress",
        JsConversionProgress {
            stage: "complete".to_string(),
            percent: 100,
            message: Some("Conversion complete".to_string()),
        },
    );

    Ok(JsConversionResult {
        id: result.id.to_string(),
        status: match result.status {
            ConversionStatus::Completed => "completed".to_string(),
            ConversionStatus::Failed => "failed".to_string(),
        },
        input_path: result.input_path.to_string_lossy().to_string(),
        output_path: result.output_path.map(|p| p.to_string_lossy().to_string()),
        input_format: result.input_format.extension().to_string(),
        output_format: result.output_format.extension().to_string(),
        input_size: result.input_size,
        output_size: result.output_size,
        space_saved: result.space_saved,
        duration_ms: result.duration_ms,
        error: result.error,
        timestamp: result.timestamp.to_rfc3339(),
    })
}

#[tauri::command]
pub fn cancel_conversion(state: State<'_, ConvxState>) -> bool {
    state.cancel_flag.store(true, Ordering::SeqCst);
    true
}

#[tauri::command]
pub fn can_convert(state: State<'_, ConvxState>, from: String, to: String) -> bool {
    match (Format::from_extension(&from), Format::from_extension(&to)) {
        (Some(f), Some(t)) => state.engine.can_convert(f, t),
        _ => false,
    }
}

#[tauri::command]
pub fn get_supported_formats() -> Vec<String> {
    [
        FormatCategory::Image,
        FormatCategory::Video,
        FormatCategory::Audio,
        FormatCategory::Document,
        FormatCategory::Data,
        FormatCategory::Ebook,
    ]
    .into_iter()
    .flat_map(Format::all_by_category)
    .map(|f| f.extension().to_string())
    .collect()
}

#[tauri::command]
pub fn get_conversion_targets(from: String) -> Vec<String> {
    match Format::from_extension(&from) {
        Some(format) => format
            .convertible_targets()
            .into_iter()
            .map(|f| f.extension().to_string())
            .collect(),
        None => vec![],
    }
}

#[tauri::command]
pub fn check_dependencies() -> JsDependencyStatus {
    // Only gate the app on core dependencies (ffmpeg + vips).
    // Python-based deps (data, formats) are optional and should not
    // block the user from using image/video/audio conversions.
    let mut missing = Vec::new();
    if DependencyChecker::check_ffmpeg().is_err() {
        missing.push("FFmpeg");
    }
    if DependencyChecker::check_vips().is_err() {
        missing.push("libvips");
    }

    if missing.is_empty() {
        JsDependencyStatus {
            ok: true,
            message: DependencyChecker::get_versions(),
        }
    } else {
        JsDependencyStatus {
            ok: false,
            message: format!("Missing core dependencies: {}", missing.join(", ")),
        }
    }
}

/// Maps a category name to the individual dependencies it contains.
fn category_deps(category: &str) -> Vec<&'static str> {
    match category {
        "media" => vec!["ffmpeg", "vips"],
        "document" => vec!["libreoffice", "pandoc", "poppler"],
        "data" => vec!["pip:pandas", "pip:openpyxl", "pip:pyarrow", "pip:numpy", "pip:h5py"],
        "formats" => vec!["pip:weasyprint", "pip:pdf2docx", "pip:mobi"],
        _ => vec![],
    }
}

/// Returns list of missing dependency category names that need installing.
#[tauri::command]
pub fn get_missing_dependencies() -> Vec<String> {
    let mut missing = Vec::new();

    // Media: ffmpeg + vips (core — blocks the app)
    if DependencyChecker::check_ffmpeg().is_err() || DependencyChecker::check_vips().is_err() {
        missing.push("media".to_string());
    }

    // Document: libreoffice + pandoc + poppler (core for document conversions)
    if DependencyChecker::libreoffice_executable().is_none()
        || DependencyChecker::pandoc_executable().is_none()
        || DependencyChecker::pdftoppm_executable().is_none()
    {
        missing.push("document".to_string());
    }

    // Data and formats (Python-based) are optional — only report if Python
    // is actually installed but modules are missing, so the wizard can fix them.
    // If Python itself isn't installed, skip these silently.
    if DependencyChecker::python3_executable().is_some() {
        if !DependencyChecker::python_has_module("pandas")
            || !DependencyChecker::python_has_module("openpyxl")
            || !DependencyChecker::python_has_module("pyarrow")
            || !DependencyChecker::python_has_module("numpy")
            || !DependencyChecker::python_has_module("h5py")
        {
            missing.push("data".to_string());
        }

        if !DependencyChecker::python_has_module("weasyprint")
            || !DependencyChecker::python_has_module("pdf2docx")
            || !DependencyChecker::python_has_module("mobi")
        {
            missing.push("formats".to_string());
        }
    }

    missing
}

/// Installs a single individual dependency by name (e.g. "ffmpeg", "pip:pandas").
fn install_one_dep(name: &str) -> JsDependencyStatus {
    #[cfg(target_os = "macos")]
    {
        if let Some(module) = name.strip_prefix("pip:") {
            // Install into ~/.convx/venv (creates venv if needed, uses bundled wheels)
            return match DependencyChecker::install_pip_module(module) {
                Ok(()) => JsDependencyStatus {
                    ok: true,
                    message: format!("{} installed", module),
                },
                Err(e) => JsDependencyStatus {
                    ok: false,
                    message: e,
                },
            };
        }

        // For non-pip packages, try Homebrew as fallback if available
        let brew = if Path::new("/opt/homebrew/bin/brew").exists() {
            Some("/opt/homebrew/bin/brew")
        } else if Path::new("/usr/local/bin/brew").exists() {
            Some("/usr/local/bin/brew")
        } else {
            None
        };

        if let Some(brew) = brew {
            // Validate package name to prevent installing arbitrary packages
            if !name.chars().all(|c| {
                c.is_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '+' || c == '@'
            }) {
                return JsDependencyStatus {
                    ok: false,
                    message: format!("Invalid package name: {}", name),
                };
            }
            match silent_command(brew).args(["install", name]).output() {
                Ok(out) if out.status.success() => JsDependencyStatus {
                    ok: true,
                    message: format!("{} installed", name),
                },
                Ok(out) => {
                    let stderr = String::from_utf8_lossy(&out.stderr);
                    if stderr.contains("already installed") {
                        JsDependencyStatus {
                            ok: true,
                            message: format!("{} already installed", name),
                        }
                    } else {
                        JsDependencyStatus {
                            ok: false,
                            message: format!("brew install {} failed: {}", name, stderr),
                        }
                    }
                }
                Err(e) => JsDependencyStatus {
                    ok: false,
                    message: format!("Could not run brew: {}", e),
                },
            }
        } else {
            // No Homebrew — deps should be bundled; suggest reinstalling the app
            JsDependencyStatus {
                ok: false,
                message: format!(
                    "{} is missing and Homebrew is not available. \
                     Try reinstalling convx from the .pkg installer, \
                     or install Homebrew (https://brew.sh) and run: brew install {}",
                    name, name
                ),
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(module) = name.strip_prefix("pip:") {
            return match DependencyChecker::install_pip_module(module) {
                Ok(()) => JsDependencyStatus {
                    ok: true,
                    message: format!("{} installed", module),
                },
                Err(e) => JsDependencyStatus {
                    ok: false,
                    message: e,
                },
            };
        }

        // Validate package name to prevent command injection
        if !name.chars().all(|c| {
            c.is_alphanumeric() || c == '-' || c == '.' || c == '_' || c == '+' || c == '@'
        }) {
            return JsDependencyStatus {
                ok: false,
                message: format!("Invalid package name: {}", name),
            };
        }

        let (cmd, args): (&str, Vec<&str>) =
            if silent_command("apt-get").arg("--version").output().is_ok() {
                ("sudo", vec!["apt-get", "install", "-y", name])
            } else if silent_command("dnf").arg("--version").output().is_ok() {
                ("sudo", vec!["dnf", "install", "-y", name])
            } else if silent_command("pacman").arg("--version").output().is_ok() {
                ("sudo", vec!["pacman", "-S", "--noconfirm", name])
            } else {
                return JsDependencyStatus {
                    ok: false,
                    message: "No supported package manager found.".to_string(),
                };
            };

        match silent_command(cmd).args(&args).output() {
            Ok(out) if out.status.success() => JsDependencyStatus {
                ok: true,
                message: format!("{} installed", name),
            },
            Ok(out) => JsDependencyStatus {
                ok: false,
                message: format!(
                    "Install {} failed: {}",
                    name,
                    String::from_utf8_lossy(&out.stderr)
                ),
            },
            Err(e) => JsDependencyStatus {
                ok: false,
                message: format!("Install {} failed: {}", name, e),
            },
        }
    }

    #[cfg(target_os = "windows")]
    {
        let winget_ids: std::collections::HashMap<&str, &str> = [
            ("ffmpeg", "Gyan.FFmpeg"),
            ("vips", "libvips.libvips"),
            ("libreoffice", "TheDocumentFoundation.LibreOffice"),
            ("pandoc", "JohnMacFarlane.Pandoc"),
        ]
        .into();

        if let Some(module) = name.strip_prefix("pip:") {
            // Ensure Python is installed before attempting pip modules
            if DependencyChecker::python3_executable().is_none() {
                let _ = silent_command("winget")
                    .args([
                        "install", "-e", "--id", "Python.Python.3.13",
                        "--accept-package-agreements", "--accept-source-agreements",
                    ])
                    .output();
            }
            return match DependencyChecker::install_pip_module(module) {
                Ok(()) => JsDependencyStatus {
                    ok: true,
                    message: format!("{} installed", module),
                },
                Err(e) => JsDependencyStatus {
                    ok: false,
                    message: e,
                },
            };
        }

        if let Some(winget_id) = winget_ids.get(name) {
            match silent_command("winget")
                .args([
                    "install",
                    "-e",
                    "--id",
                    winget_id,
                    "--accept-package-agreements",
                    "--accept-source-agreements",
                ])
                .output()
            {
                Ok(out) if out.status.success() => JsDependencyStatus {
                    ok: true,
                    message: format!("{} installed", name),
                },
                Ok(out) => JsDependencyStatus {
                    ok: false,
                    message: format!(
                        "winget install {} failed: {}",
                        name,
                        String::from_utf8_lossy(&out.stderr)
                    ),
                },
                Err(e) => JsDependencyStatus {
                    ok: false,
                    message: format!("winget failed: {}", e),
                },
            }
        } else if name == "poppler" {
            // Poppler has no winget package — try chocolatey
            match silent_command("choco")
                .args(["install", "-y", "poppler"])
                .output()
            {
                Ok(out) if out.status.success() => JsDependencyStatus {
                    ok: true,
                    message: "poppler installed via chocolatey".to_string(),
                },
                _ => JsDependencyStatus {
                    ok: false,
                    message: "poppler: no winget package available, install via chocolatey or bundled installer".to_string(),
                },
            }
        } else {
            JsDependencyStatus {
                ok: false,
                message: format!("Unknown package: {}", name),
            }
        }
    }
}

/// Installs a dependency by name or category. Categories are expanded into
/// their individual deps. Returns ok/message.
#[tauri::command]
pub fn install_single_dependency(name: String) -> JsDependencyStatus {
    let deps = category_deps(&name);

    // If category_deps returned empty, treat `name` as an individual dep name
    if deps.is_empty() {
        return install_one_dep(&name);
    }

    let mut errors = Vec::new();

    for dep in deps {
        let result = install_one_dep(dep);
        if !result.ok {
            errors.push(result.message);
        }
    }

    if errors.is_empty() {
        JsDependencyStatus {
            ok: true,
            message: format!("{} ready", name),
        }
    } else {
        JsDependencyStatus {
            ok: false,
            message: errors.join("; "),
        }
    }
}

/// Ensures post-install setup is complete (CLI symlinks, venv, bundled wheel install).
/// Called on app launch to catch cases where the .pkg postinstall script didn't finish.
#[tauri::command]
pub fn ensure_post_install() -> JsPostInstallStatus {
    let mut repairs = Vec::new();

    #[cfg(target_os = "macos")]
    {
        // Detect where the app is actually running from (unified binary)
        let exe_path = std::env::current_exe().ok();
        let macos_dir = exe_path
            .as_ref()
            .and_then(|e| e.parent().map(|p| p.to_path_buf()));
        let contents_dir = macos_dir
            .as_ref()
            .and_then(|m| m.parent().map(|p| p.to_path_buf()));
        // convx-cli and convx-mcp are symlinks to the main binary
        let cli_bin = macos_dir.as_ref().map(|m| m.join("convx-cli"));
        let mcp_bin = macos_dir.as_ref().map(|m| m.join("convx-mcp"));

        // Find the wheels directory relative to wherever the app is running
        let wheels_dir = contents_dir
            .as_ref()
            .map(|c| c.join("Resources").join("wheels"));

        // Ensure in-bundle symlinks exist (convx-cli -> main binary)
        if let (Some(ref exe), Some(ref cli)) = (&exe_path, &cli_bin) {
            if !cli.exists() && !cli.is_symlink() {
                let exe_name = exe
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let _ = std::os::unix::fs::symlink(&exe_name, cli);
            }
        }
        if let (Some(ref exe), Some(ref mcp)) = (&exe_path, &mcp_bin) {
            if !mcp.exists() && !mcp.is_symlink() {
                let exe_name = exe
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let _ = std::os::unix::fs::symlink(&exe_name, mcp);
            }
        }

        // 1. Check CLI symlink in /usr/local/bin
        if let Some(ref cli) = cli_bin {
            if cli.exists() || cli.is_symlink() {
                let symlink_path = Path::new("/usr/local/bin/convx");
                let needs_symlink = if symlink_path.is_symlink() {
                    // Symlink exists but might point to wrong target
                    std::fs::read_link(symlink_path).ok().as_deref() != Some(cli.as_path())
                } else {
                    !symlink_path.exists()
                };

                if needs_symlink {
                    // Try /usr/local/bin first (may need root)
                    let _ = std::fs::remove_file(symlink_path); // remove stale symlink
                    match std::os::unix::fs::symlink(cli, symlink_path) {
                        Ok(()) => {
                            repairs.push("Created CLI symlink: /usr/local/bin/convx".to_string())
                        }
                        Err(_) => {
                            // Fall back to ~/.local/bin (no root needed)
                            if let Ok(home) = std::env::var("HOME") {
                                let local_bin = PathBuf::from(&home).join(".local").join("bin");
                                let _ = std::fs::create_dir_all(&local_bin);
                                let local_symlink = local_bin.join("convx");
                                let _ = std::fs::remove_file(&local_symlink);
                                match std::os::unix::fs::symlink(cli, &local_symlink) {
                                    Ok(()) => {
                                        repairs.push(format!(
                                            "Created CLI symlink: {}",
                                            local_symlink.display()
                                        ));
                                        ensure_path_entry(&home, &local_bin, &mut repairs);
                                    }
                                    Err(e) => {
                                        repairs.push(format!("Could not create CLI symlink: {}", e))
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. Check MCP symlink
        if let Some(ref mcp) = mcp_bin {
            if mcp.exists() {
                let symlink_path = Path::new("/usr/local/bin/convx-mcp");
                let needs_symlink = if symlink_path.is_symlink() {
                    std::fs::read_link(symlink_path).ok().as_deref() != Some(mcp.as_path())
                } else {
                    !symlink_path.exists()
                };

                if needs_symlink {
                    let _ = std::fs::remove_file(symlink_path);
                    match std::os::unix::fs::symlink(mcp, symlink_path) {
                        Ok(()) => repairs
                            .push("Created MCP symlink: /usr/local/bin/convx-mcp".to_string()),
                        Err(_) => {
                            if let Ok(home) = std::env::var("HOME") {
                                let local_bin = PathBuf::from(&home).join(".local").join("bin");
                                let _ = std::fs::create_dir_all(&local_bin);
                                let local_symlink = local_bin.join("convx-mcp");
                                let _ = std::fs::remove_file(&local_symlink);
                                match std::os::unix::fs::symlink(mcp, &local_symlink) {
                                    Ok(()) => repairs.push(format!(
                                        "Created MCP symlink: {}",
                                        local_symlink.display()
                                    )),
                                    Err(e) => {
                                        repairs.push(format!("Could not create MCP symlink: {}", e))
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 3. Ensure ~/.convx/venv exists and has required modules
        if let Some(venv_dir) = DependencyChecker::convx_venv_dir() {
            if !venv_dir.join("bin").join("python3").exists() {
                if let Some(py) = DependencyChecker::convx_python() {
                    let _ = std::fs::create_dir_all(venv_dir.parent().unwrap_or(Path::new(".")));
                    match silent_command(&py)
                        .args(["-m", "venv", &venv_dir.to_string_lossy()])
                        .output()
                    {
                        Ok(out) if out.status.success() => {
                            repairs.push("Created Python venv at ~/.convx/venv".to_string());
                        }
                        _ => {
                            repairs.push("Python venv missing — could not auto-create".to_string());
                        }
                    }
                }
            }

            // Install missing pip modules from bundled wheels
            let pip = venv_dir.join("bin").join("pip3");
            if pip.exists() {
                let modules = [
                    "pandas",
                    "openpyxl",
                    "weasyprint",
                    "pdf2docx",
                    "PyMuPDF",
                    "mobi",
                    "pyarrow",
                    "numpy",
                    "h5py",
                ];
                let missing_modules: Vec<&str> = modules
                    .iter()
                    .filter(|m| !DependencyChecker::python_has_module(m))
                    .copied()
                    .collect();

                if !missing_modules.is_empty() {
                    let mut cmd = silent_command(&pip);
                    cmd.arg("install");
                    if let Some(ref wd) = wheels_dir {
                        if wd.exists() {
                            cmd.args(["--find-links", &wd.to_string_lossy()]);
                        }
                    }
                    cmd.args(&missing_modules);
                    match cmd.output() {
                        Ok(out) if out.status.success() => {
                            repairs.push(format!(
                                "Installed missing modules: {}",
                                missing_modules.join(", ")
                            ));
                        }
                        _ => {
                            repairs.push(format!(
                                "Could not auto-install modules: {}",
                                missing_modules.join(", ")
                            ));
                        }
                    }
                }
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        // Install missing pip modules using bundled Python directly (no venv)
        let bundled_pip = DependencyChecker::bundled_pip_path();
        let pip_path = bundled_pip.or_else(|| DependencyChecker::convx_pip());

        if let Some(pip) = pip_path {
            let modules = [
                "pandas", "openpyxl", "weasyprint", "pdf2docx", "PyMuPDF",
                "mobi", "pyarrow", "numpy", "h5py",
            ];
            let missing_modules: Vec<&str> = modules
                .iter()
                .filter(|m| !DependencyChecker::python_has_module(m))
                .copied()
                .collect();

            if !missing_modules.is_empty() {
                let mut cmd = silent_command(&pip);
                cmd.arg("install");
                if let Some(wheels) = DependencyChecker::bundled_wheels_dir() {
                    cmd.args(["--find-links", &wheels.to_string_lossy()]);
                }
                cmd.args(&missing_modules);
                match cmd.output() {
                    Ok(out) if out.status.success() => {
                        repairs.push(format!(
                            "Installed missing modules: {}",
                            missing_modules.join(", ")
                        ));
                    }
                    _ => {
                        repairs.push(format!(
                            "Could not auto-install modules: {}",
                            missing_modules.join(", ")
                        ));
                    }
                }
            }
        }
    }

    JsPostInstallStatus {
        ok: repairs.is_empty() || repairs.iter().all(|r: &String| !r.starts_with("Could not")),
        repairs,
    }
}

/// Ensures ~/.local/bin is in the user's PATH by appending to shell profile if needed.
#[cfg(target_os = "macos")]
fn ensure_path_entry(home: &str, dir: &Path, repairs: &mut Vec<String>) {
    let dir_str = dir.to_string_lossy();
    // Check if already in PATH
    if let Ok(path) = std::env::var("PATH") {
        if path.split(':').any(|p| p == dir_str.as_ref()) {
            return;
        }
    }
    // Append to .zshrc (default macOS shell)
    let zshrc = PathBuf::from(home).join(".zshrc");
    let export_line = format!("\n# Added by convx\nexport PATH=\"{}:$PATH\"\n", dir_str);
    if let Ok(contents) = std::fs::read_to_string(&zshrc) {
        if contents.contains(&dir_str.to_string()) {
            return; // Already present
        }
    }
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&zshrc)
    {
        Ok(mut f) => {
            use std::io::Write;
            let _ = f.write_all(export_line.as_bytes());
            repairs.push(format!("Added {} to PATH in ~/.zshrc", dir_str));
        }
        Err(_) => {
            repairs.push(format!(
                "Add to your PATH: export PATH=\"{}:$PATH\"",
                dir_str
            ));
        }
    }
}

/// Legacy: install all at once (kept for compatibility)
#[tauri::command]
pub fn install_dependencies() -> JsDependencyStatus {
    if DependencyChecker::check_all().is_ok() {
        return JsDependencyStatus {
            ok: true,
            message: "Dependencies already installed".to_string(),
        };
    }
    JsDependencyStatus {
        ok: false,
        message: "Use per-package install flow instead".to_string(),
    }
}

#[tauri::command]
pub fn get_file_info(path: String) -> Result<JsFileInfo, String> {
    let resolved = resolve_existing_path(&path)?;
    let p = resolved.as_path();
    let metadata = std::fs::metadata(p).map_err(|e| e.to_string())?;
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let extension = p
        .extension()
        .map(|e| e.to_string_lossy().to_string())
        .unwrap_or_default();

    Ok(JsFileInfo {
        name,
        size: metadata.len(),
        extension,
    })
}

#[tauri::command]
pub fn path_exists(path: String) -> bool {
    resolve_output_path(&path)
        .map(|p| p.exists())
        .unwrap_or(false)
}

#[tauri::command]
pub fn reveal_in_file_manager(path: String) -> Result<(), String> {
    let resolved = resolve_existing_path(&path)?;
    let p = resolved.as_path();
    if !p.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    #[cfg(target_os = "macos")]
    {
        let status = silent_command("open")
            .arg("-R")
            .arg(&resolved)
            .status()
            .map_err(|e| format!("Failed to launch Finder: {}", e))?;

        if !status.success() {
            return Err("Finder failed to reveal file".to_string());
        }
    }

    #[cfg(target_os = "windows")]
    {
        let status = silent_command("explorer")
            .arg("/select,")
            .arg(&resolved)
            .status()
            .map_err(|e| format!("Failed to launch Explorer: {}", e))?;

        if !status.success() {
            return Err("Explorer failed to reveal file".to_string());
        }
    }

    #[cfg(target_os = "linux")]
    {
        let dir = p.parent().unwrap_or(p);
        let status = silent_command("xdg-open")
            .arg(dir)
            .status()
            .map_err(|e| format!("Failed to launch file manager: {}", e))?;

        if !status.success() {
            return Err("File manager failed to open".to_string());
        }
    }

    Ok(())
}

// ─── License commands ────────────────────────────────────────────────────

#[derive(Serialize, Default)]
pub struct JsLicenseStatus {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recheck_after: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub days_overdue: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stored_device: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

#[tauri::command]
pub fn check_license() -> JsLicenseStatus {
    match license::check_license() {
        license::LicenseStatus::Valid {
            device_name,
            recheck_after,
        } => JsLicenseStatus {
            status: "valid".into(),
            device_name: Some(device_name),
            recheck_after: Some(recheck_after.to_rfc3339()),
            ..Default::default()
        },
        license::LicenseStatus::GracePeriod { days_overdue } => JsLicenseStatus {
            status: "grace_period".into(),
            days_overdue: Some(days_overdue),
            ..Default::default()
        },
        license::LicenseStatus::Expired => JsLicenseStatus {
            status: "expired".into(),
            ..Default::default()
        },
        license::LicenseStatus::Revoked => JsLicenseStatus {
            status: "revoked".into(),
            ..Default::default()
        },
        license::LicenseStatus::Tampered => JsLicenseStatus {
            status: "tampered".into(),
            ..Default::default()
        },
        license::LicenseStatus::DeviceMismatch { stored_device } => JsLicenseStatus {
            status: "device_mismatch".into(),
            stored_device: Some(stored_device),
            ..Default::default()
        },
        license::LicenseStatus::NotActivated => JsLicenseStatus {
            status: "not_activated".into(),
            ..Default::default()
        },
        license::LicenseStatus::Error(msg) => JsLicenseStatus {
            status: "error".into(),
            error_message: Some(msg),
            ..Default::default()
        },
    }
}

#[derive(Serialize)]
pub struct JsActivateResult {
    pub outcome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[tauri::command]
pub fn activate_license(key: String) -> JsActivateResult {
    match license::activate(&key) {
        Ok(license::ActivateOutcome::Activated { device_name }) => JsActivateResult {
            outcome: "activated".into(),
            device_name: Some(device_name),
            message: None,
        },
        Ok(license::ActivateOutcome::AlreadyActive { device_name }) => JsActivateResult {
            outcome: "already_active".into(),
            device_name: Some(device_name),
            message: None,
        },
        Err(e) => JsActivateResult {
            outcome: "error".into(),
            device_name: None,
            message: Some(e),
        },
    }
}

#[tauri::command]
pub fn transfer_license(key: String) -> Result<bool, String> {
    license::transfer(&key)?;
    Ok(true)
}

#[tauri::command]
pub fn deactivate_license() -> Result<bool, String> {
    license::deactivate()?;
    Ok(true)
}

#[derive(Serialize)]
pub struct JsLicenseInfo {
    pub key_masked: String,
    pub device_name: String,
    pub platform: String,
    pub activated_at: String,
    pub recheck_after: String,
}

#[tauri::command]
pub fn get_license_info() -> Option<JsLicenseInfo> {
    license::license_info().map(|info| JsLicenseInfo {
        key_masked: info.key_masked,
        device_name: info.device_name,
        platform: info.platform,
        activated_at: info.activated_at.to_rfc3339(),
        recheck_after: info.recheck_after.to_rfc3339(),
    })
}

// ─── Enterprise commands ──────────────────────────────────────────────────

#[derive(Serialize)]
pub struct JsEnterpriseConfig {
    pub has_config: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<JsEnterpriseSettings>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct JsEnterpriseSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_quality: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_directory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overwrite_existing: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub show_notifications: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_formats: Option<Vec<String>>,
    #[serde(default)]
    pub locked: bool,
}

#[tauri::command]
pub fn get_enterprise_config() -> JsEnterpriseConfig {
    match license::get_enterprise_config() {
        Some(config) => JsEnterpriseConfig {
            has_config: true,
            org_id: config.org_id,
            settings: config.settings.map(|s| JsEnterpriseSettings {
                default_quality: s.default_quality,
                default_format: s.default_format,
                output_directory: s.output_directory,
                overwrite_existing: s.overwrite_existing,
                show_notifications: s.show_notifications,
                allowed_formats: s.allowed_formats,
                locked: s.locked,
            }),
        },
        None => JsEnterpriseConfig {
            has_config: false,
            org_id: None,
            settings: None,
        },
    }
}

#[tauri::command]
pub fn auto_activate() -> JsActivateResult {
    match license::auto_activate() {
        Some(Ok(license::ActivateOutcome::Activated { device_name })) => JsActivateResult {
            outcome: "activated".into(),
            device_name: Some(device_name),
            message: None,
        },
        Some(Ok(license::ActivateOutcome::AlreadyActive { device_name })) => JsActivateResult {
            outcome: "already_active".into(),
            device_name: Some(device_name),
            message: None,
        },
        Some(Err(e)) => JsActivateResult {
            outcome: "error".into(),
            device_name: None,
            message: Some(e),
        },
        None => JsActivateResult {
            outcome: "no_config".into(),
            device_name: None,
            message: None,
        },
    }
}

// ─── MCP Server commands ──────────────────────────────────────────────────

#[derive(Serialize)]
pub struct JsMcpConfig {
    pub binary_path: String,
    pub claude_desktop: String,
    pub cursor: String,
}

#[tauri::command]
/// Returns the canonical path to the current executable, without the \\?\ UNC prefix on Windows.
fn mcp_binary_path() -> Result<String, String> {
    let exe =
        std::env::current_exe().map_err(|e| format!("Could not determine binary path: {}", e))?;
    let canonical = dunce::canonicalize(&exe).unwrap_or(exe);
    Ok(canonical.to_string_lossy().to_string())
}

#[tauri::command]
pub fn get_mcp_config() -> Result<JsMcpConfig, String> {
    let binary_path = mcp_binary_path()?;

    let config_entry = serde_json::json!({
        "mcpServers": {
            "convx": {
                "command": binary_path,
                "args": ["--mcp"]
            }
        }
    });

    let config_str =
        serde_json::to_string_pretty(&config_entry).unwrap_or_else(|_| "{}".to_string());

    Ok(JsMcpConfig {
        binary_path,
        claude_desktop: config_str.clone(),
        cursor: config_str,
    })
}

#[tauri::command]
pub fn auto_configure_mcp(target: String) -> Result<String, String> {
    let binary_path = mcp_binary_path()?;

    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| "Could not determine home directory".to_string())?;
    let home = PathBuf::from(home);

    let config_path = match target.as_str() {
        "claude-desktop" => {
            #[cfg(target_os = "macos")]
            {
                home.join("Library/Application Support/Claude/claude_desktop_config.json")
            }
            #[cfg(target_os = "windows")]
            {
                std::env::var("APPDATA")
                    .map(|a| PathBuf::from(a).join("Claude/claude_desktop_config.json"))
                    .unwrap_or_else(|_| {
                        home.join("AppData/Roaming/Claude/claude_desktop_config.json")
                    })
            }
            #[cfg(target_os = "linux")]
            {
                home.join(".config/Claude/claude_desktop_config.json")
            }
        }
        "cursor" => home.join(".cursor/mcp.json"),
        _ => {
            return Err(format!(
                "Unknown target: {}. Use 'claude-desktop' or 'cursor'.",
                target
            ))
        }
    };

    // Read existing config or start with empty object
    let mut config: serde_json::Value = if config_path.exists() {
        let contents = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("Could not read config: {}", e))?;
        serde_json::from_str(&contents).map_err(|e| format!("Could not parse config: {}", e))?
    } else {
        serde_json::json!({})
    };

    // Merge convx MCP server entry
    let servers = config
        .as_object_mut()
        .ok_or("Config is not a JSON object")?
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));

    servers
        .as_object_mut()
        .ok_or("mcpServers is not a JSON object")?
        .insert(
            "convx".to_string(),
            serde_json::json!({
                "command": binary_path,
                "args": ["--mcp"]
            }),
        );

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create config directory: {}", e))?;
    }

    let json_bytes =
        serde_json::to_string_pretty(&config).unwrap_or_else(|_| "{}".to_string());

    // Write to a temp file first, then rename — avoids issues if Claude Desktop
    // has the config file open (Windows mandatory file locking).
    let tmp_path = config_path.with_extension("json.tmp");
    std::fs::write(&tmp_path, &json_bytes)
        .map_err(|e| format!("Could not write temp config: {}", e))?;

    // Try rename (atomic on same filesystem). If it fails (file locked),
    // fall back to direct write.
    if std::fs::rename(&tmp_path, &config_path).is_err() {
        let _ = std::fs::remove_file(&tmp_path);
        std::fs::write(&config_path, &json_bytes)
            .map_err(|e| {
                format!(
                    "Could not write config (is Claude Desktop running? Close it and retry): {}",
                    e
                )
            })?;
    }

    Ok(config_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn send_conversion_audit(
    input_format: String,
    output_format: String,
    input_size: u64,
    output_size: u64,
    duration_ms: u64,
) {
    if let Some(config) = license::get_enterprise_config() {
        let event = ConversionAuditEvent {
            input_format,
            output_format,
            input_size,
            output_size,
            duration_ms,
            platform: std::env::consts::OS.to_string(),
            timestamp: {
                // Format as RFC 3339 (UTC) without pulling in chrono
                let dur = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default();
                let secs = dur.as_secs();
                let days = secs / 86400;
                let day_secs = secs % 86400;
                let h = day_secs / 3600;
                let m = (day_secs % 3600) / 60;
                let s = day_secs % 60;

                // Days since 1970-01-01 → year/month/day
                let mut y: i64 = 1970;
                let mut remaining = days as i64;
                loop {
                    let year_days: i64 = if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                        366
                    } else {
                        365
                    };
                    if remaining < year_days {
                        break;
                    }
                    remaining -= year_days;
                    y += 1;
                }
                let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
                let month_days: [i64; 12] = [
                    31,
                    if leap { 29 } else { 28 },
                    31,
                    30,
                    31,
                    30,
                    31,
                    31,
                    30,
                    31,
                    30,
                    31,
                ];
                let mut mo = 0usize;
                while mo < 12 && remaining >= month_days[mo] {
                    remaining -= month_days[mo];
                    mo += 1;
                }
                format!(
                    "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                    y,
                    mo + 1,
                    remaining + 1,
                    h,
                    m,
                    s
                )
            },
        };
        license::enterprise::send_audit_event(&config, event);
    }
}
