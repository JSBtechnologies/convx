use crate::ConvxError;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct DependencyChecker;

impl DependencyChecker {
    /// Returns the bundled deps directory, platform-dependent:
    /// - **macOS:** `<exe>/../../Resources/` (inside .app bundle)
    /// - **Windows:** `<exe_dir>\deps\`
    /// - **Linux:** `<exe_dir>/../libexec/convx-deps/` (AppImage/deb) or `/opt/convx/deps/`
    fn bundled_resources_dir() -> Option<PathBuf> {
        let exe = env::current_exe().ok()?;
        let exe_dir = exe.parent()?;

        #[cfg(target_os = "macos")]
        {
            // exe is at .app/Contents/MacOS/binary — walk up to Contents/Resources/
            let contents_dir = exe_dir.parent()?;
            if contents_dir.ends_with("Contents") {
                let res = contents_dir.join("Resources");
                if res.is_dir() {
                    return Some(res);
                }
            }
            None
        }

        #[cfg(target_os = "windows")]
        {
            // deps\ sits alongside the exe
            let deps = exe_dir.join("deps");
            if deps.is_dir() {
                return Some(deps);
            }
            None
        }

        #[cfg(target_os = "linux")]
        {
            // AppImage / deb: <exe_dir>/../libexec/convx-deps/
            if let Some(parent) = exe_dir.parent() {
                let libexec = parent.join("libexec").join("convx-deps");
                if libexec.is_dir() {
                    return Some(libexec);
                }
            }
            // System install fallback
            let opt = PathBuf::from("/opt/convx/deps");
            if opt.is_dir() {
                return Some(opt);
            }
            None
        }
    }

    /// Appends `.exe` on Windows, returns bare name otherwise.
    fn binary_name(name: &str) -> String {
        if cfg!(windows) {
            format!("{}.exe", name)
        } else {
            name.to_string()
        }
    }

    fn candidate_paths(binary: &str) -> Vec<String> {
        let mut candidates = Vec::new();
        let bin = Self::binary_name(binary);

        // Highest priority: bundled binaries
        if let Some(res) = Self::bundled_resources_dir() {
            candidates.push(res.join("bin").join(&bin).to_string_lossy().to_string());
        }

        // Bare binary name (via PATH lookup)
        candidates.push(binary.to_string());

        // Platform-specific common locations
        #[cfg(target_os = "macos")]
        {
            candidates.push(format!("/opt/homebrew/bin/{}", binary));
            candidates.push(format!("/usr/local/bin/{}", binary));
            candidates.push(format!("/opt/local/bin/{}", binary));
        }

        #[cfg(target_os = "windows")]
        {
            if let Some(pf) = env::var_os("ProgramFiles") {
                let pf = PathBuf::from(pf);
                candidates.push(pf.join(binary).join(&bin).to_string_lossy().to_string());
            }
            if let Some(local) = env::var_os("LOCALAPPDATA") {
                let local = PathBuf::from(local);
                candidates.push(local.join(binary).join(&bin).to_string_lossy().to_string());
            }
        }

        #[cfg(target_os = "linux")]
        {
            candidates.push(format!("/usr/bin/{}", binary));
            candidates.push(format!("/usr/local/bin/{}", binary));
            candidates.push(format!("/snap/bin/{}", binary));
        }

        // Expand current PATH entries explicitly
        if let Some(path_var) = env::var_os("PATH") {
            for p in env::split_paths(&path_var) {
                let full = p.join(&bin);
                candidates.push(full.to_string_lossy().to_string());
            }
        }

        candidates
    }

    fn resolve_binary(binary: &str, version_arg: &str) -> Option<String> {
        for candidate in Self::candidate_paths(binary) {
            if !candidate.contains('/') || Path::new(&candidate).exists() {
                if let Ok(output) = Command::new(&candidate).arg(version_arg).output() {
                    if output.status.success() {
                        return Some(candidate);
                    }
                }
            }
        }
        None
    }

    // ─── Virtual environment helpers ─────────────────────────────────

