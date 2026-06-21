use base64::Engine;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

struct AppState {
    backend: Mutex<Option<BackendRuntime>>,
}

struct BackendRuntime {
    child: Child,
    port: u16,
    python: String,
    token: String,
}

#[derive(Debug, Serialize)]
struct BackendProbe {
    ok: bool,
    python: Option<String>,
    version: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct BackendProcess {
    pid: u32,
    python: String,
    url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HermesConnection {
    auth_mode: String,
    base_url: String,
    is_fullscreen: bool,
    logs: Vec<String>,
    mode: String,
    native_overlay_width: u16,
    source: String,
    token: String,
    window_button_position: Option<serde_json::Value>,
    ws_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiRequest {
    path: String,
    method: Option<String>,
    body: Option<serde_json::Value>,
    timeout_ms: Option<u64>,
    #[allow(dead_code)]
    profile: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BootProgress {
    error: Option<String>,
    fake_mode: bool,
    message: String,
    phase: String,
    progress: u8,
    running: bool,
    timestamp: u128,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadFileTextResult {
    binary: bool,
    byte_size: u64,
    language: Option<String>,
    mime_type: String,
    path: String,
    text: String,
    truncated: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadDirEntry {
    is_directory: bool,
    name: String,
    path: String,
}

#[derive(Debug, Serialize)]
struct ReadDirResult {
    entries: Vec<ReadDirEntry>,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct SanitizedCwd {
    cwd: String,
    sanitized: bool,
}

#[tauri::command]
fn backend_probe() -> BackendProbe {
    match find_python() {
        Some(python) => match python_version(&python) {
            Ok(version) => match can_import_hermes(&python) {
                Ok(()) => BackendProbe {
                    ok: true,
                    python: Some(python.to_string_lossy().into_owned()),
                    version: Some(version),
                    error: None,
                },
                Err(error) => BackendProbe {
                    ok: false,
                    python: Some(python.to_string_lossy().into_owned()),
                    version: Some(version),
                    error: Some(error),
                },
            },
            Err(error) => BackendProbe {
                ok: false,
                python: Some(python.to_string_lossy().into_owned()),
                version: None,
                error: Some(error),
            },
        },
        None => BackendProbe {
            ok: false,
            python: None,
            version: None,
            error: Some("Python 3.11-3.14 не найден".to_string()),
        },
    }
}

#[tauri::command]
fn backend_version() -> Result<String, String> {
    let python = find_python().ok_or_else(|| "Python 3.11-3.14 не найден".to_string())?;
    let output = Command::new(&python)
        .args(["-m", "hermes_cli.main", "version"])
        .output()
        .map_err(|error| format!("Не удалось запустить Hermes backend: {error}"))?;

    if !output.status.success() {
        return Err(command_error("hermes version", &output));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[tauri::command]
fn start_backend(
    state: tauri::State<'_, AppState>,
    _host: Option<String>,
    port: Option<u16>,
) -> Result<BackendProcess, String> {
    let connection = ensure_backend(&state, port)?;

    Ok(BackendProcess {
        pid: connection.pid,
        python: connection.python,
        url: format!(
            "http://{}",
            connection.base_url.trim_start_matches("http://")
        ),
    })
}

#[tauri::command]
fn get_connection(
    state: tauri::State<'_, AppState>,
    _profile: Option<String>,
) -> Result<HermesConnection, String> {
    let backend = ensure_backend(&state, None)?;
    Ok(backend.connection())
}

#[tauri::command]
fn get_gateway_ws_url(
    state: tauri::State<'_, AppState>,
    _profile: Option<String>,
) -> Result<String, String> {
    let backend = ensure_backend(&state, None)?;
    Ok(backend.ws_url())
}

#[tauri::command]
fn get_boot_progress() -> BootProgress {
    BootProgress {
        error: None,
        fake_mode: false,
        message: "Hermes RU Iola Tauri готов.".to_string(),
        phase: "tauri.ready".to_string(),
        progress: 100,
        running: false,
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0),
    }
}

#[tauri::command]
fn hermes_api(
    state: tauri::State<'_, AppState>,
    request: ApiRequest,
) -> Result<serde_json::Value, String> {
    let backend = ensure_backend(&state, None)?;
    backend.api(request)
}

#[tauri::command]
fn get_version() -> serde_json::Value {
    serde_json::json!({
        "appVersion": env!("CARGO_PKG_VERSION"),
        "backendVersion": null,
        "electronVersion": null,
        "nodeVersion": null,
        "runtime": "tauri"
    })
}

#[tauri::command]
fn open_external(url: String) -> Result<(), String> {
    open::that(url).map_err(|error| format!("Не удалось открыть внешнюю ссылку: {error}"))
}

#[tauri::command]
fn read_file_text(file_path: String) -> Result<ReadFileTextResult, String> {
    let path = PathBuf::from(&file_path);
    let bytes = fs::read(&path).map_err(|error| format!("Не удалось прочитать файл: {error}"))?;
    let byte_size = bytes.len() as u64;
    let binary = bytes.iter().take(8192).any(|byte| *byte == 0);
    let truncated = bytes.len() > 1_000_000;
    let sample = if truncated {
        &bytes[..1_000_000]
    } else {
        bytes.as_slice()
    };
    let text = if binary {
        String::new()
    } else {
        String::from_utf8_lossy(sample).to_string()
    };

    Ok(ReadFileTextResult {
        binary,
        byte_size,
        language: language_for_path(&path),
        mime_type: mime_for_path(&path).to_string(),
        path: normalize_path_string(path),
        text,
        truncated,
    })
}

#[tauri::command]
fn read_file_data_url(file_path: String) -> Result<String, String> {
    let path = PathBuf::from(&file_path);
    let bytes = fs::read(&path).map_err(|error| format!("Не удалось прочитать файл: {error}"))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    Ok(format!("data:{};base64,{encoded}", mime_for_path(&path)))
}

#[tauri::command]
fn read_dir(path: String) -> ReadDirResult {
    let dir = PathBuf::from(path);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(error) => {
            return ReadDirResult {
                entries: Vec::new(),
                error: Some(error.kind().to_string()),
            }
        }
    };
    let mut result = Vec::new();
    for entry in entries.flatten() {
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };
        let name = entry.file_name().to_string_lossy().to_string();
        if name.is_empty() {
            continue;
        }
        result.push(ReadDirEntry {
            is_directory: file_type.is_dir(),
            name,
            path: normalize_path_string(entry.path()),
        });
    }
    result.sort_by(|a, b| {
        b.is_directory
            .cmp(&a.is_directory)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    ReadDirResult {
        entries: result,
        error: None,
    }
}

#[tauri::command]
fn sanitize_workspace_cwd(cwd: Option<String>) -> Result<SanitizedCwd, String> {
    let requested = match cwd.filter(|value| !value.trim().is_empty()) {
        Some(value) => PathBuf::from(value),
        None => {
            env::current_dir().map_err(|error| format!("Не удалось определить каталог: {error}"))?
        }
    };

    if requested.is_dir() {
        return Ok(SanitizedCwd {
            cwd: normalize_path_string(canonical_or_self(requested)),
            sanitized: false,
        });
    }

    for ancestor in requested.ancestors().skip(1) {
        if ancestor.is_dir() {
            return Ok(SanitizedCwd {
                cwd: normalize_path_string(canonical_or_self(ancestor.to_path_buf())),
                sanitized: true,
            });
        }
    }

    let fallback =
        env::current_dir().map_err(|error| format!("Не удалось определить каталог: {error}"))?;
    Ok(SanitizedCwd {
        cwd: normalize_path_string(canonical_or_self(fallback)),
        sanitized: true,
    })
}

#[tauri::command]
fn git_root(path: String) -> Option<String> {
    let start = PathBuf::from(path);
    let base = if start.is_file() {
        start.parent().map(Path::to_path_buf)?
    } else {
        start
    };

    for candidate in base.ancestors() {
        if candidate.join(".git").exists() {
            return Some(normalize_path_string(candidate.to_path_buf()));
        }
    }
    None
}

struct BackendConnection {
    base_url: String,
    pid: u32,
    port: u16,
    python: String,
    token: String,
}

impl BackendConnection {
    fn connection(&self) -> HermesConnection {
        HermesConnection {
            auth_mode: "token".to_string(),
            base_url: self.base_url.clone(),
            is_fullscreen: false,
            logs: Vec::new(),
            mode: "local".to_string(),
            native_overlay_width: 138,
            source: "local".to_string(),
            token: self.token.clone(),
            window_button_position: None,
            ws_url: self.ws_url(),
        }
    }

    fn ws_url(&self) -> String {
        format!("ws://127.0.0.1:{}/api/ws?token={}", self.port, self.token)
    }

    fn api(&self, request: ApiRequest) -> Result<serde_json::Value, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_millis(request.timeout_ms.unwrap_or(30_000)))
            .build()
            .map_err(|error| format!("Не удалось создать HTTP client: {error}"))?;
        let path = if request.path.starts_with('/') {
            request.path
        } else {
            format!("/{}", request.path)
        };
        let url = format!("{}{}", self.base_url, path);
        let method = request
            .method
            .unwrap_or_else(|| {
                if request.body.is_some() {
                    "POST".to_string()
                } else {
                    "GET".to_string()
                }
            })
            .to_uppercase();
        let mut builder = match method.as_str() {
            "DELETE" => client.delete(&url),
            "PATCH" => client.patch(&url),
            "POST" => client.post(&url),
            "PUT" => client.put(&url),
            _ => client.get(&url),
        }
        .header("X-Hermes-Session-Token", &self.token)
        .header("Authorization", format!("Bearer {}", self.token));

        if let Some(body) = request.body {
            builder = builder.json(&body);
        }

        let response = builder
            .send()
            .map_err(|error| format!("Hermes API недоступен: {error}"))?;
        let status = response.status();
        let text = response
            .text()
            .map_err(|error| format!("Не удалось прочитать ответ Hermes API: {error}"))?;

        if !status.is_success() {
            return Err(format!("Hermes API вернул {status}: {text}"));
        }

        if text.trim().is_empty() {
            Ok(serde_json::json!({ "ok": true }))
        } else {
            serde_json::from_str(&text)
                .map_err(|error| format!("Hermes API вернул не JSON: {error}; ответ: {text}"))
        }
    }
}

fn ensure_backend(
    state: &tauri::State<'_, AppState>,
    requested_port: Option<u16>,
) -> Result<BackendConnection, String> {
    let mut guard = state
        .backend
        .lock()
        .map_err(|_| "Backend state lock poisoned".to_string())?;

    if let Some(runtime) = guard.as_mut() {
        if runtime
            .child
            .try_wait()
            .map_err(|e| e.to_string())?
            .is_none()
        {
            return Ok(runtime.connection());
        }
    }

    let runtime = launch_backend(requested_port)?;
    let connection = runtime.connection();
    *guard = Some(runtime);
    Ok(connection)
}

impl BackendRuntime {
    fn connection(&self) -> BackendConnection {
        BackendConnection {
            base_url: format!("http://127.0.0.1:{}", self.port),
            pid: self.child.id(),
            port: self.port,
            python: self.python.clone(),
            token: self.token.clone(),
        }
    }
}

fn launch_backend(requested_port: Option<u16>) -> Result<BackendRuntime, String> {
    let python = find_python().ok_or_else(|| "Python 3.11-3.14 не найден".to_string())?;
    can_import_hermes(&python)?;

    let port = requested_port.unwrap_or_else(find_free_port);
    let token = uuid::Uuid::new_v4().simple().to_string();
    let mut child = Command::new(&python);
    child.args([
        "-m",
        "hermes_cli.main",
        "dashboard",
        "--no-open",
        "--tui",
        "--host",
        "127.0.0.1",
        "--port",
        &port.to_string(),
    ]);
    child.env("HERMES_DASHBOARD_SESSION_TOKEN", &token);
    child.stdin(Stdio::null());
    child.stdout(Stdio::null());
    child.stderr(Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        child.creation_flags(0x08000000);
    }

    let mut child = child
        .spawn()
        .map_err(|error| format!("Не удалось запустить Hermes dashboard: {error}"))?;

    wait_for_backend_ready(&mut child, port, &token, Duration::from_secs(45))?;

    Ok(BackendRuntime {
        child,
        port,
        python: python.to_string_lossy().into_owned(),
        token,
    })
}

fn find_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|addr| addr.port())
        .unwrap_or(9119)
}

fn wait_for_backend_ready(
    child: &mut Child,
    port: u16,
    token: &str,
    timeout: Duration,
) -> Result<(), String> {
    let deadline = Instant::now() + timeout;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|error| format!("Не удалось создать HTTP client: {error}"))?;
    let url = format!("http://127.0.0.1:{port}/api/status");
    let mut last_error = String::new();

    while Instant::now() < deadline {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("Не удалось проверить Hermes dashboard: {error}"))?
        {
            return Err(format!("Hermes dashboard завершился при запуске: {status}"));
        }

        match client
            .get(&url)
            .header("X-Hermes-Session-Token", token)
            .send()
        {
            Ok(response) if response.status().is_success() => return Ok(()),
            Ok(response) => last_error = format!("HTTP {}", response.status()),
            Err(error) => last_error = error.to_string(),
        }
        std::thread::sleep(Duration::from_millis(250));
    }

    Err(format!("Hermes dashboard API не готов: {last_error}"))
}

fn find_python() -> Option<PathBuf> {
    if let Ok(override_path) = env::var("HERMES_DESKTOP_PYTHON") {
        let path = PathBuf::from(override_path);
        if path.is_file() {
            return Some(path);
        }
    }

    if cfg!(windows) {
        for version in ["3.14", "3.13", "3.12", "3.11"] {
            if let Some(path) = python_from_py_launcher(version) {
                return Some(path);
            }
        }
    }

    for command in ["python3", "python"] {
        if let Some(path) = find_on_path(command) {
            if python_is_supported(&path) {
                return Some(path);
            }
        }
    }

    None
}

fn python_from_py_launcher(version: &str) -> Option<PathBuf> {
    let py = find_on_path("py.exe").or_else(|| find_on_path("py"))?;
    let output = Command::new(py)
        .args([
            format!("-{version}"),
            "-c".to_string(),
            "import sys; print(sys.executable)".to_string(),
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = PathBuf::from(String::from_utf8_lossy(&output.stdout).trim());
    if path.is_file() {
        Some(path)
    } else {
        None
    }
}

fn find_on_path(command: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    let extensions: Vec<String> = if cfg!(windows) {
        env::var("PATHEXT")
            .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string())
            .split(';')
            .map(str::to_string)
            .chain(std::iter::once(String::new()))
            .collect()
    } else {
        vec![String::new()]
    };

    for dir in env::split_paths(&path_var) {
        for extension in &extensions {
            let candidate = dir.join(format!("{command}{extension}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

fn python_is_supported(path: &Path) -> bool {
    let script =
        "import sys; raise SystemExit(0 if (3, 11) <= sys.version_info[:2] <= (3, 14) else 1)";
    Command::new(path)
        .args(["-c", script])
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn python_version(path: &Path) -> Result<String, String> {
    let output = Command::new(path)
        .arg("--version")
        .output()
        .map_err(|error| format!("Не удалось проверить Python: {error}"))?;
    if !output.status.success() {
        return Err(command_error("python --version", &output));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    Ok(if stdout.is_empty() { stderr } else { stdout })
}

fn can_import_hermes(path: &Path) -> Result<(), String> {
    let output = Command::new(path)
        .args(["-c", "import hermes_cli"])
        .output()
        .map_err(|error| format!("Не удалось проверить hermes_cli: {error}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(command_error("python -c import hermes_cli", &output))
    }
}

fn command_error(label: &str, output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let details = if !stderr.is_empty() { stderr } else { stdout };
    if details.is_empty() {
        format!("{label} завершился с кодом {}", output.status)
    } else {
        format!("{label}: {details}")
    }
}

fn canonical_or_self(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

fn normalize_path_string(path: PathBuf) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn mime_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
        .as_deref()
    {
        Some("css") => "text/css",
        Some("gif") => "image/gif",
        Some("html") | Some("htm") => "text/html",
        Some("jpeg") | Some("jpg") => "image/jpeg",
        Some("js") | Some("mjs") | Some("cjs") => "text/javascript",
        Some("json") => "application/json",
        Some("md") | Some("markdown") => "text/markdown",
        Some("pdf") => "application/pdf",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("txt") | Some("log") => "text/plain",
        Some("webp") => "image/webp",
        Some("xml") => "application/xml",
        _ => "application/octet-stream",
    }
}

fn language_for_path(path: &Path) -> Option<String> {
    let language = match path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
        .as_deref()
    {
        Some("css") => "css",
        Some("html") | Some("htm") => "html",
        Some("js") | Some("mjs") | Some("cjs") => "javascript",
        Some("json") => "json",
        Some("md") | Some("markdown") => "markdown",
        Some("py") => "python",
        Some("rs") => "rust",
        Some("ts") => "typescript",
        Some("tsx") => "tsx",
        Some("xml") => "xml",
        Some("yaml") | Some("yml") => "yaml",
        _ => return None,
    };
    Some(language.to_string())
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            backend: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            backend_probe,
            backend_version,
            get_boot_progress,
            get_connection,
            get_gateway_ws_url,
            get_version,
            git_root,
            hermes_api,
            open_external,
            read_dir,
            read_file_data_url,
            read_file_text,
            sanitize_workspace_cwd,
            start_backend
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Hermes RU Iola Tauri");
}