    /// Returns `~/.convx/venv`
    pub fn convx_venv_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".convx").join("venv"))
    }

    /// Returns the venv Python binary if the venv exists, otherwise system python3.
    pub fn convx_python() -> Option<String> {
        if let Some(venv) = Self::convx_venv_dir() {
            let bin = if cfg!(windows) {
                venv.join("Scripts").join("python.exe")
            } else {
                venv.join("bin").join("python3")
            };
            if bin.exists() {
                return Some(bin.to_string_lossy().to_string());
            }
        }
        Self::python3_executable()
    }

    /// Returns the venv pip binary path, falling back to bundled Python's pip.
    pub fn convx_pip() -> Option<String> {
        if let Some(venv) = Self::convx_venv_dir() {
            let bin = if cfg!(windows) {
                venv.join("Scripts").join("pip.exe")
            } else {
                venv.join("bin").join("pip3")
            };
            if bin.exists() {
                return Some(bin.to_string_lossy().to_string());
            }
        }
        // Fall back to bundled Python's pip (e.g. Windows embeddable distribution)
        Self::bundled_pip_path()
    }

    /// Returns the bundled wheels directory if it exists inside the .app bundle.
    pub fn bundled_wheels_dir() -> Option<PathBuf> {
        let res = Self::bundled_resources_dir()?;
        let wheels = res.join("wheels");
        if wheels.is_dir() {
            Some(wheels)
        } else {
            None
        }
    }

    /// Creates the convx venv if it doesn't exist. Returns the venv dir.
    pub fn ensure_venv() -> Result<PathBuf, String> {
        let venv_dir = Self::convx_venv_dir()
            .ok_or_else(|| "Could not determine home directory".to_string())?;

        // Already exists and has a Python binary
        let py_bin = if cfg!(windows) {
            venv_dir.join("Scripts").join("python.exe")
        } else {
            venv_dir.join("bin").join("python3")
        };

        if py_bin.exists() {
            return Ok(venv_dir);
        }

        // Try bundled Python first, then system python3
        let system_py = Self::python3_executable()
            .ok_or_else(|| "python3 not found — install Python 3 first".to_string())?;

        // Ensure parent dir exists
        if let Some(parent) = venv_dir.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create ~/.convx: {}", e))?;
        }

        let output = Command::new(&system_py)
            .args(["-m", "venv", &venv_dir.to_string_lossy()])
            .output()
            .map_err(|e| format!("Failed to run python3 -m venv: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to create venv: {}", stderr));
        }

        Ok(venv_dir)
    }

    /// Returns bundled pip path if it exists (e.g. Windows bundled installer).
    /// This avoids needing to create a venv with embeddable Python.
    pub fn bundled_pip_path() -> Option<String> {
        if let Some(res) = Self::bundled_resources_dir() {
            let bundled_pip = if cfg!(windows) {
                res.join("python").join("Scripts").join("pip.exe")
            } else {
                res.join("python").join("bin").join("pip3")
            };
            if bundled_pip.exists() {
                return Some(bundled_pip.to_string_lossy().to_string());
            }
        }
        None
    }

    /// Installs a pip module into the convx venv or via bundled Python.
    /// Prefers bundled pip (Windows bundled installer) over venv to avoid
    /// issues with embeddable Python lacking the venv module.
    pub fn install_pip_module(module: &str) -> Result<(), String> {
        // Try bundled pip first (works on Windows embeddable Python)
        let pip = if let Some(bundled_pip) = Self::bundled_pip_path() {
            bundled_pip
        } else {
            // Fall back to venv approach (macOS/Linux, bootstrapper installs)
            Self::ensure_venv()?;
            Self::convx_pip()
                .ok_or_else(|| "pip not found in venv after creation".to_string())?
        };

        let mut args = vec!["install".to_string()];

        // Use bundled wheels for offline install if available
        if let Some(wheels) = Self::bundled_wheels_dir() {
            args.push("--find-links".to_string());
            args.push(wheels.to_string_lossy().to_string());
        }

        args.push(module.to_string());

        let output = Command::new(&pip)
            .args(&args)
            .output()
            .map_err(|e| format!("Failed to run pip install: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("pip install {} failed: {}", module, stderr));
        }

        Ok(())
    }

    pub fn ffmpeg_executable() -> Option<String> {
        Self::resolve_binary("ffmpeg", "-version")
    }

    pub fn ffprobe_executable() -> Option<String> {
        Self::resolve_binary("ffprobe", "-version")
    }

    pub fn vips_executable() -> Option<String> {
        Self::resolve_binary("vips", "--version")
    }

    pub fn libreoffice_executable() -> Option<String> {
        // Check bundled headless LibreOffice first
        if let Some(res) = Self::bundled_resources_dir() {
            #[cfg(target_os = "windows")]
            let bundled = res.join("LibreOffice").join("program").join("soffice.exe");
            #[cfg(not(target_os = "windows"))]
            let bundled = res.join("LibreOffice").join("soffice");
            if bundled.exists() {
                return Some(bundled.to_string_lossy().to_string());
            }
        }

        // Platform-specific standard locations
        #[cfg(target_os = "macos")]
        {
            let macos_lo = "/Applications/LibreOffice.app/Contents/MacOS/soffice";
            if Path::new(macos_lo).exists() {
                return Some(macos_lo.to_string());
            }
        }
        #[cfg(target_os = "windows")]
        {
            for pf_var in &["ProgramFiles", "ProgramFiles(x86)"] {
                if let Some(pf) = env::var_os(pf_var) {
                    let lo = PathBuf::from(pf)
                        .join("LibreOffice")
                        .join("program")
                        .join("soffice.exe");
                    if lo.exists() {
                        return Some(lo.to_string_lossy().to_string());
                    }
                }
            }
        }
        #[cfg(target_os = "linux")]
        {
            let linux_lo = "/usr/lib/libreoffice/program/soffice";
            if Path::new(linux_lo).exists() {
                return Some(linux_lo.to_string());
            }
        }

        Self::resolve_binary("soffice", "--version")
    }

    pub fn pandoc_executable() -> Option<String> {
        Self::resolve_binary("pandoc", "--version")
    }

    pub fn pdftoppm_executable() -> Option<String> {
        Self::resolve_binary("pdftoppm", "-v")
    }

    pub fn python3_executable() -> Option<String> {
        // Check bundled Python first (platform-specific layout)
        if let Some(res) = Self::bundled_resources_dir() {
            #[cfg(target_os = "macos")]
            let bundled = res
                .join("Python.framework")
                .join("Versions")
                .join("Current")
                .join("bin")
                .join("python3");
            #[cfg(target_os = "windows")]
            let bundled = res.join("python").join("python.exe");
            #[cfg(target_os = "linux")]
            let bundled = res.join("python").join("bin").join("python3");
            if bundled.exists() {
                return Some(bundled.to_string_lossy().to_string());
            }
        }
        // On Windows, Python installs as "python" not "python3"
        if cfg!(windows) {
            if let Some(py) = Self::resolve_binary("python", "--version") {
                return Some(py);
            }
        }
        Self::resolve_binary("python3", "--version")
    }

    /// Returns library search paths for native dependencies (GLib, Pango, Cairo, etc.)
    /// needed by WeasyPrint and other Python packages that use ctypes/cffi.
    pub fn native_lib_search_path() -> String {
        let mut paths = Vec::new();
        let sep = Self::path_separator();

        // Bundled libs
        if let Some(res) = Self::bundled_resources_dir() {
            let lib = res.join("lib");
            if lib.is_dir() {
                paths.push(lib.to_string_lossy().to_string());
            }
            // On Windows, DLLs also live in bin/ alongside executables
            #[cfg(target_os = "windows")]
            {
                let bin = res.join("bin");
                if bin.is_dir() {
                    paths.push(bin.to_string_lossy().to_string());
                }
            }
        }

        // Platform-specific system lib paths
        #[cfg(target_os = "macos")]
        {
            if Path::new("/opt/homebrew/lib").is_dir() {
                paths.push("/opt/homebrew/lib".to_string());
            }
            if Path::new("/usr/local/lib").is_dir() {
                paths.push("/usr/local/lib".to_string());
            }
        }
        #[cfg(target_os = "linux")]
        {
            if Path::new("/usr/lib/x86_64-linux-gnu").is_dir() {
                paths.push("/usr/lib/x86_64-linux-gnu".to_string());
            }
            if Path::new("/usr/local/lib").is_dir() {
                paths.push("/usr/local/lib".to_string());
            }
        }

        // Preserve existing library path env var
        let existing_var = Self::lib_path_env_var();
        if let Some(existing) = env::var_os(existing_var) {
            paths.push(existing.to_string_lossy().to_string());
        }

        paths.join(sep)
    }

    /// Returns the environment variable name for native library search paths.
    pub fn lib_path_env_var() -> &'static str {
        if cfg!(target_os = "macos") {
            "DYLD_LIBRARY_PATH"
        } else if cfg!(target_os = "windows") {
            "PATH"
        } else {
            "LD_LIBRARY_PATH"
        }
    }

    /// Returns the path separator for the current platform (":" or ";").
    fn path_separator() -> &'static str {
        if cfg!(windows) {
            ";"
        } else {
            ":"
        }
    }

    /// Sets native library environment variables on a Command for the current platform.
    pub fn set_lib_env(cmd: &mut Command) {
        let lib_path = Self::native_lib_search_path();
        let var = Self::lib_path_env_var();
        cmd.env(var, &lib_path);
        #[cfg(target_os = "macos")]
        {
            cmd.env("DYLD_FALLBACK_LIBRARY_PATH", &lib_path);
        }
    }

    /// Returns the weasyprint CLI binary path (venv first, bundled Python, then system).
    pub fn weasyprint_executable() -> Option<String> {
        if let Some(venv) = Self::convx_venv_dir() {
            let bin = if cfg!(windows) {
                venv.join("Scripts").join("weasyprint.exe")
            } else {
                venv.join("bin").join("weasyprint")
            };
            if bin.exists() {
                return Some(bin.to_string_lossy().to_string());
            }
        }
        // Check bundled Python's scripts (Windows embeddable distribution)
        if let Some(res) = Self::bundled_resources_dir() {
            let bundled = if cfg!(windows) {
                res.join("python").join("Scripts").join("weasyprint.exe")
            } else {
                res.join("python").join("bin").join("weasyprint")
            };
            if bundled.exists() {
                return Some(bundled.to_string_lossy().to_string());
            }
        }
        Self::resolve_binary("weasyprint", "--version")
    }

    pub fn python_has_module(module: &str) -> bool {
        // Check venv Python first, then fall back to system Python
        let Some(py) = Self::convx_python() else {
            return false;
        };

        Command::new(py)
            .args(["-c", "import importlib.util,sys; sys.exit(0 if importlib.util.find_spec(sys.argv[1]) else 1)"])
            .arg(module)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Check if FFmpeg is installed
    pub fn check_ffmpeg() -> Result<(), ConvxError> {
        if Self::ffmpeg_executable().is_some() {
            Ok(())
        } else {
            Err(ConvxError::FfmpegNotFound)
        }
    }

    /// Check if libvips is installed
    pub fn check_vips() -> Result<(), ConvxError> {
        if Self::vips_executable().is_some() {
            Ok(())
        } else {
            Err(ConvxError::VipsNotFound)
        }
    }

    /// Check all dependencies and return a helpful error message
    pub fn check_all() -> Result<(), String> {
        let mut missing = Vec::new();

        if Self::check_ffmpeg().is_err() {
            missing.push("FFmpeg");
        }

        if Self::check_vips().is_err() {
            missing.push("libvips");
        }

        if Self::libreoffice_executable().is_none() {
            missing.push("LibreOffice (soffice)");
        }

        if Self::pandoc_executable().is_none() {
            missing.push("pandoc");
        }

        if Self::pdftoppm_executable().is_none() {
            missing.push("poppler (pdftoppm)");
        }

        if Self::python3_executable().is_none() {
            missing.push("python3");
        } else {
            if !Self::python_has_module("pandas") {
                missing.push("python module pandas");
            }
            if !Self::python_has_module("openpyxl") {
                missing.push("python module openpyxl");
            }
            if !Self::python_has_module("weasyprint") {
                missing.push("python module weasyprint");
            }
            if !Self::python_has_module("pdf2docx") {
                missing.push("python module pdf2docx");
            }
            if !Self::python_has_module("mobi") {
                missing.push("python module mobi");
            }
            if !Self::python_has_module("pyarrow") {
                missing.push("python module pyarrow");
            }
            if !Self::python_has_module("numpy") {
                missing.push("python module numpy");
            }
            if !Self::python_has_module("h5py") {
                missing.push("python module h5py");
            }
        }

        if missing.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "Missing dependencies: {}\n\n\
                Install all dependencies:\n\
                \n\
                On macOS:\n\
                  brew install ffmpeg vips pandoc poppler python@3 && brew install --cask libreoffice\n\
                  python3 -m venv ~/.convx/venv\n\
                  ~/.convx/venv/bin/pip install pandas openpyxl weasyprint \"pdf2docx\" \"PyMuPDF==1.23.26\" mobi pyarrow numpy h5py\n\
                \n\
                On Debian/Ubuntu:\n\
                  sudo apt-get install ffmpeg libvips-tools pandoc poppler-utils libreoffice-core libreoffice-writer libreoffice-calc libreoffice-impress python3 python3-venv\n\
                  python3 -m venv ~/.convx/venv\n\
                  ~/.convx/venv/bin/pip install pandas openpyxl weasyprint \"pdf2docx\" \"PyMuPDF==1.23.26\" mobi pyarrow numpy h5py",
                missing.join(", ")
            ))
        }
    }

    /// Get version information for installed dependencies
    pub fn get_versions() -> String {
        let mut versions = Vec::new();

        // FFmpeg version
        if let Some(ffmpeg) = Self::ffmpeg_executable() {
            if let Ok(output) = Command::new(ffmpeg).arg("-version").output() {
                if let Some(first_line) = String::from_utf8_lossy(&output.stdout).lines().next() {
                    versions.push(format!("FFmpeg: {}", first_line));
                }
            }
        }

        // libvips version
        if let Some(vips) = Self::vips_executable() {
            if let Ok(output) = Command::new(vips).arg("--version").output() {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                versions.push(format!("libvips: {}", version));
            }
        }

        if let Some(soffice) = Self::libreoffice_executable() {
            if let Ok(output) = Command::new(soffice).arg("--version").output() {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !version.is_empty() {
                    versions.push(format!("LibreOffice: {}", version));
                }
            }
        }

        if let Some(pandoc) = Self::pandoc_executable() {
            if let Ok(output) = Command::new(pandoc).arg("--version").output() {
                if let Some(first_line) = String::from_utf8_lossy(&output.stdout).lines().next() {
                    versions.push(format!("Pandoc: {}", first_line));
                }
            }
        }

        if let Some(pdftoppm) = Self::pdftoppm_executable() {
            if let Ok(output) = Command::new(pdftoppm).arg("-v").output() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if let Some(first_line) = stderr.lines().next() {
                    versions.push(format!("Poppler: {}", first_line));
                }
            }
        }

        if let Some(py) = Self::python3_executable() {
            if let Ok(output) = Command::new(py).arg("--version").output() {
                let mut version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if version.is_empty() {
                    version = String::from_utf8_lossy(&output.stderr).trim().to_string();
                }
                if !version.is_empty() {
                    versions.push(format!("Python: {}", version));
                }
            }

            versions.push(format!(
                "Python modules: pandas={}, openpyxl={}, weasyprint={}, pdf2docx={}, mobi={}, pyarrow={}, numpy={}, h5py={}",
                Self::python_has_module("pandas"),
                Self::python_has_module("openpyxl"),
                Self::python_has_module("weasyprint"),
                Self::python_has_module("pdf2docx"),
                Self::python_has_module("mobi"),
                Self::python_has_module("pyarrow"),
                Self::python_has_module("numpy"),
                Self::python_has_module("h5py"),
            ));
        }

        if versions.is_empty() {
            "No dependencies found".to_string()
        } else {
            versions.join("\n")
        }
    }
}
