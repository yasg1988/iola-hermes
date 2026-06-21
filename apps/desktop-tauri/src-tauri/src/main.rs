use base64::Engine;
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use portable_pty::{native_pty_system, Child as PtyChild, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::Cursor;
use std::io::{Read, Write};
use std::net::{IpAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::process::{Child as ProcessChild, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::menu::{Menu, MenuItem, Submenu};
use tauri::utils::config::Color;
use tauri::{
    Emitter, Manager, Theme, TitleBarStyle, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    WindowEvent,
};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_deep_link::DeepLinkExt;
use tauri_plugin_dialog::DialogExt;

const LINK_TITLE_BYTE_BUDGET: usize = 96 * 1024;
const LINK_TITLE_TIMEOUT: Duration = Duration::from_secs(5);
const LINK_TITLE_USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/136.0.0.0 Safari/537.36";
const MARKETPLACE_GALLERY_QUERY_URL: &str =
    "https://marketplace.visualstudio.com/_apis/public/gallery/extensionquery";
const MARKETPLACE_MAX_JSON_BYTES: usize = 4 * 1024 * 1024;
const MARKETPLACE_MAX_VSIX_BYTES: usize = 40 * 1024 * 1024;
const MARKETPLACE_TIMEOUT: Duration = Duration::from_secs(20);
const MARKETPLACE_USER_AGENT: &str = "Hermes-RU-Iola-Desktop";
const MARKETPLACE_VSIX_ASSET_TYPE: &str = "Microsoft.VisualStudio.Services.VSIXPackage";
const MENU_OPEN_UPDATES_ID: &str = "hermes-open-updates";
const TEXT_PREVIEW_MAX_BYTES: u64 = 512 * 1024;

struct AppState {
    backend: Arc<Mutex<Option<BackendRuntime>>>,
    backend_pool: Arc<Mutex<HashMap<String, BackendRuntime>>>,
    boot_progress: Arc<Mutex<BootProgress>>,
    oauth_sessions: Arc<Mutex<Option<HashMap<String, StoredOauthSession>>>>,
    preview_watchers: Arc<Mutex<HashMap<String, PreviewWatcherRuntime>>>,
    terminals: Arc<Mutex<HashMap<String, TerminalRuntime>>>,
}

struct DeepLinkState {
    pending: Mutex<Option<DeepLinkPayload>>,
    ready: Mutex<bool>,
}

struct BackendRuntime {
    pid: u32,
    port: u16,
    python: String,
    token: String,
}

struct TerminalRuntime {
    child: Box<dyn PtyChild + Send>,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
}

struct PreviewWatcherRuntime {
    _watcher: RecommendedWatcher,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MarketplaceSearchItem {
    extension_id: String,
    display_name: String,
    publisher: String,
    description: String,
    installs: u64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MarketplaceThemeFile {
    label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    ui_theme: Option<String>,
    contents: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MarketplaceThemeResult {
    extension_id: String,
    display_name: String,
    themes: Vec<MarketplaceThemeFile>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RevalidateConnectionResult {
    ok: bool,
    rebuilt: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct TouchBackendResult {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct BootstrapResetResult {
    ok: bool,
}

#[derive(Debug, Serialize)]
struct BootstrapCancelResult {
    ok: bool,
    cancelled: bool,
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

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopConnectionConfigFile {
    mode: String,
    remote: DesktopRemoteConfig,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopRemoteConfig {
    auth_mode: Option<String>,
    token: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopConnectionConfigInput {
    mode: String,
    remote_auth_mode: Option<String>,
    remote_token: Option<String>,
    remote_url: Option<String>,
    #[allow(dead_code)]
    profile: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopConnectionConfigView {
    env_override: bool,
    mode: String,
    profile: Option<String>,
    remote_auth_mode: String,
    remote_oauth_connected: bool,
    remote_token_preview: Option<String>,
    remote_token_set: bool,
    remote_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopConnectionTestResult {
    base_url: String,
    ok: bool,
    version: Option<String>,
}

#[derive(Debug, Serialize)]
struct DesktopActiveProfile {
    profile: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct DesktopActiveProfileConfig {
    profile: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DesktopOauthLoginInput {
    password: Option<String>,
    username: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OpenSessionWindowOptions {
    watch: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct StoredOauthSession {
    cookies: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopAuthProvider {
    display_name: String,
    name: String,
    supports_password: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopConnectionProbeResult {
    auth_mode: String,
    base_url: String,
    error: Option<String>,
    providers: Vec<DesktopAuthProvider>,
    reachable: bool,
    version: Option<String>,
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

#[derive(Clone, Debug, Serialize)]
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

#[derive(Clone, Debug, Serialize)]
struct BackendExit {
    code: Option<i32>,
    signal: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
struct DeepLinkPayload {
    kind: String,
    name: String,
    params: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HermesWindowState {
    is_fullscreen: bool,
    native_overlay_width: u16,
    window_button_position: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HermesWorktreeInfo {
    repo_root: String,
    worktree_root: String,
    is_main_worktree: bool,
    branch: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HermesTitleBarTheme {
    background: String,
    foreground: String,
}

#[derive(Debug, Deserialize)]
struct TranslucencyInput {
    intensity: i32,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct TranslucencyConfig {
    intensity: u8,
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

#[derive(Clone, Debug, Serialize)]
struct PreviewFileChanged {
    id: String,
    path: String,
    url: String,
}

#[derive(Debug, Serialize)]
struct PreviewWatch {
    id: String,
    path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewTarget {
    binary: Option<bool>,
    byte_size: Option<u64>,
    kind: String,
    label: String,
    language: Option<String>,
    large: Option<bool>,
    mime_type: Option<String>,
    path: Option<String>,
    preview_kind: Option<String>,
    source: String,
    url: String,
}

#[derive(Debug, Serialize)]
struct SanitizedCwd {
    cwd: String,
    sanitized: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DefaultProjectDirView {
    default_label: String,
    dir: Option<String>,
    resolved_cwd: String,
}

#[derive(Debug, Serialize)]
struct DefaultProjectDirSaveResult {
    dir: Option<String>,
}

#[derive(Debug, Serialize)]
struct DefaultProjectDirPickResult {
    canceled: bool,
    dir: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct DefaultProjectDirConfig {
    dir: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SelectPathsOptions {
    default_path: Option<String>,
    directories: Option<bool>,
    multiple: Option<bool>,
    title: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TerminalStartOptions {
    cols: Option<u16>,
    cwd: Option<String>,
    rows: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct TerminalSize {
    cols: u16,
    rows: u16,
}

#[derive(Debug, Serialize)]
struct TerminalSession {
    cwd: String,
    id: String,
    shell: String,
}

#[derive(Clone, Debug, Serialize)]
struct TerminalExit {
    code: Option<i32>,
    signal: Option<String>,
}

#[derive(Debug, Serialize)]
struct RevealLogsResult {
    error: Option<String>,
    ok: bool,
    path: String,
}

#[derive(Debug, Serialize)]
struct RecentLogsResult {
    lines: Vec<String>,
    path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UninstallRunResult {
    error: Option<String>,
    message: String,
    mode: String,
    ok: bool,
    pid: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateApplyOptions {
    dirty_strategy: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpdateBranch {
    branch: String,
}

#[derive(Debug, Deserialize)]
struct GitHubReleaseAsset {
    browser_download_url: String,
    name: String,
    size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    assets: Vec<GitHubReleaseAsset>,
    html_url: String,
    name: Option<String>,
    tag_name: String,
}

#[derive(Debug)]
struct PackagedUpdateAsset {
    download_url: String,
    name: String,
    size: Option<u64>,
    version: String,
}

struct ShellSpec {
    args: Vec<String>,
    command: String,
    name: String,
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
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    _host: Option<String>,
    port: Option<u16>,
) -> Result<BackendProcess, String> {
    let connection = ensure_backend(&app, &state, port, None)?;

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
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    profile: Option<String>,
) -> Result<HermesConnection, String> {
    if let Some(connection) = remote_connection_from_config(&app, &state)? {
        return Ok(connection);
    }
    let backend = ensure_backend(&app, &state, None, profile)?;
    Ok(backend.connection())
}

#[tauri::command]
fn get_gateway_ws_url(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    profile: Option<String>,
) -> Result<String, String> {
    if let Some(connection) = remote_connection_from_config(&app, &state)? {
        return Ok(connection.ws_url);
    }
    let backend = ensure_backend(&app, &state, None, profile)?;
    Ok(backend.ws_url)
}

#[tauri::command]
fn revalidate_connection(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> RevalidateConnectionResult {
    match current_backend_connection(&app, &state, None) {
        Ok(connection) => match probe_backend_status(&connection, 2_500) {
            Ok(()) => RevalidateConnectionResult {
                ok: true,
                rebuilt: false,
                error: None,
            },
            Err(error) => RevalidateConnectionResult {
                ok: false,
                rebuilt: false,
                error: Some(error),
            },
        },
        Err(error) => RevalidateConnectionResult {
            ok: false,
            rebuilt: false,
            error: Some(error),
        },
    }
}

#[tauri::command]
fn touch_backend(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    profile: Option<String>,
) -> TouchBackendResult {
    match current_backend_connection(&app, &state, profile).and_then(|connection| {
        probe_backend_status(&connection, 2_500)?;
        Ok(())
    }) {
        Ok(()) => TouchBackendResult {
            ok: true,
            error: None,
        },
        Err(error) => TouchBackendResult {
            ok: false,
            error: Some(error),
        },
    }
}

#[tauri::command]
fn get_boot_progress(state: tauri::State<'_, AppState>) -> BootProgress {
    state
        .boot_progress
        .lock()
        .map(|progress| progress.clone())
        .unwrap_or_else(|_| default_boot_progress())
}

#[tauri::command]
fn hermes_api(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    request: ApiRequest,
) -> Result<serde_json::Value, String> {
    if let Some(connection) = remote_backend_connection_from_config(&app, &state)? {
        return connection.api(request);
    }
    let profile = request.profile.clone();
    let backend = ensure_backend(&app, &state, None, profile)?;
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
fn get_connection_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    profile: Option<String>,
) -> Result<DesktopConnectionConfigView, String> {
    let config = read_connection_config(&app)?;
    Ok(connection_config_view(&app, &config, profile, Some(&state)))
}

#[tauri::command]
fn save_connection_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    payload: DesktopConnectionConfigInput,
) -> Result<DesktopConnectionConfigView, String> {
    let existing = read_connection_config(&app)?;
    let config = coerce_connection_config(payload, existing)?;
    write_connection_config(&app, &config)?;
    Ok(connection_config_view(&app, &config, None, Some(&state)))
}

#[tauri::command]
fn apply_connection_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    payload: DesktopConnectionConfigInput,
) -> Result<DesktopConnectionConfigView, String> {
    let existing = read_connection_config(&app)?;
    let config = coerce_connection_config(payload, existing)?;
    write_connection_config(&app, &config)?;
    teardown_all_backends(&state);
    Ok(connection_config_view(&app, &config, None, Some(&state)))
}

#[tauri::command]
fn get_active_profile(app: tauri::AppHandle) -> DesktopActiveProfile {
    DesktopActiveProfile {
        profile: read_active_desktop_profile(&app),
    }
}

#[tauri::command]
fn set_active_profile(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    name: Option<String>,
) -> Result<DesktopActiveProfile, String> {
    let next = write_active_desktop_profile(&app, name.as_deref())?;
    teardown_all_backends(&state);
    reload_main_window(&app);
    Ok(DesktopActiveProfile { profile: next })
}

#[tauri::command]
fn reset_bootstrap(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> BootstrapResetResult {
    teardown_all_backends(&state);
    reset_boot_progress(&state);
    reload_main_window(&app);
    BootstrapResetResult { ok: true }
}

#[tauri::command]
fn repair_bootstrap(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> BootstrapResetResult {
    reset_bootstrap(app, state)
}

#[tauri::command]
fn cancel_bootstrap() -> BootstrapCancelResult {
    BootstrapCancelResult {
        ok: true,
        cancelled: false,
    }
}

#[tauri::command]
fn test_connection_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    payload: DesktopConnectionConfigInput,
) -> Result<DesktopConnectionTestResult, String> {
    let existing = read_connection_config(&app)?;
    let config = coerce_connection_config_with_token(payload, existing, false)?;
    let connection = if config.mode == "remote" {
        remote_backend_connection(&config, Some((&app, &state)))?
    } else {
        ensure_backend(&app, &state, None, None)?
    };
    let status = connection.api(ApiRequest {
        body: None,
        method: Some("GET".to_string()),
        path: "/api/status".to_string(),
        profile: None,
        timeout_ms: Some(8_000),
    })?;
    Ok(DesktopConnectionTestResult {
        base_url: connection.base_url,
        ok: true,
        version: status
            .get("version")
            .and_then(|value| value.as_str())
            .map(str::to_string),
    })
}

#[tauri::command]
fn set_native_theme(app: tauri::AppHandle, mode: String) -> Result<bool, String> {
    let theme = match mode.as_str() {
        "dark" => Some(Theme::Dark),
        "light" => Some(Theme::Light),
        "system" => None,
        other => return Err(format!("Неизвестный режим темы: {other}")),
    };
    app.set_theme(theme);
    Ok(true)
}

#[tauri::command]
fn set_title_bar_theme(
    app: tauri::AppHandle,
    payload: HermesTitleBarTheme,
) -> Result<bool, String> {
    let background = parse_hex_color(&payload.background, 255)?;
    // В Tauri пока нет стабильного API для цвета текста заголовка, но мост
    // все равно проверяет данные в формате, совместимом с Electron.
    parse_hex_color(&payload.foreground, 255)?;
    for window in app.webview_windows().values() {
        apply_title_bar_theme(window, background)?;
    }
    Ok(true)
}

#[tauri::command]
fn set_translucency(app: tauri::AppHandle, payload: TranslucencyInput) -> Result<bool, String> {
    let intensity = clamp_translucency(payload.intensity);
    write_translucency_config(&app, intensity)?;
    Ok(true)
}

#[tauri::command]
fn signal_deep_link_ready(
    app: tauri::AppHandle,
    deep_links: tauri::State<'_, DeepLinkState>,
) -> Result<serde_json::Value, String> {
    {
        let mut ready = deep_links
            .ready
            .lock()
            .map_err(|_| "Deep link state lock poisoned".to_string())?;
        *ready = true;
    }

    let pending = {
        let mut pending = deep_links
            .pending
            .lock()
            .map_err(|_| "Deep link state lock poisoned".to_string())?;
        pending.take()
    };
    if let Some(payload) = pending {
        deliver_deep_link_payload(&app, payload);
    }

    Ok(serde_json::json!({ "ok": true }))
}

#[tauri::command]
fn probe_connection_config(remote_url: String) -> DesktopConnectionProbeResult {
    let base_url = match normalize_remote_base_url(&remote_url) {
        Ok(url) => url,
        Err(error) => {
            return DesktopConnectionProbeResult {
                auth_mode: "unknown".to_string(),
                base_url: remote_url,
                error: Some(error),
                providers: Vec::new(),
                reachable: false,
                version: None,
            }
        }
    };
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
    {
        Ok(client) => client,
        Err(error) => {
            return DesktopConnectionProbeResult {
                auth_mode: "unknown".to_string(),
                base_url,
                error: Some(error.to_string()),
                providers: Vec::new(),
                reachable: false,
                version: None,
            }
        }
    };
    let status = match client.get(format!("{base_url}/api/status")).send() {
        Ok(response) if response.status().is_success() => response.json::<serde_json::Value>().ok(),
        Ok(response) => {
            return DesktopConnectionProbeResult {
                auth_mode: "unknown".to_string(),
                base_url,
                error: Some(format!("HTTP {}", response.status())),
                providers: Vec::new(),
                reachable: false,
                version: None,
            }
        }
        Err(error) => {
            return DesktopConnectionProbeResult {
                auth_mode: "unknown".to_string(),
                base_url,
                error: Some(error.to_string()),
                providers: Vec::new(),
                reachable: false,
                version: None,
            }
        }
    };
    let auth_mode = if status
        .as_ref()
        .and_then(|value| value.get("auth_required"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
    {
        "oauth"
    } else {
        "token"
    };

    let providers = if auth_mode == "oauth" {
        client
            .get(format!("{base_url}/api/auth/providers"))
            .send()
            .ok()
            .and_then(|response| response.json::<serde_json::Value>().ok())
            .and_then(|body| {
                body.get("providers")
                    .and_then(|value| value.as_array())
                    .cloned()
            })
            .unwrap_or_default()
            .into_iter()
            .filter_map(|provider| {
                let name = provider.get("name")?.as_str()?.to_string();
                Some(DesktopAuthProvider {
                    display_name: provider
                        .get("display_name")
                        .and_then(|value| value.as_str())
                        .unwrap_or(&name)
                        .to_string(),
                    name,
                    supports_password: provider
                        .get("supports_password")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false),
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    DesktopConnectionProbeResult {
        auth_mode: auth_mode.to_string(),
        base_url,
        error: None,
        providers,
        reachable: true,
        version: status
            .as_ref()
            .and_then(|value| value.get("version"))
            .and_then(|value| value.as_str())
            .map(str::to_string),
    }
}

#[tauri::command]
async fn open_session_window(
    app: tauri::AppHandle,
    session_id: String,
    opts: Option<OpenSessionWindowOptions>,
) -> serde_json::Value {
    let session_id = session_id.trim();
    if session_id.is_empty() {
        return serde_json::json!({
            "ok": false,
            "error": "empty-session-id"
        });
    }

    match open_or_focus_secondary_window(
        &app,
        &session_window_label(session_id),
        Some(session_id),
        opts.unwrap_or_default().watch.unwrap_or(false),
        false,
    ) {
        Ok(()) => serde_json::json!({ "ok": true }),
        Err(error) => serde_json::json!({ "ok": false, "error": error }),
    }
}

#[tauri::command]
async fn open_new_session_window(app: tauri::AppHandle) -> serde_json::Value {
    let label = format!("new-session-{}", timestamp_millis());
    match open_or_focus_secondary_window(&app, &label, None, false, true) {
        Ok(()) => serde_json::json!({ "ok": true }),
        Err(error) => serde_json::json!({ "ok": false, "error": error }),
    }
}

#[tauri::command]
async fn oauth_login_connection_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    remote_url: String,
    credentials: Option<DesktopOauthLoginInput>,
) -> Result<serde_json::Value, String> {
    let base_url = normalize_remote_base_url(&remote_url)?;
    let providers = auth_providers(&base_url)?;
    let password_provider = providers.iter().find(|provider| provider.supports_password);

    let Some(provider) = password_provider else {
        return oauth_redirect_login_connection_config(&app, &state, &base_url, &providers);
    };

    let username = credentials
        .as_ref()
        .and_then(|value| value.username.as_deref())
        .unwrap_or("")
        .trim();
    let password = credentials
        .as_ref()
        .and_then(|value| value.password.as_deref())
        .unwrap_or("");
    if username.is_empty() || password.is_empty() {
        return Ok(serde_json::json!({
            "ok": false,
            "baseUrl": base_url,
            "connected": false,
            "requiresCredentials": true,
            "provider": provider.name,
            "providerLabel": provider.display_name,
        }));
    }

    let client = oauth_client()?;
    let response = client
        .post(format!("{base_url}/auth/password-login"))
        .json(&serde_json::json!({
            "provider": provider.name,
            "username": username,
            "password": password,
            "next": "/"
        }))
        .send()
        .map_err(|error| format!("Не удалось выполнить вход в Hermes gateway: {error}"))?;
    let status = response.status();
    let cookies = cookies_from_response(&response);
    let text = response
        .text()
        .map_err(|error| format!("Не удалось прочитать ответ входа: {error}"))?;
    if !status.is_success() {
        return Err(format!("Hermes gateway отклонил вход ({status}): {text}"));
    }

    let mut session = StoredOauthSession { cookies };
    let connected = oauth_session_connected_with_session(&client, &base_url, &mut session);
    if connected {
        save_oauth_session(&app, &state, &base_url, session)?;
    }

    Ok(serde_json::json!({
        "ok": connected,
        "baseUrl": base_url,
        "connected": connected
    }))
}

#[tauri::command]
fn oauth_logout_connection_config(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    remote_url: Option<String>,
) -> serde_json::Value {
    if let Some(url) = remote_url
        .as_deref()
        .and_then(|value| normalize_remote_base_url(value).ok())
    {
        let _ = remove_oauth_session(&app, &state, Some(&url));
    } else {
        let _ = remove_oauth_session(&app, &state, None);
    }
    serde_json::json!({
        "ok": true,
        "connected": false
    })
}

#[tauri::command]
fn updates_get_branch(app: tauri::AppHandle) -> Result<UpdateBranch, String> {
    Ok(UpdateBranch {
        branch: read_update_branch(&app)?,
    })
}

#[tauri::command]
fn updates_set_branch(app: tauri::AppHandle, name: String) -> Result<UpdateBranch, String> {
    let branch = if name.trim().is_empty() {
        "main".to_string()
    } else {
        name.trim().to_string()
    };
    write_update_branch(&app, &branch)?;
    Ok(UpdateBranch { branch })
}

#[tauri::command]
fn updates_check(app: tauri::AppHandle) -> serde_json::Value {
    match check_desktop_update(&app) {
        Ok(value) => value,
        Err(error) => serde_json::json!({
            "supported": false,
            "reason": "check-failed",
            "message": error,
            "fetchedAt": now_millis()
        }),
    }
}

#[tauri::command]
fn updates_apply(app: tauri::AppHandle, opts: Option<UpdateApplyOptions>) -> serde_json::Value {
    let strategy = opts
        .and_then(|value| value.dirty_strategy)
        .unwrap_or_else(|| "abort".to_string());
    emit_update_progress(&app, "prepare", "Проверяю канал обновлений", Some(5), None);
    match apply_desktop_update(&app, &strategy) {
        Ok(value) => value,
        Err(error) => {
            emit_update_progress(
                &app,
                "error",
                "Обновление не выполнено",
                Some(100),
                Some(&error),
            );
            serde_json::json!({
                "ok": false,
                "error": "apply-failed",
                "message": error
            })
        }
    }
}

#[tauri::command]
fn open_external(url: String) -> Result<(), String> {
    open::that(url).map_err(|error| format!("Не удалось открыть внешнюю ссылку: {error}"))
}

#[tauri::command]
fn fetch_link_title(url: String) -> Result<String, String> {
    let parsed = parse_fetchable_link_title_url(&url)?;
    let client = reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(3))
        .timeout(LINK_TITLE_TIMEOUT)
        .user_agent(LINK_TITLE_USER_AGENT)
        .build()
        .map_err(|error| format!("Не удалось создать HTTP-клиент: {error}"))?;
    let mut response = client
        .get(parsed)
        .header(
            reqwest::header::ACCEPT,
            "text/html,application/xhtml+xml;q=0.9,*/*;q=0.5",
        )
        .header(reqwest::header::ACCEPT_LANGUAGE, "ru-RU,ru;q=0.9,en;q=0.6")
        .header(reqwest::header::ACCEPT_ENCODING, "identity")
        .send()
        .map_err(|error| format!("Не удалось получить страницу: {error}"))?;
    if !response.status().is_success() {
        return Ok(String::new());
    }

    let mut bytes = Vec::with_capacity(16 * 1024);
    let mut limited = response.by_ref().take(LINK_TITLE_BYTE_BUDGET as u64);
    limited
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Не удалось прочитать страницу: {error}"))?;
    Ok(usable_link_title(&parse_html_title(
        &String::from_utf8_lossy(&bytes),
    )))
}

#[tauri::command]
async fn search_marketplace_themes(query: String) -> Result<Vec<MarketplaceSearchItem>, String> {
    tauri::async_runtime::spawn_blocking(move || search_marketplace_themes_blocking(&query, 20))
        .await
        .map_err(|error| format!("Не удалось выполнить поиск тем: {error}"))?
}

#[tauri::command]
async fn fetch_marketplace_themes(id: String) -> Result<MarketplaceThemeResult, String> {
    tauri::async_runtime::spawn_blocking(move || fetch_marketplace_themes_blocking(&id))
        .await
        .map_err(|error| format!("Не удалось загрузить тему: {error}"))?
}

fn search_marketplace_themes_blocking(
    query: &str,
    limit: usize,
) -> Result<Vec<MarketplaceSearchItem>, String> {
    let text = query.trim();
    let page_size = limit.clamp(1, 50);
    let mut criteria = vec![
        serde_json::json!({ "filterType": 8, "value": "Microsoft.VisualStudio.Code" }),
        serde_json::json!({ "filterType": 5, "value": "Themes" }),
        serde_json::json!({ "filterType": 12, "value": "4096" }),
    ];

    if !text.is_empty() {
        criteria.push(serde_json::json!({ "filterType": 10, "value": text }));
    }

    let payload = serde_json::json!({
        "filters": [{
            "criteria": criteria,
            "pageNumber": 1,
            "pageSize": (page_size * 2).min(50),
            "sortBy": 4,
            "sortOrder": 0
        }],
        "flags": 772
    });
    let json = marketplace_gallery_query(payload)?;
    let extensions = json
        .pointer("/results/0/extensions")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();

    let mut items = Vec::new();
    for extension in extensions {
        if looks_like_icon_theme(&extension) {
            continue;
        }

        let extension_name = json_str(&extension, &["extensionName"]).unwrap_or_default();
        let publisher_name =
            json_str(&extension, &["publisher", "publisherName"]).unwrap_or_default();
        if extension_name.is_empty() || publisher_name.is_empty() {
            continue;
        }

        let installs = extension
            .get("statistics")
            .and_then(|value| value.as_array())
            .and_then(|stats| {
                stats.iter().find_map(|stat| {
                    if json_str(stat, &["statisticName"]).as_deref() == Some("install") {
                        stat.get("value").and_then(|value| value.as_f64())
                    } else {
                        None
                    }
                })
            })
            .unwrap_or(0.0)
            .round()
            .max(0.0) as u64;

        items.push(MarketplaceSearchItem {
            extension_id: format!("{publisher_name}.{extension_name}"),
            display_name: json_str(&extension, &["displayName"]).unwrap_or(extension_name),
            publisher: json_str(&extension, &["publisher", "displayName"])
                .unwrap_or(publisher_name),
            description: json_str(&extension, &["shortDescription"]).unwrap_or_default(),
            installs,
        });

        if items.len() >= page_size {
            break;
        }
    }

    Ok(items)
}

fn fetch_marketplace_themes_blocking(id: &str) -> Result<MarketplaceThemeResult, String> {
    let extension_id = id.trim();
    if !valid_marketplace_extension_id(extension_id) {
        return Err("Ожидается Marketplace id вида \"publisher.extension\".".to_string());
    }

    let (display_name, vsix_url) = resolve_marketplace_extension(extension_id)?;
    let client = marketplace_client()?;
    let mut response = client
        .get(vsix_url)
        .header(reqwest::header::USER_AGENT, MARKETPLACE_USER_AGENT)
        .send()
        .map_err(|error| format!("Не удалось скачать VSIX: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Marketplace вернул ошибку при скачивании VSIX: {error}"))?;
    let vsix = read_limited_response(&mut response, MARKETPLACE_MAX_VSIX_BYTES)?;
    let themes = extract_marketplace_themes(vsix)?;

    Ok(MarketplaceThemeResult {
        extension_id: extension_id.to_string(),
        display_name,
        themes,
    })
}

fn resolve_marketplace_extension(id: &str) -> Result<(String, String), String> {
    let payload = serde_json::json!({
        "filters": [{
            "criteria": [{ "filterType": 7, "value": id }],
            "pageNumber": 1,
            "pageSize": 1
        }],
        "flags": 914
    });
    let json = marketplace_gallery_query(payload)?;
    let extension = json
        .pointer("/results/0/extensions/0")
        .ok_or_else(|| format!("Расширение \"{id}\" не найдено в Marketplace."))?;
    let display_name = json_str(extension, &["displayName"]).unwrap_or_else(|| id.to_string());
    let version = extension
        .pointer("/versions/0")
        .ok_or_else(|| format!("У расширения \"{id}\" нет опубликованных версий."))?;
    let files = version
        .get("files")
        .and_then(|value| value.as_array())
        .ok_or_else(|| format!("У расширения \"{id}\" нет файлов для скачивания."))?;
    let vsix_url = files
        .iter()
        .find(|file| json_str(file, &["assetType"]).as_deref() == Some(MARKETPLACE_VSIX_ASSET_TYPE))
        .and_then(|file| json_str(file, &["source"]))
        .ok_or_else(|| format!("Не найден VSIX-пакет для \"{id}\"."))?;

    Ok((display_name, vsix_url))
}

fn marketplace_gallery_query(payload: serde_json::Value) -> Result<serde_json::Value, String> {
    let client = marketplace_client()?;
    let mut response = client
        .post(MARKETPLACE_GALLERY_QUERY_URL)
        .header(
            reqwest::header::ACCEPT,
            "application/json;api-version=3.0-preview.1",
        )
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(reqwest::header::USER_AGENT, MARKETPLACE_USER_AGENT)
        .json(&payload)
        .send()
        .map_err(|error| format!("Не удалось обратиться к VS Code Marketplace: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Marketplace вернул ошибку: {error}"))?;
    let bytes = read_limited_response(&mut response, MARKETPLACE_MAX_JSON_BYTES)?;

    serde_json::from_slice(&bytes).map_err(|error| format!("Marketplace вернул не JSON: {error}"))
}

fn marketplace_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(MARKETPLACE_TIMEOUT)
        .redirect(reqwest::redirect::Policy::limited(5))
        .build()
        .map_err(|error| format!("Не удалось создать HTTP-клиент Marketplace: {error}"))
}

fn read_limited_response(
    response: &mut reqwest::blocking::Response,
    max_bytes: usize,
) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 16 * 1024];

    loop {
        let read = response
            .read(&mut buffer)
            .map_err(|error| format!("Не удалось прочитать ответ: {error}"))?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.len() > max_bytes {
            return Err("Ответ превысил допустимый размер.".to_string());
        }
    }

    Ok(bytes)
}

fn extract_marketplace_themes(vsix: Vec<u8>) -> Result<Vec<MarketplaceThemeFile>, String> {
    let cursor = Cursor::new(vsix);
    let mut archive = zip::ZipArchive::new(cursor)
        .map_err(|error| format!("Некорректный VSIX-архив: {error}"))?;
    let package_json = read_zip_entry_to_string(&mut archive, "extension/package.json")?;
    let package: serde_json::Value = serde_json::from_str(&package_json)
        .map_err(|error| format!("Некорректный package.json расширения: {error}"))?;
    let contributed = package
        .pointer("/contributes/themes")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();

    let mut themes = Vec::new();
    for entry in contributed {
        let Some(path) = json_str(&entry, &["path"]) else {
            continue;
        };
        let name = marketplace_theme_entry_name(&path);
        let Ok(contents) = read_zip_entry_to_string(&mut archive, &name) else {
            continue;
        };
        let label = json_str(&entry, &["label"])
            .or_else(|| json_str(&entry, &["id"]))
            .or_else(|| json_str(&package, &["displayName"]))
            .or_else(|| json_str(&package, &["name"]))
            .unwrap_or_else(|| "VS Code Theme".to_string());

        themes.push(MarketplaceThemeFile {
            label,
            ui_theme: json_str(&entry, &["uiTheme"]),
            contents,
        });
    }

    Ok(themes)
}

fn read_zip_entry_to_string<R: Read + std::io::Seek>(
    archive: &mut zip::ZipArchive<R>,
    name: &str,
) -> Result<String, String> {
    let mut file = archive
        .by_name(name)
        .map_err(|error| format!("Не найден файл {name} в VSIX: {error}"))?;
    let mut text = String::new();
    file.read_to_string(&mut text)
        .map_err(|error| format!("Не удалось прочитать {name}: {error}"))?;
    Ok(text)
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
fn watch_preview_file(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    url: String,
) -> Result<PreviewWatch, String> {
    let file_path = file_path_from_preview_url(&url)?;
    if !file_path.is_file() {
        return Err("Файл предпросмотра не найден".to_string());
    }
    let watch_dir = file_path
        .parent()
        .ok_or_else(|| "Не удалось определить папку файла предпросмотра".to_string())?
        .to_path_buf();
    let target_name = file_path
        .file_name()
        .map(|value| value.to_string_lossy().to_string())
        .ok_or_else(|| "Не удалось определить имя файла предпросмотра".to_string())?;
    let id = uuid::Uuid::new_v4().simple().to_string();
    let normalized_path = normalize_path_string(file_path.clone());
    let event_path = file_path.clone();
    let event_id = id.clone();
    let event_app = app.clone();

    let mut watcher = RecommendedWatcher::new(
        move |result: notify::Result<notify::Event>| {
            let Ok(event) = result else {
                return;
            };
            if !matches!(
                event.kind,
                EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
            ) {
                return;
            }
            if !event.paths.is_empty()
                && !event.paths.iter().any(|path| {
                    path.file_name()
                        .map(|value| value.to_string_lossy() == target_name)
                        .unwrap_or(false)
                })
            {
                return;
            }
            if !event_path.is_file() {
                return;
            }
            let payload_path = normalize_path_string(event_path.clone());
            let payload_url = file_url_for_path(&event_path);
            let payload = PreviewFileChanged {
                id: event_id.clone(),
                path: payload_path,
                url: payload_url,
            };
            let event_app = event_app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_millis(120));
                if let Some(window) = event_app.get_webview_window("main") {
                    let _ = window.emit("hermes:preview-file-changed", payload);
                }
            });
        },
        Config::default(),
    )
    .map_err(|error| format!("Не удалось создать watcher предпросмотра: {error}"))?;
    watcher
        .watch(&watch_dir, RecursiveMode::NonRecursive)
        .map_err(|error| format!("Не удалось запустить watcher предпросмотра: {error}"))?;

    state
        .preview_watchers
        .lock()
        .map_err(|_| "Preview watcher state lock poisoned".to_string())?
        .insert(id.clone(), PreviewWatcherRuntime { _watcher: watcher });

    Ok(PreviewWatch {
        id,
        path: normalized_path,
    })
}

#[tauri::command]
fn stop_preview_file_watch(state: tauri::State<'_, AppState>, id: String) -> Result<bool, String> {
    let removed = state
        .preview_watchers
        .lock()
        .map_err(|_| "Preview watcher state lock poisoned".to_string())?
        .remove(&id)
        .is_some();
    Ok(removed)
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

#[tauri::command]
fn normalize_preview_target(raw_target: String, base_dir: Option<String>) -> Option<PreviewTarget> {
    let raw = raw_target.trim();
    if raw.is_empty() {
        return None;
    }
    if raw.starts_with("http://") || raw.starts_with("https://") {
        return preview_url_target(raw);
    }
    preview_file_target(raw, base_dir.as_deref())
}

#[tauri::command]
fn worktrees(cwds: Vec<String>) -> HashMap<String, Option<HermesWorktreeInfo>> {
    let mut result = HashMap::new();
    for cwd in cwds {
        if cwd.trim().is_empty() || result.contains_key(&cwd) {
            continue;
        }
        result.insert(cwd.clone(), resolve_worktree(&PathBuf::from(cwd)));
    }
    result
}

#[tauri::command]
fn get_default_project_dir(app: tauri::AppHandle) -> Result<DefaultProjectDirView, String> {
    let default_dir = default_project_dir_fallback();
    let configured = read_default_project_dir(&app);
    Ok(DefaultProjectDirView {
        default_label: normalize_path_string(default_dir.clone()),
        dir: configured
            .as_ref()
            .map(|path| normalize_path_string(path.clone())),
        resolved_cwd: normalize_path_string(configured.unwrap_or(default_dir)),
    })
}

#[tauri::command]
fn set_default_project_dir(
    app: tauri::AppHandle,
    dir: Option<String>,
) -> Result<DefaultProjectDirSaveResult, String> {
    let next = dir
        .and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| absolute_path(PathBuf::from(trimmed)))
        })
        .map(canonical_or_self);
    if let Some(path) = next.as_ref() {
        fs::create_dir_all(path)
            .map_err(|error| format!("Не удалось создать папку проекта: {error}"))?;
    }
    let next = next.map(canonical_or_self);
    write_default_project_dir(&app, next.as_ref())?;
    Ok(DefaultProjectDirSaveResult {
        dir: next.map(normalize_path_string),
    })
}

#[tauri::command]
fn pick_default_project_dir(app: tauri::AppHandle) -> Result<DefaultProjectDirPickResult, String> {
    let default_path = read_default_project_dir(&app).unwrap_or_else(default_project_dir_fallback);
    let picked = app
        .dialog()
        .file()
        .set_title("Выберите папку проекта по умолчанию")
        .set_directory(default_path)
        .blocking_pick_folder();
    Ok(match picked {
        Some(path) => DefaultProjectDirPickResult {
            canceled: false,
            dir: Some(normalize_path_string(path.into_path().unwrap_or_default())),
        },
        None => DefaultProjectDirPickResult {
            canceled: true,
            dir: None,
        },
    })
}

#[tauri::command]
fn select_paths(
    app: tauri::AppHandle,
    options: Option<SelectPathsOptions>,
) -> Result<Vec<String>, String> {
    let options = options.unwrap_or(SelectPathsOptions {
        default_path: None,
        directories: None,
        multiple: None,
        title: None,
    });
    let mut dialog = app.dialog().file();

    if let Some(title) = options.title {
        dialog = dialog.set_title(title);
    }
    if let Some(default_path) = options.default_path {
        dialog = dialog.set_directory(default_path);
    }

    let paths = if options.directories.unwrap_or(false) {
        if options.multiple.unwrap_or(false) {
            dialog.blocking_pick_folders()
        } else {
            dialog.blocking_pick_folder().map(|path| vec![path])
        }
    } else if options.multiple.unwrap_or(false) {
        dialog.blocking_pick_files()
    } else {
        dialog.blocking_pick_file().map(|path| vec![path])
    };

    Ok(paths
        .unwrap_or_default()
        .into_iter()
        .map(|path| normalize_path_string(path.into_path().unwrap_or_default()))
        .collect())
}

#[tauri::command]
fn write_clipboard(app: tauri::AppHandle, text: String) -> Result<bool, String> {
    app.clipboard()
        .write_text(text)
        .map_err(|error| format!("Не удалось записать текст в буфер обмена: {error}"))?;
    Ok(true)
}

#[tauri::command]
fn save_image_from_url(url: String) -> Result<bool, String> {
    let response = reqwest::blocking::get(&url)
        .map_err(|error| format!("Не удалось скачать изображение: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Сервер вернул {}", response.status()));
    }
    let mime = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let ext = image_extension_from_url_or_mime(&url, mime);
    let bytes = response
        .bytes()
        .map_err(|error| format!("Не удалось прочитать изображение: {error}"))?;
    save_image_bytes(bytes.as_ref(), &ext)?;
    Ok(true)
}

#[tauri::command]
fn save_image_buffer(data: Vec<u8>, ext: String) -> Result<String, String> {
    save_image_bytes(&data, &ext)
}

#[tauri::command]
fn save_clipboard_image(app: tauri::AppHandle) -> Result<String, String> {
    let image = app
        .clipboard()
        .read_image()
        .map_err(|error| format!("В буфере обмена нет изображения: {error}"))?;
    let rgba = image.rgba().to_vec();
    let buffer =
        image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(image.width(), image.height(), rgba)
            .ok_or_else(|| "Не удалось подготовить изображение из буфера обмена.".to_string())?;
    let mut png = Cursor::new(Vec::new());
    buffer
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(|error| format!("Не удалось закодировать изображение: {error}"))?;
    save_image_bytes(&png.into_inner(), "png")
}

#[tauri::command]
fn reveal_logs(app: tauri::AppHandle) -> RevealLogsResult {
    match ensure_log_dir(&app) {
        Ok(path) => match open::that(&path) {
            Ok(()) => RevealLogsResult {
                error: None,
                ok: true,
                path: normalize_path_string(path),
            },
            Err(error) => RevealLogsResult {
                error: Some(error.to_string()),
                ok: false,
                path: normalize_path_string(path),
            },
        },
        Err(error) => RevealLogsResult {
            error: Some(error),
            ok: false,
            path: String::new(),
        },
    }
}

#[tauri::command]
fn get_recent_logs(app: tauri::AppHandle) -> RecentLogsResult {
    let path = match ensure_log_dir(&app) {
        Ok(path) => path,
        Err(error) => {
            return RecentLogsResult {
                lines: vec![error],
                path: String::new(),
            }
        }
    };
    let lines = read_recent_log_lines(&path, 200);
    RecentLogsResult {
        lines,
        path: normalize_path_string(path),
    }
}

#[tauri::command]
fn uninstall_summary(app: tauri::AppHandle) -> serde_json::Value {
    let fallback = || uninstall_fallback_summary(&app);
    let Some(python) = find_python() else {
        return fallback();
    };
    if can_import_hermes(&python).is_err() {
        return fallback();
    }

    let output = Command::new(&python)
        .args(["-m", "hermes_cli.main", "uninstall", "--gui-summary"])
        .output();
    let Ok(output) = output else {
        return fallback();
    };
    if !output.status.success() {
        return fallback();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let Some(raw_json) = stdout.lines().rev().find(|line| !line.trim().is_empty()) else {
        return fallback();
    };
    serde_json::from_str(raw_json).unwrap_or_else(|_| fallback())
}

#[tauri::command]
fn uninstall_run(mode: String) -> UninstallRunResult {
    let normalized = mode.trim().to_lowercase();
    let args: Vec<&str> = match normalized.as_str() {
        "gui" => vec!["-m", "hermes_cli.main", "uninstall", "--gui", "--yes"],
        "lite" | "keep-data" | "keep_data" => vec!["-m", "hermes_cli.main", "uninstall", "--yes"],
        "full" => vec!["-m", "hermes_cli.main", "uninstall", "--full", "--yes"],
        _ => {
            return UninstallRunResult {
                error: Some("Неизвестный режим удаления.".to_string()),
                message: "Поддерживаются режимы gui, lite и full.".to_string(),
                mode: normalized,
                ok: false,
                pid: None,
            }
        }
    };

    let Some(python) = find_python() else {
        return UninstallRunResult {
            error: Some("Python 3.11-3.14 не найден".to_string()),
            message: "Не удалось запустить удаление Hermes RU Iola.".to_string(),
            mode: normalized,
            ok: false,
            pid: None,
        };
    };
    if let Err(error) = can_import_hermes(&python) {
        return UninstallRunResult {
            error: Some(error),
            message: "Hermes CLI недоступен в найденном Python.".to_string(),
            mode: normalized,
            ok: false,
            pid: None,
        };
    }

    let mut command = Command::new(&python);
    command.args(&args);
    command.stdin(Stdio::null());
    command.stdout(Stdio::null());
    command.stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }

    match command.spawn() {
        Ok(child) => UninstallRunResult {
            error: None,
            message: "Удаление запущено в отдельном процессе.".to_string(),
            mode: normalized,
            ok: true,
            pid: Some(child.id()),
        },
        Err(error) => UninstallRunResult {
            error: Some(error.to_string()),
            message: "Не удалось запустить удаление Hermes RU Iola.".to_string(),
            mode: normalized,
            ok: false,
            pid: None,
        },
    }
}

#[tauri::command]
fn terminal_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    options: Option<TerminalStartOptions>,
) -> Result<TerminalSession, String> {
    let options = options.unwrap_or(TerminalStartOptions {
        cols: None,
        cwd: None,
        rows: None,
    });
    let cols = options.cols.unwrap_or(80).max(2);
    let rows = options.rows.unwrap_or(24).max(2);
    let cwd = safe_terminal_cwd(options.cwd)?;
    let shell = terminal_shell_command()?;
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| format!("Не удалось создать PTY: {error}"))?;

    let mut command = CommandBuilder::new(&shell.command);
    command.args(&shell.args);
    command.cwd(&cwd);
    for (key, value) in terminal_shell_env() {
        command.env(key, value);
    }

    let child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| format!("Не удалось запустить shell: {error}"))?;
    drop(pair.slave);

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| format!("Не удалось подключить чтение PTY: {error}"))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| format!("Не удалось подключить запись PTY: {error}"))?;
    let id = uuid::Uuid::new_v4().simple().to_string();
    let data_event = terminal_channel(&id, "data");
    let exit_event = terminal_channel(&id, "exit");
    let emit_app = app.clone();

    std::thread::spawn(move || {
        let mut buf = [0_u8; 8192];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    let data = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = emit_app.emit(&data_event, data);
                }
                Err(_) => break,
            }
        }
        let _ = emit_app.emit(
            &exit_event,
            TerminalExit {
                code: None,
                signal: None,
            },
        );
    });

    let mut terminals = state
        .terminals
        .lock()
        .map_err(|_| "Terminal state lock poisoned".to_string())?;
    terminals.insert(
        id.clone(),
        TerminalRuntime {
            child,
            master: pair.master,
            writer,
        },
    );

    Ok(TerminalSession {
        cwd: normalize_path_string(cwd),
        id,
        shell: shell.name,
    })
}

#[tauri::command]
fn terminal_write(
    state: tauri::State<'_, AppState>,
    id: String,
    data: String,
) -> Result<bool, String> {
    let mut terminals = state
        .terminals
        .lock()
        .map_err(|_| "Terminal state lock poisoned".to_string())?;
    let Some(session) = terminals.get_mut(&id) else {
        return Ok(false);
    };
    session
        .writer
        .write_all(data.as_bytes())
        .map_err(|error| format!("Не удалось записать в PTY: {error}"))?;
    Ok(true)
}

#[tauri::command]
fn terminal_resize(
    state: tauri::State<'_, AppState>,
    id: String,
    size: TerminalSize,
) -> Result<bool, String> {
    let mut terminals = state
        .terminals
        .lock()
        .map_err(|_| "Terminal state lock poisoned".to_string())?;
    let Some(session) = terminals.get_mut(&id) else {
        return Ok(false);
    };
    session
        .master
        .resize(PtySize {
            rows: size.rows.max(2),
            cols: size.cols.max(2),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| format!("Не удалось изменить размер PTY: {error}"))?;
    Ok(true)
}

#[tauri::command]
fn terminal_dispose(state: tauri::State<'_, AppState>, id: String) -> Result<bool, String> {
    let mut terminals = state
        .terminals
        .lock()
        .map_err(|_| "Terminal state lock poisoned".to_string())?;
    let Some(mut session) = terminals.remove(&id) else {
        return Ok(false);
    };
    let _ = session.child.kill();
    Ok(true)
}

struct BackendConnection {
    auth_mode: String,
    base_url: String,
    mode: String,
    oauth_session: Option<StoredOauthSession>,
    pid: u32,
    python: String,
    source: String,
    token: String,
    ws_url: String,
}

impl BackendConnection {
    fn connection(&self) -> HermesConnection {
        HermesConnection {
            auth_mode: self.auth_mode.clone(),
            base_url: self.base_url.clone(),
            is_fullscreen: false,
            logs: Vec::new(),
            mode: self.mode.clone(),
            native_overlay_width: 138,
            source: self.source.clone(),
            token: self.token.clone(),
            window_button_position: None,
            ws_url: self.ws_url.clone(),
        }
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
        };

        if self.auth_mode == "oauth" {
            let session = self
                .oauth_session
                .as_ref()
                .ok_or_else(|| "OAuth session не найдена.".to_string())?;
            builder = attach_oauth_cookies(builder, session);
        } else {
            builder = builder
                .header("X-Hermes-Session-Token", &self.token)
                .header("Authorization", format!("Bearer {}", self.token));
        }

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
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
    requested_port: Option<u16>,
    profile: Option<String>,
) -> Result<BackendConnection, String> {
    let profile = normalize_desktop_profile_name(profile.as_deref())?;
    let profile_key = backend_profile_key(profile.as_deref());
    let active_key = backend_profile_key(read_active_desktop_profile(app).as_deref());
    if requested_port.is_none() && profile_key != active_key {
        return ensure_pooled_backend(app, state, profile);
    }

    let mut guard = state
        .backend
        .lock()
        .map_err(|_| "Backend state lock poisoned".to_string())?;

    if let Some(runtime) = guard.as_ref() {
        return Ok(runtime.connection());
    }

    emit_boot_progress(
        app,
        state,
        "tauri.backend.resolve",
        "Проверяю Hermes backend.",
        12,
        true,
        None,
    );
    let runtime = launch_backend(app, state, requested_port, profile, None)?;
    let connection = runtime.connection();
    *guard = Some(runtime);
    Ok(connection)
}

fn ensure_pooled_backend(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
    profile: Option<String>,
) -> Result<BackendConnection, String> {
    let profile_key = backend_profile_key(profile.as_deref());
    let mut guard = state
        .backend_pool
        .lock()
        .map_err(|_| "Backend pool lock poisoned".to_string())?;

    if let Some(runtime) = guard.get(&profile_key) {
        return Ok(runtime.connection());
    }

    emit_boot_progress(
        app,
        state,
        "tauri.backend.pool.resolve",
        "Проверяю Hermes backend для профиля.",
        12,
        true,
        None,
    );
    let runtime = launch_backend(app, state, None, profile, Some(profile_key.clone()))?;
    let connection = runtime.connection();
    guard.insert(profile_key, runtime);
    Ok(connection)
}

impl BackendRuntime {
    fn connection(&self) -> BackendConnection {
        BackendConnection {
            auth_mode: "token".to_string(),
            base_url: format!("http://127.0.0.1:{}", self.port),
            mode: "local".to_string(),
            oauth_session: None,
            pid: self.pid,
            python: self.python.clone(),
            source: "local".to_string(),
            token: self.token.clone(),
            ws_url: format!("ws://127.0.0.1:{}/api/ws?token={}", self.port, self.token),
        }
    }
}

fn default_boot_progress() -> BootProgress {
    BootProgress {
        error: None,
        fake_mode: false,
        message: "Hermes RU Iola запускается.".to_string(),
        phase: "tauri.init".to_string(),
        progress: 2,
        running: true,
        timestamp: now_millis(),
    }
}

fn reset_boot_progress(state: &tauri::State<'_, AppState>) {
    if let Ok(mut progress) = state.boot_progress.lock() {
        *progress = default_boot_progress();
    }
}

fn emit_boot_progress(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
    phase: &str,
    message: &str,
    progress: u8,
    running: bool,
    error: Option<String>,
) {
    let payload = BootProgress {
        error,
        fake_mode: false,
        message: message.to_string(),
        phase: phase.to_string(),
        progress,
        running,
        timestamp: now_millis(),
    };
    if let Ok(mut current) = state.boot_progress.lock() {
        *current = payload.clone();
    }
    let _ = app.emit("hermes:boot-progress", payload);
}

fn watch_backend_exit(
    app: tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
    mut child: ProcessChild,
    pool_key: Option<String>,
) {
    let backend_state = Arc::clone(&state.backend);
    let backend_pool = Arc::clone(&state.backend_pool);
    let boot_progress = Arc::clone(&state.boot_progress);
    std::thread::spawn(move || {
        let pooled = pool_key.is_some();
        let status = child.wait().ok();
        if let Some(key) = pool_key {
            if let Ok(mut pool) = backend_pool.lock() {
                pool.remove(&key);
            }
        } else if let Ok(mut backend) = backend_state.lock() {
            *backend = None;
        }
        let code = status.and_then(|value| value.code());
        let payload = BackendExit { code, signal: None };
        if !pooled {
            if let Ok(mut progress) = boot_progress.lock() {
                let current_progress = progress.progress;
                *progress = BootProgress {
                    error: Some("Hermes dashboard завершился.".to_string()),
                    fake_mode: false,
                    message: "Hermes dashboard завершился.".to_string(),
                    phase: "tauri.backend.exit".to_string(),
                    progress: current_progress,
                    running: false,
                    timestamp: now_millis(),
                };
            }
            let _ = app.emit("hermes:backend-exit", payload);
        }
    });
}

fn launch_backend(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
    requested_port: Option<u16>,
    profile: Option<String>,
    pool_key: Option<String>,
) -> Result<BackendRuntime, String> {
    let python = find_python().ok_or_else(|| "Python 3.11-3.14 не найден".to_string())?;
    emit_boot_progress(
        app,
        state,
        "tauri.backend.python",
        "Проверяю Python и hermes_cli.",
        20,
        true,
        None,
    );
    can_import_hermes(&python)?;

    let port = requested_port.unwrap_or_else(find_free_port);
    let token = uuid::Uuid::new_v4().simple().to_string();
    emit_boot_progress(
        app,
        state,
        "tauri.backend.spawn",
        "Запускаю локальный Hermes dashboard.",
        35,
        true,
        None,
    );
    let mut child = Command::new(&python);
    child.args(["-m", "hermes_cli.main"]);
    if let Some(profile) = profile.as_deref() {
        child.args(["--profile", &profile]);
    }
    child.args([
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

    let pid = child.id();
    emit_boot_progress(
        app,
        state,
        "tauri.backend.wait",
        "Жду готовности Hermes dashboard API.",
        55,
        true,
        None,
    );
    wait_for_backend_ready(&mut child, port, &token, Duration::from_secs(45))?;
    watch_backend_exit(app.clone(), state, child, pool_key);
    emit_boot_progress(
        app,
        state,
        "tauri.backend.ready",
        "Hermes dashboard готов.",
        100,
        false,
        None,
    );

    Ok(BackendRuntime {
        port,
        pid,
        python: python.to_string_lossy().into_owned(),
        token,
    })
}

fn teardown_primary_backend(state: &tauri::State<'_, AppState>) {
    let runtime = state.backend.lock().ok().and_then(|mut guard| guard.take());
    if let Some(runtime) = runtime {
        terminate_process(runtime.pid);
    }
}

fn teardown_all_backends(state: &tauri::State<'_, AppState>) {
    teardown_primary_backend(state);
    let runtimes = state
        .backend_pool
        .lock()
        .ok()
        .map(|mut guard| {
            guard
                .drain()
                .map(|(_, runtime)| runtime)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    for runtime in runtimes {
        terminate_process(runtime.pid);
    }
}

fn terminate_process(pid: u32) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

fn reload_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.eval("window.location.reload()");
    }
}

fn find_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|addr| addr.port())
        .unwrap_or(9119)
}

fn wait_for_backend_ready(
    child: &mut ProcessChild,
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

fn connection_config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("Не удалось определить каталог настроек: {error}"))?;
    Ok(dir.join("connection.json"))
}

fn active_profile_config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("Не удалось определить каталог настроек: {error}"))?;
    Ok(dir.join("active-profile.json"))
}

fn translucency_config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("Не удалось определить каталог настроек: {error}"))?;
    Ok(dir.join("translucency.json"))
}

fn read_active_desktop_profile(app: &tauri::AppHandle) -> Option<String> {
    let path = active_profile_config_path(app).ok()?;
    let raw = fs::read_to_string(path).ok()?;
    let config = serde_json::from_str::<DesktopActiveProfileConfig>(&raw).ok()?;
    normalize_desktop_profile_name(config.profile.as_deref())
        .ok()
        .flatten()
}

fn write_active_desktop_profile(
    app: &tauri::AppHandle,
    name: Option<&str>,
) -> Result<Option<String>, String> {
    let profile = normalize_desktop_profile_name(name)?;
    let path = active_profile_config_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Не удалось создать каталог настроек: {error}"))?;
    }
    let raw = serde_json::to_string_pretty(&DesktopActiveProfileConfig {
        profile: profile.clone(),
    })
    .map_err(|error| format!("Не удалось сериализовать настройки профиля: {error}"))?;
    fs::write(path, raw)
        .map_err(|error| format!("Не удалось сохранить настройки профиля: {error}"))?;
    Ok(profile)
}

fn normalize_desktop_profile_name(name: Option<&str>) -> Result<Option<String>, String> {
    let value = name.unwrap_or_default().trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value == "default" || valid_profile_name(value) {
        Ok(Some(value.to_string()))
    } else {
        Err(format!("Некорректное имя профиля: {value}"))
    }
}

fn valid_profile_name(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.is_empty() || bytes.len() > 64 {
        return false;
    }
    if !bytes[0].is_ascii_lowercase() && !bytes[0].is_ascii_digit() {
        return false;
    }
    bytes.iter().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(*byte, b'_' | b'-')
    })
}

fn default_project_dir_config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("Не удалось определить каталог настроек: {error}"))?;
    Ok(dir.join("project-dir.json"))
}

fn default_project_dir_fallback() -> PathBuf {
    home_dir()
        .or_else(|| env::current_dir().ok())
        .map(canonical_or_self)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn read_default_project_dir(app: &tauri::AppHandle) -> Option<PathBuf> {
    let path = default_project_dir_config_path(app).ok()?;
    let raw = fs::read_to_string(path).ok()?;
    let config = serde_json::from_str::<DefaultProjectDirConfig>(&raw).ok()?;
    let dir = config.dir?.trim().to_string();
    if dir.is_empty() {
        return None;
    }
    let path = PathBuf::from(dir);
    path.is_dir().then(|| canonical_or_self(path))
}

fn write_default_project_dir(app: &tauri::AppHandle, dir: Option<&PathBuf>) -> Result<(), String> {
    let path = default_project_dir_config_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Не удалось создать каталог настроек: {error}"))?;
    }
    let raw = serde_json::to_string_pretty(&DefaultProjectDirConfig {
        dir: dir.map(|path| normalize_path_string(path.clone())),
    })
    .map_err(|error| format!("Не удалось сериализовать настройки папки проекта: {error}"))?;
    fs::write(path, raw)
        .map_err(|error| format!("Не удалось сохранить настройки папки проекта: {error}"))
}

fn clamp_translucency(value: i32) -> u8 {
    value.clamp(0, 100) as u8
}

fn parse_hex_color(value: &str, alpha: u8) -> Result<Color, String> {
    let raw = value.trim().strip_prefix('#').unwrap_or(value.trim());
    if raw.len() != 6 || !raw.as_bytes().iter().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("Некорректный цвет titlebar: {value}"));
    }
    let red = u8::from_str_radix(&raw[0..2], 16)
        .map_err(|_| format!("Некорректный цвет titlebar: {value}"))?;
    let green = u8::from_str_radix(&raw[2..4], 16)
        .map_err(|_| format!("Некорректный цвет titlebar: {value}"))?;
    let blue = u8::from_str_radix(&raw[4..6], 16)
        .map_err(|_| format!("Некорректный цвет titlebar: {value}"))?;
    Ok(Color(red, green, blue, alpha))
}

fn apply_title_bar_theme(window: &WebviewWindow, background: Color) -> Result<(), String> {
    let _ = window.set_title_bar_style(TitleBarStyle::Transparent);
    window
        .set_background_color(Some(background))
        .map_err(|error| format!("Не удалось применить цвет окна: {error}"))
}

fn parse_fetchable_link_title_url(raw_url: &str) -> Result<reqwest::Url, String> {
    let parsed = reqwest::Url::parse(raw_url.trim())
        .map_err(|_| "Некорректная внешняя ссылка".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Поддерживаются только http/https ссылки".to_string());
    }
    let host = parsed
        .host_str()
        .unwrap_or_default()
        .trim_matches(|ch| ch == '[' || ch == ']')
        .to_ascii_lowercase();
    if host == "localhost" {
        return Err("Локальные ссылки не запрашиваются для предпросмотра".to_string());
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_private_link_title_ip(ip) {
            return Err("Локальные ссылки не запрашиваются для предпросмотра".to_string());
        }
    }
    Ok(parsed)
}

fn is_private_link_title_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(value) => {
            value.is_loopback()
                || value.is_private()
                || value.is_link_local()
                || value.is_broadcast()
                || value.is_documentation()
                || value.is_unspecified()
        }
        IpAddr::V6(value) => {
            value.is_loopback()
                || value.is_unspecified()
                || value.is_unique_local()
                || value.is_unicast_link_local()
        }
    }
}

fn parse_html_title(html: &str) -> String {
    let lower = html.to_lowercase();
    let Some(start) = lower.find("<title") else {
        return String::new();
    };
    let after_open = &html[start..];
    let Some(open_end) = after_open.find('>') else {
        return String::new();
    };
    let content_start = start + open_end + 1;
    let Some(close_offset) = lower[content_start..].find("</title>") else {
        return String::new();
    };
    decode_html_entities(&html[content_start..content_start + close_offset])
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn decode_html_entities(value: &str) -> String {
    let chars = value.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(value.len());
    let mut index = 0;
    while index < chars.len() {
        if chars[index] != '&' {
            output.push(chars[index]);
            index += 1;
            continue;
        }
        let Some(relative_end) = chars[index + 1..].iter().position(|ch| *ch == ';') else {
            output.push(chars[index]);
            index += 1;
            continue;
        };
        let end = index + 1 + relative_end;
        let entity = chars[index + 1..end].iter().collect::<String>();
        if let Some(decoded) = decode_html_entity(&entity) {
            output.push(decoded);
            index = end + 1;
        } else {
            output.push(chars[index]);
            index += 1;
        }
    }
    output
}

fn decode_html_entity(entity: &str) -> Option<char> {
    match entity.to_ascii_lowercase().as_str() {
        "amp" => Some('&'),
        "apos" | "#39" => Some('\''),
        "gt" => Some('>'),
        "lt" => Some('<'),
        "nbsp" => Some(' '),
        "quot" => Some('"'),
        value if value.starts_with("#x") => u32::from_str_radix(&value[2..], 16)
            .ok()
            .and_then(char::from_u32),
        value if value.starts_with('#') => value[1..].parse::<u32>().ok().and_then(char::from_u32),
        _ => None,
    }
}

fn valid_marketplace_extension_id(value: &str) -> bool {
    let mut parts = value.split('.');
    let Some(publisher) = parts.next() else {
        return false;
    };
    let Some(extension) = parts.next() else {
        return false;
    };
    if parts.next().is_some() || publisher.is_empty() || extension.is_empty() {
        return false;
    }

    publisher.chars().all(marketplace_id_char) && extension.chars().all(marketplace_id_char)
}

fn marketplace_id_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}

fn marketplace_theme_entry_name(theme_path: &str) -> String {
    let normalized = theme_path
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string();
    format!("extension/{normalized}")
}

fn looks_like_icon_theme(extension: &serde_json::Value) -> bool {
    let has_icon_tag = extension
        .get("tags")
        .and_then(|value| value.as_array())
        .map(|tags| {
            tags.iter().any(|tag| {
                let tag = tag.as_str().unwrap_or_default().to_ascii_lowercase();
                tag == "icon-theme" || tag == "product-icon-theme"
            })
        })
        .unwrap_or(false);
    if has_icon_tag {
        return true;
    }

    let text = format!(
        "{} {}",
        json_str(extension, &["displayName"]).unwrap_or_default(),
        json_str(extension, &["shortDescription"]).unwrap_or_default()
    )
    .to_ascii_lowercase();

    [
        "icon theme",
        "file icon",
        "file icons",
        "product icon",
        "product icons",
        "icon pack",
        "fileicons",
    ]
    .iter()
    .any(|needle| text.contains(needle))
}

fn json_str(value: &serde_json::Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(ToString::to_string)
}

fn usable_link_title(value: &str) -> String {
    let cleaned = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(240)
        .collect::<String>();
    let lower = cleaned.to_lowercase();
    let blocked = [
        "access denied",
        "attention required",
        "captcha",
        "error",
        "forbidden",
        "just a moment",
        "request blocked",
        "too many requests",
    ];
    if blocked.iter().any(|marker| lower.contains(marker)) {
        String::new()
    } else {
        cleaned
    }
}

fn resolve_worktree(start_path: &Path) -> Option<HermesWorktreeInfo> {
    let resolved = canonical_or_self(start_path.to_path_buf());
    let start = if resolved.is_file() {
        resolved.parent()?.to_path_buf()
    } else if resolved.is_dir() {
        resolved
    } else {
        return None;
    };
    let host = find_git_host(&start)?;
    resolve_worktree_from_host(&host)
}

fn find_git_host(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    for _ in 0..64 {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
    None
}

fn resolve_worktree_from_host(host: &Path) -> Option<HermesWorktreeInfo> {
    let dotgit = host.join(".git");
    if dotgit.is_dir() {
        return Some(HermesWorktreeInfo {
            repo_root: normalize_path_string(host.to_path_buf()),
            worktree_root: normalize_path_string(host.to_path_buf()),
            is_main_worktree: true,
            branch: read_git_branch(&dotgit),
        });
    }
    if !dotgit.is_file() {
        return None;
    }

    let contents = fs::read_to_string(&dotgit).ok()?;
    let gitdir = contents
        .lines()
        .find_map(|line| line.trim().strip_prefix("gitdir:").map(str::trim))?;
    let admin_dir = resolve_relative_path(host, gitdir);
    let common_dir = fs::read_to_string(admin_dir.join("commondir"))
        .ok()
        .map(|raw| resolve_relative_path(&admin_dir, raw.trim()))
        .unwrap_or_else(|| {
            admin_dir
                .parent()
                .and_then(Path::parent)
                .map(Path::to_path_buf)
                .unwrap_or_else(|| admin_dir.clone())
        });
    let repo_root = common_dir.parent()?.to_path_buf();

    Some(HermesWorktreeInfo {
        repo_root: normalize_path_string(repo_root),
        worktree_root: normalize_path_string(host.to_path_buf()),
        is_main_worktree: false,
        branch: read_git_branch(&admin_dir),
    })
}

fn resolve_relative_path(base: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        canonical_or_self(path)
    } else {
        canonical_or_self(base.join(path))
    }
}

fn read_git_branch(git_dir: &Path) -> Option<String> {
    let head = fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    if let Some(branch) = head.strip_prefix("ref: refs/heads/") {
        let branch = branch.trim();
        return (!branch.is_empty()).then(|| branch.to_string());
    }
    if head.len() >= 7 && head.len() <= 40 && head.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Some(head.chars().take(8).collect());
    }
    None
}

fn preview_url_target(raw: &str) -> Option<PreviewTarget> {
    let mut url = reqwest::Url::parse(raw).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let host = url.host_str()?.to_ascii_lowercase();
    if !matches!(
        host.as_str(),
        "0.0.0.0" | "127.0.0.1" | "::1" | "[::1]" | "localhost"
    ) {
        return None;
    }
    if host == "0.0.0.0" {
        url.set_host(Some("127.0.0.1")).ok()?;
    }
    Some(PreviewTarget {
        binary: None,
        byte_size: None,
        kind: "url".to_string(),
        label: preview_label_for_url(&url),
        language: None,
        large: None,
        mime_type: None,
        path: None,
        preview_kind: None,
        source: raw.to_string(),
        url: url.to_string(),
    })
}

fn preview_file_target(raw: &str, base_dir: Option<&str>) -> Option<PreviewTarget> {
    let base = base_dir
        .filter(|value| !value.trim().is_empty())
        .map(expand_user_path)
        .map(absolute_path)
        .unwrap_or_else(default_project_dir_fallback);
    let mut resolved = if raw.to_ascii_lowercase().starts_with("file:") {
        file_path_from_preview_url(raw).ok()?
    } else {
        let expanded = expand_user_path(raw);
        if expanded.is_absolute() {
            expanded
        } else {
            base.join(expanded)
        }
    };
    resolved = canonical_or_self(resolved);
    if resolved.is_dir() {
        resolved = resolved.join("index.html");
    }
    if !resolved.is_file() {
        return None;
    }
    resolved = canonical_or_self(resolved);
    let mime_type = mime_for_path(&resolved).to_string();
    let (binary, byte_size, large) = preview_file_metadata(&resolved, &mime_type);
    let ext = resolved
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    let is_html = matches!(ext.as_str(), "html" | "htm");
    let is_image = mime_type.starts_with("image/");
    let preview_kind = if is_html {
        "html"
    } else if is_image {
        "image"
    } else if binary {
        "binary"
    } else {
        "text"
    };
    Some(PreviewTarget {
        binary: Some(binary),
        byte_size: Some(byte_size),
        kind: "file".to_string(),
        label: resolved.file_name()?.to_string_lossy().to_string(),
        language: language_for_path(&resolved).or_else(|| Some("text".to_string())),
        large: Some(large),
        mime_type: Some(mime_type),
        path: Some(normalize_path_string(resolved.clone())),
        preview_kind: Some(preview_kind.to_string()),
        source: raw.to_string(),
        url: file_url_for_path(&resolved),
    })
}

fn preview_label_for_url(url: &reqwest::Url) -> String {
    let path = if url.path() == "/" { "" } else { url.path() };
    format!("{}{}", url.host_str().unwrap_or_default(), path)
}

fn expand_user_path(value: &str) -> PathBuf {
    let trimmed = value.trim();
    if trimmed == "~" {
        return home_dir().unwrap_or_else(|| PathBuf::from(trimmed));
    }
    if let Some(rest) = trimmed.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(trimmed)
}

fn preview_file_metadata(path: &Path, mime_type: &str) -> (bool, u64, bool) {
    let byte_size = fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let binary = if mime_type.starts_with("image/") {
        false
    } else {
        read_file_prefix(path, 4096)
            .map(|bytes| looks_binary(&bytes))
            .unwrap_or(false)
    };
    (binary, byte_size, byte_size > TEXT_PREVIEW_MAX_BYTES)
}

fn read_file_prefix(path: &Path, limit: usize) -> Result<Vec<u8>, std::io::Error> {
    let mut file = fs::File::open(path)?;
    let mut buffer = vec![0; limit];
    let bytes_read = file.read(&mut buffer)?;
    buffer.truncate(bytes_read);
    Ok(buffer)
}

fn looks_binary(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let mut suspicious = 0usize;
    for byte in bytes {
        if *byte == 0 {
            return true;
        }
        if *byte < 32 && !matches!(*byte, 9 | 10 | 13) {
            suspicious += 1;
        }
    }
    suspicious as f64 / bytes.len() as f64 > 0.12
}

fn write_translucency_config(app: &tauri::AppHandle, intensity: u8) -> Result<(), String> {
    let path = translucency_config_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Не удалось создать каталог настроек: {error}"))?;
    }
    let raw = serde_json::to_string_pretty(&TranslucencyConfig { intensity })
        .map_err(|error| format!("Не удалось сериализовать настройки прозрачности: {error}"))?;
    fs::write(path, raw)
        .map_err(|error| format!("Не удалось сохранить настройки прозрачности: {error}"))
}

fn default_connection_config() -> DesktopConnectionConfigFile {
    DesktopConnectionConfigFile {
        mode: "local".to_string(),
        remote: DesktopRemoteConfig {
            auth_mode: Some("token".to_string()),
            token: None,
            url: None,
        },
    }
}

fn read_connection_config(app: &tauri::AppHandle) -> Result<DesktopConnectionConfigFile, String> {
    let path = connection_config_path(app)?;
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(_) => return Ok(default_connection_config()),
    };
    let parsed = serde_json::from_str::<DesktopConnectionConfigFile>(&raw)
        .unwrap_or_else(|_| default_connection_config());
    Ok(normalize_connection_config(parsed))
}

fn write_connection_config(
    app: &tauri::AppHandle,
    config: &DesktopConnectionConfigFile,
) -> Result<(), String> {
    let path = connection_config_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Не удалось создать каталог настроек: {error}"))?;
    }
    let raw = serde_json::to_string_pretty(config)
        .map_err(|error| format!("Не удалось сериализовать настройки подключения: {error}"))?;
    fs::write(path, raw)
        .map_err(|error| format!("Не удалось сохранить настройки подключения: {error}"))
}

fn normalize_connection_config(
    mut config: DesktopConnectionConfigFile,
) -> DesktopConnectionConfigFile {
    config.mode = if config.mode == "remote" {
        "remote".to_string()
    } else {
        "local".to_string()
    };
    config.remote.auth_mode = Some(norm_auth_mode(config.remote.auth_mode.as_deref()).to_string());
    if let Some(url) = config
        .remote
        .url
        .clone()
        .filter(|value| !value.trim().is_empty())
    {
        config.remote.url = normalize_remote_base_url(&url).ok();
    }
    config
}

fn coerce_connection_config(
    input: DesktopConnectionConfigInput,
    existing: DesktopConnectionConfigFile,
) -> Result<DesktopConnectionConfigFile, String> {
    coerce_connection_config_with_token(input, existing, true)
}

fn coerce_connection_config_with_token(
    input: DesktopConnectionConfigInput,
    existing: DesktopConnectionConfigFile,
    _persist_token: bool,
) -> Result<DesktopConnectionConfigFile, String> {
    let mode = if input.mode == "remote" {
        "remote"
    } else {
        "local"
    };
    let auth_mode = norm_auth_mode(
        input
            .remote_auth_mode
            .as_deref()
            .or(existing.remote.auth_mode.as_deref()),
    );
    let remote_url = input
        .remote_url
        .as_deref()
        .or(existing.remote.url.as_deref())
        .unwrap_or("")
        .trim();
    let token = input
        .remote_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or_else(|| existing.remote.token.clone());

    let normalized_url = if remote_url.is_empty() {
        None
    } else {
        Some(normalize_remote_base_url(remote_url)?)
    };

    if mode == "remote" {
        let url = normalized_url
            .clone()
            .ok_or_else(|| "URL удаленного Hermes gateway обязателен.".to_string())?;
        if auth_mode == "token" && token.as_deref().unwrap_or("").is_empty() {
            return Err("Session token удаленного Hermes gateway обязателен.".to_string());
        }
        Ok(DesktopConnectionConfigFile {
            mode: "remote".to_string(),
            remote: DesktopRemoteConfig {
                auth_mode: Some(auth_mode.to_string()),
                token,
                url: Some(url),
            },
        })
    } else {
        Ok(DesktopConnectionConfigFile {
            mode: "local".to_string(),
            remote: DesktopRemoteConfig {
                auth_mode: Some(auth_mode.to_string()),
                token,
                url: normalized_url,
            },
        })
    }
}

fn connection_config_view(
    app: &tauri::AppHandle,
    config: &DesktopConnectionConfigFile,
    profile: Option<String>,
    state: Option<&tauri::State<'_, AppState>>,
) -> DesktopConnectionConfigView {
    let token = config.remote.token.clone().unwrap_or_default();
    let oauth_connected = if config.mode == "remote"
        && norm_auth_mode(config.remote.auth_mode.as_deref()) == "oauth"
    {
        config
            .remote
            .url
            .as_deref()
            .and_then(|url| normalize_remote_base_url(url).ok())
            .and_then(|url| state.map(|state| oauth_session_connected(state, app, &url)))
            .unwrap_or(false)
    } else {
        false
    };
    DesktopConnectionConfigView {
        env_override: false,
        mode: config.mode.clone(),
        profile,
        remote_auth_mode: norm_auth_mode(config.remote.auth_mode.as_deref()).to_string(),
        remote_oauth_connected: oauth_connected,
        remote_token_preview: token_preview(&token),
        remote_token_set: !token.is_empty(),
        remote_url: config.remote.url.clone().unwrap_or_default(),
    }
}

fn norm_auth_mode(value: Option<&str>) -> &'static str {
    if value == Some("oauth") {
        "oauth"
    } else {
        "token"
    }
}

fn token_preview(token: &str) -> Option<String> {
    if token.is_empty() {
        None
    } else if token.len() <= 8 {
        Some("********".to_string())
    } else {
        let start = &token[..4];
        let end = &token[token.len().saturating_sub(4)..];
        Some(format!("{start}...{end}"))
    }
}

fn normalize_remote_base_url(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim().trim_end_matches('/');
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err("URL должен начинаться с http:// или https://".to_string());
    }
    Ok(trimmed.to_string())
}

fn auth_providers(base_url: &str) -> Result<Vec<DesktopAuthProvider>, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|error| format!("Не удалось создать HTTP client: {error}"))?;
    let response = client
        .get(format!("{base_url}/api/auth/providers"))
        .send()
        .map_err(|error| format!("Не удалось получить providers gateway: {error}"))?;
    if !response.status().is_success() {
        return Err(format!("Gateway providers вернул {}", response.status()));
    }
    let body = response
        .json::<serde_json::Value>()
        .map_err(|error| format!("Gateway providers вернул не JSON: {error}"))?;
    Ok(body
        .get("providers")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|provider| {
            let name = provider.get("name")?.as_str()?.to_string();
            Some(DesktopAuthProvider {
                display_name: provider
                    .get("display_name")
                    .and_then(|value| value.as_str())
                    .unwrap_or(&name)
                    .to_string(),
                name,
                supports_password: provider
                    .get("supports_password")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false),
            })
        })
        .collect())
}

fn oauth_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| format!("Не удалось создать OAuth HTTP client: {error}"))
}

fn open_or_focus_secondary_window(
    app: &tauri::AppHandle,
    label: &str,
    session_id: Option<&str>,
    watch: bool,
    new_session: bool,
) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(label) {
        focus_webview_window(&window);
        return Ok(());
    }

    let route = secondary_window_route(session_id, watch, new_session);
    let window = WebviewWindowBuilder::new(app, label.to_string(), WebviewUrl::App(route.into()))
        .title("Hermes RU Iola")
        .inner_size(420.0, 620.0)
        .min_inner_size(420.0, 620.0)
        .resizable(true)
        .transparent(true)
        .build()
        .map_err(|error| format!("Не удалось открыть окно сессии: {error}"))?;
    install_window_state_events(&window);
    focus_webview_window(&window);
    Ok(())
}

fn focus_webview_window(window: &WebviewWindow) {
    let _ = window.unminimize();
    let _ = window.show();
    let _ = window.set_focus();
}

fn secondary_window_route(session_id: Option<&str>, watch: bool, new_session: bool) -> PathBuf {
    let mut query = "?win=secondary".to_string();
    if new_session {
        query.push_str("&new=1");
    }
    if watch {
        query.push_str("&watch=1");
    }
    let route = if new_session {
        "#/".to_string()
    } else {
        format!(
            "#/{}",
            percent_encoding::utf8_percent_encode(
                session_id.unwrap_or_default(),
                percent_encoding::NON_ALPHANUMERIC
            )
        )
    };
    PathBuf::from(format!("index.html{query}{route}"))
}

fn session_window_label(session_id: &str) -> String {
    let mut hasher = DefaultHasher::new();
    session_id.hash(&mut hasher);
    format!("session-{:x}", hasher.finish())
}

fn oauth_redirect_login_connection_config(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
    base_url: &str,
    providers: &[DesktopAuthProvider],
) -> Result<serde_json::Value, String> {
    let login_url = reqwest::Url::parse(&format!("{base_url}/login"))
        .map_err(|error| format!("Некорректный URL входа gateway: {error}"))?;
    let label = format!("oauth-login-{}", timestamp_millis());
    let login_window = WebviewWindowBuilder::new(app, label, WebviewUrl::External(login_url))
        .title("Вход в Hermes gateway")
        .inner_size(520.0, 720.0)
        .resizable(true)
        .build()
        .map_err(|error| format!("Не удалось открыть окно OAuth-входа: {error}"))?;

    let closed = Arc::new(Mutex::new(false));
    let closed_for_event = Arc::clone(&closed);
    login_window.on_window_event(move |event| {
        if matches!(
            event,
            WindowEvent::CloseRequested { .. } | WindowEvent::Destroyed
        ) {
            if let Ok(mut guard) = closed_for_event.lock() {
                *guard = true;
            }
        }
    });

    let client = oauth_client()?;
    let deadline = Instant::now() + Duration::from_secs(300);
    let provider_label = providers
        .first()
        .map(|provider| provider.display_name.clone())
        .unwrap_or_else(|| "OAuth".to_string());

    while Instant::now() < deadline {
        if closed.lock().map(|guard| *guard).unwrap_or(true) {
            return Ok(serde_json::json!({
                "ok": false,
                "baseUrl": base_url,
                "connected": false,
                "requiresExternalOauth": true,
                "error": "Окно входа закрыто до завершения авторизации."
            }));
        }

        let mut session = oauth_session_from_webview(&login_window, base_url)?;
        if !session.cookies.is_empty()
            && oauth_session_connected_with_session(&client, base_url, &mut session)
        {
            save_oauth_session(app, state, base_url, session)?;
            let _ = login_window.close();
            return Ok(serde_json::json!({
                "ok": true,
                "baseUrl": base_url,
                "connected": true,
                "providerLabel": provider_label
            }));
        }

        std::thread::sleep(Duration::from_millis(750));
    }

    let _ = login_window.close();
    Ok(serde_json::json!({
        "ok": false,
        "baseUrl": base_url,
        "connected": false,
        "requiresExternalOauth": true,
        "error": "Вход не завершен за 5 минут. Повторите попытку."
    }))
}

fn oauth_session_from_webview(
    window: &WebviewWindow,
    base_url: &str,
) -> Result<StoredOauthSession, String> {
    let cookie_url = reqwest::Url::parse(&format!("{base_url}/api/auth/me"))
        .map_err(|error| format!("Некорректный URL проверки gateway: {error}"))?;
    let cookies = window
        .cookies_for_url(cookie_url)
        .map_err(|error| format!("Не удалось прочитать OAuth cookies из WebView: {error}"))?;
    let mut session = StoredOauthSession::default();
    for cookie in cookies {
        let name = cookie.name().trim();
        let value = cookie.value().trim();
        if !name.is_empty() && !value.is_empty() {
            session.cookies.insert(name.to_string(), value.to_string());
        }
    }
    Ok(session)
}

fn timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn oauth_sessions_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("Не удалось определить каталог настроек: {error}"))?;
    Ok(dir.join("oauth-sessions.json"))
}

fn read_oauth_sessions(app: &tauri::AppHandle) -> HashMap<String, StoredOauthSession> {
    let path = match oauth_sessions_path(app) {
        Ok(path) => path,
        Err(_) => return HashMap::new(),
    };
    let raw = match fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(_) => return HashMap::new(),
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn write_oauth_sessions(
    app: &tauri::AppHandle,
    sessions: &HashMap<String, StoredOauthSession>,
) -> Result<(), String> {
    let path = oauth_sessions_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Не удалось создать каталог настроек: {error}"))?;
    }
    let raw = serde_json::to_string_pretty(sessions)
        .map_err(|error| format!("Не удалось сериализовать OAuth sessions: {error}"))?;
    fs::write(path, raw).map_err(|error| format!("Не удалось сохранить OAuth sessions: {error}"))
}

fn oauth_sessions(
    state: &tauri::State<'_, AppState>,
    app: &tauri::AppHandle,
) -> Result<HashMap<String, StoredOauthSession>, String> {
    let mut guard = state
        .oauth_sessions
        .lock()
        .map_err(|_| "OAuth state lock poisoned".to_string())?;
    if guard.is_none() {
        *guard = Some(read_oauth_sessions(app));
    }
    Ok(guard.clone().unwrap_or_default())
}

fn oauth_session(
    state: &tauri::State<'_, AppState>,
    app: &tauri::AppHandle,
    base_url: &str,
) -> Result<StoredOauthSession, String> {
    oauth_sessions(state, app)?.get(base_url).cloned().ok_or_else(|| {
        "Remote Hermes gateway использует OAuth/password вход, но активная Tauri-сессия не найдена. Откройте настройки gateway и выполните вход.".to_string()
    })
}

fn save_oauth_session(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
    base_url: &str,
    session: StoredOauthSession,
) -> Result<(), String> {
    let mut sessions = oauth_sessions(state, app)?;
    sessions.insert(base_url.to_string(), session);
    write_oauth_sessions(app, &sessions)?;
    let mut guard = state
        .oauth_sessions
        .lock()
        .map_err(|_| "OAuth state lock poisoned".to_string())?;
    *guard = Some(sessions);
    Ok(())
}

fn remove_oauth_session(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
    base_url: Option<&str>,
) -> Result<(), String> {
    let mut sessions = oauth_sessions(state, app)?;
    if let Some(base_url) = base_url {
        sessions.remove(base_url);
    } else {
        sessions.clear();
    }
    write_oauth_sessions(app, &sessions)?;
    let mut guard = state
        .oauth_sessions
        .lock()
        .map_err(|_| "OAuth state lock poisoned".to_string())?;
    *guard = Some(sessions);
    Ok(())
}

fn oauth_session_connected(
    state: &tauri::State<'_, AppState>,
    app: &tauri::AppHandle,
    base_url: &str,
) -> bool {
    let Ok(mut session) = oauth_session(state, app, base_url) else {
        return false;
    };
    let Ok(client) = oauth_client() else {
        return false;
    };
    let connected = oauth_session_connected_with_session(&client, base_url, &mut session);
    if connected {
        let _ = save_oauth_session(app, state, base_url, session);
    }
    connected
}

fn oauth_session_connected_with_session(
    client: &reqwest::blocking::Client,
    base_url: &str,
    session: &mut StoredOauthSession,
) -> bool {
    let response =
        attach_oauth_cookies(client.get(format!("{base_url}/api/auth/me")), session).send();
    match response {
        Ok(response) => {
            let ok = response.status().is_success();
            merge_response_cookies(session, &response);
            ok
        }
        Err(_) => false,
    }
}

fn mint_gateway_ws_ticket(
    base_url: &str,
    session: &mut StoredOauthSession,
) -> Result<String, String> {
    let client = oauth_client()?;
    let response = attach_oauth_cookies(
        client.post(format!("{base_url}/api/auth/ws-ticket")),
        session,
    )
    .send()
    .map_err(|error| format!("Не удалось получить WS ticket gateway: {error}"))?;
    let status = response.status();
    merge_response_cookies(session, &response);
    let text = response
        .text()
        .map_err(|error| format!("Не удалось прочитать WS ticket: {error}"))?;
    if !status.is_success() {
        return Err(format!("Gateway WS ticket вернул {status}: {text}"));
    }
    let body = serde_json::from_str::<serde_json::Value>(&text)
        .map_err(|error| format!("Gateway WS ticket вернул не JSON: {error}; ответ: {text}"))?;
    body.get("ticket")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .ok_or_else(|| "Gateway не вернул WS ticket.".to_string())
}

fn attach_oauth_cookies(
    builder: reqwest::blocking::RequestBuilder,
    session: &StoredOauthSession,
) -> reqwest::blocking::RequestBuilder {
    let header = cookie_header(session);
    if header.is_empty() {
        builder
    } else {
        builder.header(reqwest::header::COOKIE, header)
    }
}

fn cookie_header(session: &StoredOauthSession) -> String {
    session
        .cookies
        .iter()
        .filter(|(name, value)| !name.trim().is_empty() && !value.trim().is_empty())
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("; ")
}

fn cookies_from_response(response: &reqwest::blocking::Response) -> HashMap<String, String> {
    let mut cookies = HashMap::new();
    for value in response.headers().get_all(reqwest::header::SET_COOKIE) {
        let Ok(raw) = value.to_str() else {
            continue;
        };
        if let Some((name, cookie_value)) = parse_set_cookie(raw) {
            cookies.insert(name, cookie_value);
        }
    }
    cookies
}

fn merge_response_cookies(
    session: &mut StoredOauthSession,
    response: &reqwest::blocking::Response,
) {
    for value in response.headers().get_all(reqwest::header::SET_COOKIE) {
        let Ok(raw) = value.to_str() else {
            continue;
        };
        if let Some((name, cookie_value)) = parse_set_cookie(raw) {
            if cookie_value.is_empty() || raw.to_ascii_lowercase().contains("max-age=0") {
                session.cookies.remove(&name);
            } else {
                session.cookies.insert(name, cookie_value);
            }
        }
    }
}

fn parse_set_cookie(raw: &str) -> Option<(String, String)> {
    let first = raw.split(';').next()?.trim();
    let (name, value) = first.split_once('=')?;
    let name = name.trim();
    if name.is_empty() {
        return None;
    }
    Some((name.to_string(), value.trim().to_string()))
}

fn build_gateway_ws_url_with_ticket(base_url: &str, ticket: &str) -> String {
    let ws_scheme = if base_url.starts_with("https://") {
        "wss"
    } else {
        "ws"
    };
    let ws_base = base_url
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    format!("{ws_scheme}://{ws_base}/api/ws?ticket={ticket}")
}

fn remote_connection_from_config(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
) -> Result<Option<HermesConnection>, String> {
    Ok(
        remote_backend_connection_from_config(app, state)?
            .map(|connection| connection.connection()),
    )
}

fn remote_backend_connection_from_config(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
) -> Result<Option<BackendConnection>, String> {
    let config = read_connection_config(app)?;
    if config.mode != "remote" {
        return Ok(None);
    }
    Ok(Some(remote_backend_connection(
        &config,
        Some((app, state)),
    )?))
}

fn remote_backend_connection(
    config: &DesktopConnectionConfigFile,
    runtime: Option<(&tauri::AppHandle, &tauri::State<'_, AppState>)>,
) -> Result<BackendConnection, String> {
    let auth_mode = norm_auth_mode(config.remote.auth_mode.as_deref());
    let base_url = config
        .remote
        .url
        .clone()
        .ok_or_else(|| "URL удаленного Hermes gateway не задан.".to_string())?;
    if auth_mode == "oauth" {
        let (app, state) = runtime.ok_or_else(|| {
            "OAuth gateway требует активную Tauri-сессию. Выполните вход в настройках gateway."
                .to_string()
        })?;
        let mut session = oauth_session(state, app, &base_url)?;
        let ticket = mint_gateway_ws_ticket(&base_url, &mut session)?;
        save_oauth_session(app, state, &base_url, session.clone())?;
        return Ok(BackendConnection {
            auth_mode: "oauth".to_string(),
            base_url: base_url.clone(),
            mode: "remote".to_string(),
            oauth_session: Some(session),
            pid: 0,
            python: String::new(),
            source: "settings".to_string(),
            token: String::new(),
            ws_url: build_gateway_ws_url_with_ticket(&base_url, &ticket),
        });
    }
    let token = config
        .remote
        .token
        .clone()
        .ok_or_else(|| "Session token удаленного Hermes gateway не задан.".to_string())?;
    let ws_scheme = if base_url.starts_with("https://") {
        "wss"
    } else {
        "ws"
    };
    let ws_base = base_url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .to_string();
    Ok(BackendConnection {
        auth_mode: "token".to_string(),
        base_url,
        mode: "remote".to_string(),
        oauth_session: None,
        pid: 0,
        python: String::new(),
        source: "settings".to_string(),
        token: token.clone(),
        ws_url: format!("{ws_scheme}://{ws_base}/api/ws?token={token}"),
    })
}

fn current_backend_connection(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
    profile: Option<String>,
) -> Result<BackendConnection, String> {
    if let Some(connection) = remote_backend_connection_from_config(app, state)? {
        return Ok(connection);
    }
    ensure_backend(app, state, None, profile)
}

fn backend_profile_key(profile: Option<&str>) -> String {
    profile
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("default")
        .to_string()
}

fn probe_backend_status(connection: &BackendConnection, timeout_ms: u64) -> Result<(), String> {
    connection.api(ApiRequest {
        body: None,
        method: Some("GET".to_string()),
        path: "/api/status".to_string(),
        profile: None,
        timeout_ms: Some(timeout_ms),
    })?;
    Ok(())
}

fn canonical_or_self(path: PathBuf) -> PathBuf {
    path.canonicalize().unwrap_or(path)
}

fn absolute_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| PathBuf::from("."))
    }
}

fn file_path_from_preview_url(raw_url: &str) -> Result<PathBuf, String> {
    if raw_url.trim().is_empty() {
        return Err("Пустой путь предпросмотра".to_string());
    }
    if let Ok(parsed) = reqwest::Url::parse(raw_url) {
        if parsed.scheme() == "file" {
            return parsed
                .to_file_path()
                .map_err(|_| "Не удалось преобразовать file:// URL в путь".to_string());
        }
    }
    Ok(PathBuf::from(raw_url))
}

fn file_url_for_path(path: &Path) -> String {
    reqwest::Url::from_file_path(path)
        .map(|url| url.to_string())
        .unwrap_or_else(|_| normalize_path_string(path.to_path_buf()))
}

fn normalize_path_string(path: PathBuf) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    if let Some(rest) = normalized.strip_prefix("//?/UNC/") {
        format!("//{rest}")
    } else if let Some(rest) = normalized.strip_prefix("//?/") {
        rest.to_string()
    } else {
        normalized
    }
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

fn terminal_channel(id: &str, suffix: &str) -> String {
    format!("hermes:terminal:{id}:{suffix}")
}

fn safe_terminal_cwd(cwd: Option<String>) -> Result<PathBuf, String> {
    let fallback = home_dir()
        .or_else(|| env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let candidate = cwd
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or(fallback.clone());

    if candidate.is_dir() {
        return Ok(canonical_or_self(candidate));
    }

    if candidate.is_file() {
        if let Some(parent) = candidate.parent() {
            return Ok(canonical_or_self(parent.to_path_buf()));
        }
    }

    Ok(canonical_or_self(fallback))
}

fn terminal_shell_command() -> Result<ShellSpec, String> {
    let override_shell = env::var("HERMES_DESKTOP_SHELL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            if cfg!(windows) {
                None
            } else {
                env::var("SHELL")
                    .ok()
                    .filter(|value| !value.trim().is_empty())
            }
        });

    if let Some(shell) = override_shell {
        let path = PathBuf::from(&shell);
        let resolved = if path.is_file() {
            Some(path)
        } else {
            find_on_path(&shell)
        };
        if let Some(path) = resolved {
            return Ok(shell_spec_for(path));
        }
    }

    if cfg!(windows) {
        let command = find_on_path("pwsh.exe")
            .or_else(|| find_on_path("pwsh"))
            .or_else(windows_powershell_path)
            .or_else(|| env::var("COMSPEC").ok().map(PathBuf::from))
            .unwrap_or_else(|| PathBuf::from("cmd.exe"));
        return Ok(shell_spec_for(command));
    }

    for candidate in ["/bin/zsh", "/bin/bash", "/bin/sh"] {
        let path = PathBuf::from(candidate);
        if path.is_file() {
            return Ok(shell_spec_for(path));
        }
    }

    Ok(shell_spec_for(PathBuf::from("/bin/sh")))
}

fn shell_spec_for(path: PathBuf) -> ShellSpec {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("shell")
        .to_lowercase();
    let command = path.to_string_lossy().to_string();
    let args = if name.starts_with("pwsh") || name.starts_with("powershell") {
        vec!["-NoLogo".to_string()]
    } else {
        Vec::new()
    };

    ShellSpec {
        args,
        command,
        name,
    }
}

fn terminal_shell_env() -> Vec<(String, String)> {
    let mut envs = Vec::new();
    for (key, value) in env::vars() {
        if key == "npm_config_prefix"
            || key.starts_with("npm_config_")
            || key.starts_with("npm_package_")
        {
            continue;
        }
        if matches!(key.as_str(), "NO_COLOR" | "FORCE_COLOR" | "COLORFGBG") {
            continue;
        }
        envs.push((key, value));
    }

    upsert_env(&mut envs, "COLORTERM", "truecolor");
    upsert_env(&mut envs, "LC_CTYPE", "UTF-8");
    upsert_env(&mut envs, "TERM", "xterm-256color");
    upsert_env(&mut envs, "TERM_PROGRAM", "Hermes");
    upsert_env(&mut envs, "TERM_PROGRAM_VERSION", env!("CARGO_PKG_VERSION"));
    upsert_env(&mut envs, "HERMES_DESKTOP_TERMINAL", "1");
    envs
}

fn upsert_env(envs: &mut Vec<(String, String)>, key: &str, value: &str) {
    if let Some((_, existing)) = envs.iter_mut().find(|(candidate, _)| candidate == key) {
        if existing.is_empty() || key != "LC_CTYPE" {
            *existing = value.to_string();
        }
    } else {
        envs.push((key.to_string(), value.to_string()));
    }
}

fn home_dir() -> Option<PathBuf> {
    if cfg!(windows) {
        env::var_os("USERPROFILE").map(PathBuf::from)
    } else {
        env::var_os("HOME").map(PathBuf::from)
    }
}

fn uninstall_fallback_summary(app: &tauri::AppHandle) -> serde_json::Value {
    let hermes_home = env::var_os("HERMES_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|path| path.join(".hermes")))
        .unwrap_or_else(|| PathBuf::from(".hermes"));
    let userdata_dir = app.path().app_data_dir().ok();
    let running_app_path = env::current_exe().ok();
    let agent_installed = find_python()
        .as_deref()
        .map(|python| can_import_hermes(python).is_ok())
        .unwrap_or(false);

    serde_json::json!({
        "agent_installed": agent_installed,
        "gui_installed": true,
        "hermes_home": normalize_path_string(hermes_home),
        "packaged_app_paths": [],
        "platform": env::consts::OS,
        "probe": "tauri-fallback",
        "running_app_path": running_app_path.map(normalize_path_string).unwrap_or_default(),
        "source_built_artifacts": [],
        "userdata_dir": userdata_dir.clone().map(normalize_path_string).unwrap_or_default(),
        "userdata_exists": userdata_dir.as_ref().map(|path| path.exists()).unwrap_or(false),
    })
}

fn save_image_bytes(data: &[u8], ext: &str) -> Result<String, String> {
    let dir = dirs::download_dir()
        .or_else(home_dir)
        .or_else(|| env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let extension = sanitize_extension(ext);
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let mut path = dir.join(format!("hermes-image-{stamp}.{extension}"));
    let mut counter = 1;
    while path.exists() {
        path = dir.join(format!("hermes-image-{stamp}-{counter}.{extension}"));
        counter += 1;
    }
    fs::write(&path, data).map_err(|error| format!("Не удалось сохранить изображение: {error}"))?;
    Ok(normalize_path_string(path))
}

fn sanitize_extension(ext: &str) -> String {
    let cleaned = ext
        .trim()
        .trim_start_matches('.')
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_lowercase();
    match cleaned.as_str() {
        "gif" | "jpeg" | "jpg" | "png" | "svg" | "webp" => cleaned,
        _ => "png".to_string(),
    }
}

fn image_extension_from_url_or_mime(url: &str, mime: &str) -> String {
    if let Some(ext) = Path::new(url.split('?').next().unwrap_or(url))
        .extension()
        .and_then(|value| value.to_str())
    {
        let sanitized = sanitize_extension(ext);
        if sanitized != "png" || ext.eq_ignore_ascii_case("png") {
            return sanitized;
        }
    }

    match mime.split(';').next().unwrap_or("").trim() {
        "image/gif" => "gif",
        "image/jpeg" => "jpg",
        "image/svg+xml" => "svg",
        "image/webp" => "webp",
        _ => "png",
    }
    .to_string()
}

fn ensure_log_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let path = app
        .path()
        .app_log_dir()
        .map_err(|error| format!("Не удалось определить каталог логов: {error}"))?;
    fs::create_dir_all(&path)
        .map_err(|error| format!("Не удалось создать каталог логов: {error}"))?;
    Ok(path)
}

fn read_recent_log_lines(dir: &Path, limit: usize) -> Vec<String> {
    let mut files = match fs::read_dir(dir) {
        Ok(entries) => entries
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                let modified = entry
                    .metadata()
                    .and_then(|metadata| metadata.modified())
                    .ok()?;
                Some((modified, path))
            })
            .collect::<Vec<_>>(),
        Err(error) => return vec![format!("Не удалось прочитать каталог логов: {error}")],
    };
    files.sort_by(|a, b| b.0.cmp(&a.0));

    let Some((_, path)) = files.into_iter().find(|(_, path)| path.is_file()) else {
        return Vec::new();
    };
    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) => {
            return vec![format!(
                "Не удалось прочитать лог {}: {error}",
                normalize_path_string(path)
            )]
        }
    };
    let mut lines = content
        .lines()
        .rev()
        .take(limit)
        .map(str::to_string)
        .collect::<Vec<_>>();
    lines.reverse();
    lines
}

fn update_config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("Не удалось определить каталог настроек: {error}"))?;
    Ok(dir.join("updates.json"))
}

fn read_update_branch(app: &tauri::AppHandle) -> Result<String, String> {
    let path = update_config_path(app)?;
    let raw = fs::read_to_string(path).unwrap_or_default();
    let branch = serde_json::from_str::<serde_json::Value>(&raw)
        .ok()
        .and_then(|value| {
            value
                .get("branch")
                .and_then(|branch| branch.as_str())
                .map(str::to_string)
        })
        .filter(|branch| !branch.trim().is_empty())
        .unwrap_or_else(|| "main".to_string());
    Ok(branch)
}

fn write_update_branch(app: &tauri::AppHandle, branch: &str) -> Result<(), String> {
    let path = update_config_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Не удалось создать каталог настроек: {error}"))?;
    }
    fs::write(
        path,
        serde_json::to_string_pretty(&serde_json::json!({ "branch": branch }))
            .map_err(|error| format!("Не удалось сериализовать настройки обновлений: {error}"))?,
    )
    .map_err(|error| format!("Не удалось сохранить настройки обновлений: {error}"))
}

fn source_update_root() -> Option<PathBuf> {
    let mut dir = env::current_dir().ok()?;
    loop {
        if dir.join(".git").exists() && dir.join("pyproject.toml").exists() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

fn run_git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|error| format!("Не удалось запустить git: {error}"))?;
    if !output.status.success() {
        return Err(command_error(&format!("git {}", args.join(" ")), &output));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_dirty(root: &Path) -> Result<bool, String> {
    Ok(!run_git(root, &["status", "--porcelain"])?.trim().is_empty())
}

fn git_current_sha(root: &Path) -> Result<String, String> {
    run_git(root, &["rev-parse", "HEAD"])
}

fn git_current_branch(root: &Path) -> Result<String, String> {
    run_git(root, &["branch", "--show-current"])
}

fn check_source_update(app: &tauri::AppHandle) -> Result<serde_json::Value, String> {
    let branch = read_update_branch(app)?;
    let Some(root) = source_update_root() else {
        return Ok(serde_json::json!({
            "supported": false,
            "branch": branch,
            "reason": "not-git-source",
            "message": "Tauri-приложение запущено не из git-репозитория исходников.",
            "fetchedAt": now_millis()
        }));
    };
    let current_branch = git_current_branch(&root).unwrap_or_default();
    let current_sha = git_current_sha(&root)?;
    let dirty = git_dirty(&root)?;
    let fetch_result = run_git(&root, &["fetch", "origin", &branch]);
    let target_ref = format!("origin/{branch}");
    let target_sha = run_git(&root, &["rev-parse", &target_ref]).unwrap_or_default();
    let behind = if target_sha.is_empty() {
        0
    } else {
        run_git(
            &root,
            &["rev-list", "--count", &format!("HEAD..{target_ref}")],
        )
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(0)
    };

    Ok(serde_json::json!({
        "supported": true,
        "branch": branch,
        "currentBranch": current_branch,
        "currentSha": current_sha,
        "targetSha": if target_sha.is_empty() { serde_json::Value::Null } else { serde_json::json!(target_sha) },
        "behind": behind,
        "dirty": dirty,
        "error": fetch_result.err(),
        "fetchedAt": now_millis()
    }))
}

fn check_desktop_update(app: &tauri::AppHandle) -> Result<serde_json::Value, String> {
    if source_update_root().is_some() {
        return check_source_update(app);
    }
    check_packaged_update()
}

fn apply_desktop_update(
    app: &tauri::AppHandle,
    dirty_strategy: &str,
) -> Result<serde_json::Value, String> {
    if source_update_root().is_some() {
        return apply_source_update(app, dirty_strategy);
    }
    apply_packaged_update(app)
}

fn github_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(45))
        .user_agent("Hermes-RU-Iola-Tauri-Updater")
        .build()
        .map_err(|error| format!("Не удалось создать GitHub HTTP client: {error}"))
}

fn fetch_latest_github_release() -> Result<GitHubRelease, String> {
    let response = github_client()?
        .get("https://api.github.com/repos/yasg1988/iola-hermes/releases/latest")
        .send()
        .map_err(|error| format!("Не удалось получить GitHub Release: {error}"))?;
    let status = response.status();
    let text = response
        .text()
        .map_err(|error| format!("Не удалось прочитать GitHub Release: {error}"))?;
    if !status.is_success() {
        return Err(format!("GitHub Releases вернул {status}: {text}"));
    }
    serde_json::from_str(&text).map_err(|error| format!("GitHub Release вернул не JSON: {error}"))
}

fn check_packaged_update() -> Result<serde_json::Value, String> {
    let release = fetch_latest_github_release()?;
    let current_version = env!("CARGO_PKG_VERSION");
    let asset = select_packaged_update_asset(&release.assets);
    let latest_version = asset
        .as_ref()
        .map(|asset| asset.version.clone())
        .or_else(|| release_name_version(&release.name.clone().unwrap_or_default()))
        .unwrap_or_else(|| release.tag_name.trim_start_matches('v').to_string());
    let behind = if version_is_newer(&latest_version, current_version) {
        1
    } else {
        0
    };

    Ok(serde_json::json!({
        "supported": asset.is_some(),
        "channel": "github-release",
        "currentSha": current_version,
        "targetSha": if behind > 0 { serde_json::json!(release.tag_name) } else { serde_json::Value::Null },
        "targetVersion": latest_version,
        "behind": behind,
        "releaseName": release.name,
        "releaseUrl": release.html_url,
        "assetName": asset.as_ref().map(|asset| asset.name.clone()),
        "assetSize": asset.as_ref().and_then(|asset| asset.size),
        "reason": if asset.is_some() { serde_json::Value::Null } else { serde_json::json!("no-compatible-asset") },
        "message": if asset.is_some() {
            serde_json::Value::Null
        } else {
            serde_json::json!("В последнем GitHub Release нет подходящего Tauri installer для этой платформы.")
        },
        "fetchedAt": now_millis()
    }))
}

fn apply_packaged_update(app: &tauri::AppHandle) -> Result<serde_json::Value, String> {
    emit_update_progress(
        app,
        "fetch",
        "Получаю сведения о последнем GitHub Release",
        Some(15),
        None,
    );
    let release = fetch_latest_github_release()?;
    let current_version = env!("CARGO_PKG_VERSION");
    let asset = select_packaged_update_asset(&release.assets).ok_or_else(|| {
        "В последнем GitHub Release нет подходящего installer для этой платформы.".to_string()
    })?;
    if !version_is_newer(&asset.version, current_version) {
        return Ok(serde_json::json!({
            "ok": true,
            "message": "Установленная Tauri-версия уже актуальна.",
            "currentVersion": current_version,
            "targetVersion": asset.version
        }));
    }

    emit_update_progress(
        app,
        "fetch",
        &format!("Скачиваю {}", asset.name),
        Some(35),
        None,
    );
    let path = download_update_asset(&asset)?;
    emit_update_progress(
        app,
        "restart",
        "Запускаю установщик обновления",
        Some(90),
        None,
    );
    launch_update_asset(&path)?;
    emit_update_progress(
        app,
        "restart",
        "Установщик обновления запущен. Hermes RU Iola будет закрыт.",
        Some(100),
        None,
    );
    app.exit(0);

    Ok(serde_json::json!({
        "ok": true,
        "handedOff": true,
        "updater": normalize_path_string(path),
        "targetVersion": asset.version,
        "message": "Установщик обновления запущен."
    }))
}

fn select_packaged_update_asset(assets: &[GitHubReleaseAsset]) -> Option<PackagedUpdateAsset> {
    let candidates = assets
        .iter()
        .filter_map(|asset| {
            let name = asset.name.as_str();
            if !platform_asset_matches(name) {
                return None;
            }
            let version = release_asset_version(name)?;
            Some(PackagedUpdateAsset {
                download_url: asset.browser_download_url.clone(),
                name: asset.name.clone(),
                size: asset.size,
                version,
            })
        })
        .collect::<Vec<_>>();

    candidates
        .into_iter()
        .max_by(|a, b| compare_versions(&a.version, &b.version))
}

fn platform_asset_matches(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if !lower.contains("tauri") {
        return false;
    }
    if cfg!(windows) {
        lower.ends_with(".exe") && !lower.ends_with(".exe.blockmap")
    } else if cfg!(target_os = "linux") {
        lower.ends_with(".appimage")
    } else {
        false
    }
}

fn release_asset_version(name: &str) -> Option<String> {
    let mut current = String::new();
    for ch in name.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            current.push(ch);
            continue;
        }
        if looks_like_version(&current) {
            return Some(current.trim_matches('.').to_string());
        }
        current.clear();
    }
    if looks_like_version(&current) {
        Some(current.trim_matches('.').to_string())
    } else {
        None
    }
}

fn release_name_version(name: &str) -> Option<String> {
    name.split_whitespace()
        .map(|part| part.trim_start_matches('v'))
        .find(|part| looks_like_version(part))
        .map(|part| part.trim_matches('.').to_string())
}

fn looks_like_version(value: &str) -> bool {
    let parts = value.trim_matches('.').split('.').collect::<Vec<_>>();
    parts.len() >= 3
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
}

fn version_is_newer(candidate: &str, current: &str) -> bool {
    compare_versions(candidate, current).is_gt()
}

fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let a_parts = version_parts(a);
    let b_parts = version_parts(b);
    let len = a_parts.len().max(b_parts.len());
    for index in 0..len {
        let a_value = *a_parts.get(index).unwrap_or(&0);
        let b_value = *b_parts.get(index).unwrap_or(&0);
        match a_value.cmp(&b_value) {
            std::cmp::Ordering::Equal => continue,
            ordering => return ordering,
        }
    }
    std::cmp::Ordering::Equal
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release_asset(name: &str) -> GitHubReleaseAsset {
        GitHubReleaseAsset {
            browser_download_url: format!("https://example.test/{name}"),
            name: name.to_string(),
            size: None,
        }
    }

    #[test]
    fn clamps_translucency_to_supported_range() {
        assert_eq!(clamp_translucency(-20), 0);
        assert_eq!(clamp_translucency(55), 55);
        assert_eq!(clamp_translucency(120), 100);
    }

    #[test]
    fn parses_six_digit_hex_colors() {
        let color = parse_hex_color("#112233", 200).expect("valid color");

        assert_eq!(color.0, 0x11);
        assert_eq!(color.1, 0x22);
        assert_eq!(color.2, 0x33);
        assert_eq!(color.3, 200);
    }

    #[test]
    fn rejects_invalid_hex_colors() {
        assert!(parse_hex_color("#12345", 255).is_err());
        assert!(parse_hex_color("#12xx45", 255).is_err());
        assert!(parse_hex_color("11223344", 255).is_err());
    }

    #[test]
    fn parses_html_title_with_entities() {
        let html = r#"
            <!doctype html>
            <html><head><title> Hermes &amp; Iola &#x2014; тест&nbsp;страницы </title></head></html>
        "#;

        assert_eq!(parse_html_title(html), "Hermes & Iola — тест страницы");
    }

    #[test]
    fn filters_unusable_link_titles() {
        assert_eq!(usable_link_title("Just a moment..."), "");
        assert_eq!(
            usable_link_title(" Нормальный   заголовок "),
            "Нормальный заголовок"
        );
    }

    #[test]
    fn rejects_non_fetchable_link_title_urls() {
        assert!(parse_fetchable_link_title_url("file:///tmp/test.html").is_err());
        assert!(parse_fetchable_link_title_url("http://localhost:3000").is_err());
        assert!(parse_fetchable_link_title_url("http://192.168.1.10").is_err());
        assert!(parse_fetchable_link_title_url("http://[::1]:3000").is_err());
        assert!(parse_fetchable_link_title_url("https://example.com/path").is_ok());
    }

    #[test]
    fn validates_marketplace_extension_ids() {
        assert!(valid_marketplace_extension_id(
            "dracula-theme.theme-dracula"
        ));
        assert!(valid_marketplace_extension_id("publisher_name.theme_1"));
        assert!(!valid_marketplace_extension_id("theme-only"));
        assert!(!valid_marketplace_extension_id(".theme"));
        assert!(!valid_marketplace_extension_id("publisher."));
        assert!(!valid_marketplace_extension_id("publisher.theme.extra"));
        assert!(!valid_marketplace_extension_id("publisher/theme.name"));
    }

    #[test]
    fn normalizes_marketplace_theme_entry_names() {
        assert_eq!(
            marketplace_theme_entry_name("./themes/dark.json"),
            "extension/themes/dark.json"
        );
        assert_eq!(
            marketplace_theme_entry_name(r"\themes\light.json"),
            "extension/themes/light.json"
        );
    }

    #[test]
    fn detects_icon_theme_marketplace_results() {
        let icon = serde_json::json!({
            "displayName": "Nice Icons",
            "shortDescription": "A file icon theme",
            "tags": ["theme"]
        });
        let color = serde_json::json!({
            "displayName": "Nice Dark",
            "shortDescription": "A color theme",
            "tags": ["theme"]
        });
        let tagged = serde_json::json!({
            "displayName": "Icons",
            "tags": ["icon-theme"]
        });

        assert!(looks_like_icon_theme(&icon));
        assert!(looks_like_icon_theme(&tagged));
        assert!(!looks_like_icon_theme(&color));
    }

    #[test]
    fn resolves_main_git_worktree() {
        let temp = test_temp_dir("main-worktree");
        let repo = temp.join("repo");
        fs::create_dir_all(repo.join(".git")).expect("git dir");
        fs::write(repo.join(".git").join("HEAD"), "ref: refs/heads/main\n").expect("head");
        fs::create_dir_all(repo.join("src")).expect("src");

        let info = resolve_worktree(&repo.join("src")).expect("worktree info");

        let expected_repo = normalize_path_string(repo.canonicalize().unwrap_or(repo));
        assert_eq!(info.repo_root, expected_repo);
        assert_eq!(info.worktree_root, expected_repo);
        assert!(info.is_main_worktree);
        assert_eq!(info.branch.as_deref(), Some("main"));
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn resolves_linked_git_worktree() {
        let temp = test_temp_dir("linked-worktree");
        let repo = temp.join("repo");
        let linked = temp.join("repo-feature");
        let admin = repo.join(".git").join("worktrees").join("repo-feature");
        fs::create_dir_all(repo.join(".git")).expect("main git dir");
        fs::create_dir_all(&admin).expect("linked admin");
        fs::create_dir_all(&linked).expect("linked checkout");
        fs::write(repo.join(".git").join("HEAD"), "ref: refs/heads/main\n").expect("main head");
        fs::write(admin.join("HEAD"), "ref: refs/heads/feature/rust\n").expect("linked head");
        fs::write(admin.join("commondir"), "../..\n").expect("commondir");
        fs::write(
            linked.join(".git"),
            format!("gitdir: {}\n", normalize_path_string(admin.clone())),
        )
        .expect("linked git file");

        let info = resolve_worktree(&linked).expect("worktree info");

        let expected_repo = normalize_path_string(repo.canonicalize().unwrap_or(repo));
        let expected_linked = normalize_path_string(linked.canonicalize().unwrap_or(linked));
        assert_eq!(info.repo_root, expected_repo);
        assert_eq!(info.worktree_root, expected_linked);
        assert!(!info.is_main_worktree);
        assert_eq!(info.branch.as_deref(), Some("feature/rust"));
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn reads_detached_head_short_sha() {
        let temp = test_temp_dir("detached-worktree");
        let repo = temp.join("repo");
        fs::create_dir_all(repo.join(".git")).expect("git dir");
        fs::write(
            repo.join(".git").join("HEAD"),
            "0123456789abcdef0123456789abcdef01234567\n",
        )
        .expect("head");

        let info = resolve_worktree(&repo).expect("worktree info");

        assert_eq!(info.branch.as_deref(), Some("01234567"));
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn serializes_default_project_dir_config() {
        let config = DefaultProjectDirConfig {
            dir: Some("C:/Work/Hermes".to_string()),
        };
        let raw = serde_json::to_string(&config).expect("json");
        let parsed = serde_json::from_str::<DefaultProjectDirConfig>(&raw).expect("parsed");

        assert_eq!(parsed.dir.as_deref(), Some("C:/Work/Hermes"));
    }

    #[test]
    fn validates_desktop_profile_names() {
        assert_eq!(
            normalize_desktop_profile_name(Some("default")).expect("default"),
            Some("default".to_string())
        );
        assert_eq!(
            normalize_desktop_profile_name(Some("team_1-prod")).expect("profile"),
            Some("team_1-prod".to_string())
        );
        assert_eq!(
            normalize_desktop_profile_name(Some(" ")).expect("empty"),
            None
        );
        assert!(normalize_desktop_profile_name(Some("Bad")).is_err());
        assert!(normalize_desktop_profile_name(Some("-bad")).is_err());
        assert!(normalize_desktop_profile_name(Some("bad.name")).is_err());
    }

    #[test]
    fn serializes_active_profile_config() {
        let config = DesktopActiveProfileConfig {
            profile: Some("default".to_string()),
        };
        let raw = serde_json::to_string(&config).expect("json");
        let parsed = serde_json::from_str::<DesktopActiveProfileConfig>(&raw).expect("parsed");

        assert_eq!(parsed.profile.as_deref(), Some("default"));
    }

    #[test]
    fn normalizes_backend_profile_keys() {
        assert_eq!(backend_profile_key(None), "default");
        assert_eq!(backend_profile_key(Some("")), "default");
        assert_eq!(backend_profile_key(Some("  ")), "default");
        assert_eq!(backend_profile_key(Some("team_1")), "team_1");
    }

    #[test]
    fn strips_windows_extended_path_prefix() {
        assert_eq!(
            normalize_path_string(PathBuf::from(r"\\?\C:\Users\Hermes")),
            "C:/Users/Hermes"
        );
        assert_eq!(
            normalize_path_string(PathBuf::from(r"\\?\UNC\server\share")),
            "//server/share"
        );
    }

    #[test]
    fn normalizes_local_preview_url_targets() {
        let target = preview_url_target("http://0.0.0.0:5173/index.html").expect("local url");

        assert_eq!(target.kind, "url");
        assert_eq!(target.label, "127.0.0.1/index.html");
        assert!(target.url.starts_with("http://127.0.0.1:5173/"));
        assert!(preview_url_target("https://example.com").is_none());
    }

    #[test]
    fn normalizes_preview_file_targets() {
        let temp = test_temp_dir("preview-file");
        let file = temp.join("hello.rs");
        fs::write(&file, "fn main() {}\n").expect("preview file");

        let target = preview_file_target("hello.rs", Some(temp.to_str().unwrap())).expect("target");

        assert_eq!(target.kind, "file");
        assert_eq!(target.label, "hello.rs");
        assert_eq!(target.preview_kind.as_deref(), Some("text"));
        assert_eq!(target.language.as_deref(), Some("rust"));
        assert_eq!(target.binary, Some(false));
        assert_eq!(target.byte_size, Some(13));
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn normalizes_preview_directory_to_index_html() {
        let temp = test_temp_dir("preview-dir");
        let dir = temp.join("site");
        fs::create_dir_all(&dir).expect("site dir");
        fs::write(dir.join("index.html"), "<h1>Йола</h1>").expect("index");

        let target = preview_file_target("site", Some(temp.to_str().unwrap())).expect("target");

        assert_eq!(target.label, "index.html");
        assert_eq!(target.preview_kind.as_deref(), Some("html"));
        assert_eq!(target.mime_type.as_deref(), Some("text/html"));
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn detects_binary_preview_files() {
        let temp = test_temp_dir("preview-binary");
        let file = temp.join("data.bin");
        fs::write(&file, [0, 1, 2, 3, 4]).expect("binary");

        let target = preview_file_target("data.bin", Some(temp.to_str().unwrap())).expect("target");

        assert_eq!(target.preview_kind.as_deref(), Some("binary"));
        assert_eq!(target.binary, Some(true));
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn parses_tauri_release_asset_versions() {
        assert_eq!(
            release_asset_version("Hermes-RU-Iola-Tauri-0.17.5-win-x64.exe").as_deref(),
            Some("0.17.5")
        );
        assert_eq!(
            release_asset_version("Hermes-RU-Iola-Tauri-0.17.5-linux-x86_64.AppImage").as_deref(),
            Some("0.17.5")
        );
        assert_eq!(
            release_asset_version("Hermes-RU-Iola-Tauri-0.17.5-linux-x86_64.rpm").as_deref(),
            Some("0.17.5")
        );
    }

    #[test]
    #[cfg(windows)]
    fn matches_real_windows_tauri_installer_names() {
        assert!(platform_asset_matches(
            "Hermes-RU-Iola-Tauri-0.17.5-win-x64.exe"
        ));
        assert!(!platform_asset_matches(
            "Hermes-RU-Iola-Tauri-0.17.5-win-x64.msi"
        ));
        assert!(!platform_asset_matches(
            "Hermes-RU-Iola-Tauri-0.17.5-win-x64.exe.blockmap"
        ));
        assert!(!platform_asset_matches("Hermes-RU-Iola-0.17.5-win-x64.exe"));
    }

    #[test]
    #[cfg(windows)]
    fn selects_tauri_windows_installer_from_mixed_release_assets() {
        let assets = vec![
            release_asset("Hermes-RU-Iola-0.17.5-win-x64.exe"),
            release_asset("Hermes-RU-Iola-0.17.5-win-x64.exe.blockmap"),
            release_asset("Hermes-RU-Iola-0.17.5-win-x64.msi"),
            release_asset("Hermes-RU-Iola-Tauri-0.17.5-win-x64.msi"),
            release_asset("Hermes-RU-Iola-Tauri-0.17.5-win-x64.exe"),
            release_asset("iola_hermes-0.17.5-py3-none-any.whl"),
        ];

        let selected = select_packaged_update_asset(&assets).expect("expected Tauri asset");

        assert_eq!(selected.name, "Hermes-RU-Iola-Tauri-0.17.5-win-x64.exe");
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn matches_real_linux_tauri_appimage_names() {
        assert!(platform_asset_matches(
            "Hermes-RU-Iola-Tauri-0.17.5-linux-x86_64.AppImage"
        ));
        assert!(!platform_asset_matches(
            "Hermes-RU-Iola-Tauri-0.17.5-linux-amd64.deb"
        ));
        assert!(!platform_asset_matches(
            "Hermes-RU-Iola-Tauri-0.17.5-linux-x86_64.rpm"
        ));
        assert!(!platform_asset_matches(
            "Hermes-RU-Iola-0.17.5-linux-x86_64.AppImage"
        ));
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn selects_tauri_linux_appimage_from_mixed_release_assets() {
        let assets = vec![
            release_asset("Hermes-RU-Iola-0.17.5-linux-amd64.deb"),
            release_asset("Hermes-RU-Iola-0.17.5-linux-x86_64.AppImage"),
            release_asset("Hermes-RU-Iola-0.17.5-linux-x86_64.rpm"),
            release_asset("Hermes-RU-Iola-Tauri-0.17.5-linux-amd64.deb"),
            release_asset("Hermes-RU-Iola-Tauri-0.17.5-linux-x86_64.AppImage"),
            release_asset("Hermes-RU-Iola-Tauri-0.17.5-linux-x86_64.rpm"),
            release_asset("iola_hermes-0.17.5.tar.gz"),
        ];

        let selected = select_packaged_update_asset(&assets).expect("expected Tauri asset");

        assert_eq!(
            selected.name,
            "Hermes-RU-Iola-Tauri-0.17.5-linux-x86_64.AppImage"
        );
    }
}

#[cfg(test)]
fn test_temp_dir(name: &str) -> PathBuf {
    let path = env::temp_dir().join(format!(
        "iola-hermes-tauri-{name}-{}",
        uuid::Uuid::new_v4().simple()
    ));
    fs::create_dir_all(&path).expect("temp dir");
    path
}

fn version_parts(value: &str) -> Vec<u64> {
    value
        .trim_start_matches('v')
        .split(|ch| ch == '.' || ch == '-' || ch == '+')
        .filter_map(|part| part.parse::<u64>().ok())
        .collect()
}

fn download_update_asset(asset: &PackagedUpdateAsset) -> Result<PathBuf, String> {
    let response = github_client()?
        .get(&asset.download_url)
        .send()
        .map_err(|error| format!("Не удалось скачать {}: {error}", asset.name))?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("GitHub asset {} вернул {status}", asset.name));
    }
    let bytes = response
        .bytes()
        .map_err(|error| format!("Не удалось прочитать {}: {error}", asset.name))?;
    let dir = env::temp_dir().join("hermes-ru-iola-updates");
    fs::create_dir_all(&dir)
        .map_err(|error| format!("Не удалось создать каталог обновлений: {error}"))?;
    let path = dir.join(sanitize_update_asset_name(&asset.name));
    fs::write(&path, &bytes)
        .map_err(|error| format!("Не удалось сохранить installer обновления: {error}"))?;
    make_update_asset_executable(&path)?;
    Ok(path)
}

fn sanitize_update_asset_name(name: &str) -> String {
    name.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(unix)]
fn make_update_asset_executable(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .map_err(|error| format!("Не удалось прочитать permissions installer: {error}"))?
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions)
        .map_err(|error| format!("Не удалось сделать installer исполняемым: {error}"))
}

#[cfg(not(unix))]
fn make_update_asset_executable(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn launch_update_asset(path: &Path) -> Result<(), String> {
    Command::new(path)
        .spawn()
        .map(|_| ())
        .map_err(|error| format!("Не удалось запустить installer обновления: {error}"))
}

fn apply_source_update(
    app: &tauri::AppHandle,
    dirty_strategy: &str,
) -> Result<serde_json::Value, String> {
    let branch = read_update_branch(app)?;
    let root = source_update_root()
        .ok_or_else(|| "Tauri-приложение запущено не из git-репозитория исходников.".to_string())?;
    if git_dirty(&root)? {
        if dirty_strategy == "force" {
            return Err(
                "Force update для Tauri отключен: он может потерять локальные изменения."
                    .to_string(),
            );
        }
        return Err(
            "В рабочем дереве есть локальные изменения. Сначала сохраните или закоммитьте их."
                .to_string(),
        );
    }
    emit_update_progress(app, "fetch", "Получаю обновления из GitHub", Some(25), None);
    run_git(&root, &["fetch", "origin", &branch])?;
    emit_update_progress(app, "pull", "Применяю fast-forward update", Some(65), None);
    run_git(&root, &["pull", "--ff-only", "origin", &branch])?;
    emit_update_progress(
        app,
        "restart",
        "Обновление применено, перезапустите приложение",
        Some(100),
        None,
    );
    Ok(serde_json::json!({
        "ok": true,
        "branch": branch,
        "message": "Обновление применено. Перезапустите Tauri-приложение."
    }))
}

fn emit_update_progress(
    app: &tauri::AppHandle,
    stage: &str,
    message: &str,
    percent: Option<u8>,
    error: Option<&str>,
) {
    let _ = app.emit(
        "hermes:updates:progress",
        serde_json::json!({
            "stage": stage,
            "message": message,
            "percent": percent,
            "error": error,
            "at": now_millis()
        }),
    );
}

fn now_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

#[cfg(windows)]
fn windows_powershell_path() -> Option<PathBuf> {
    let windir = env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
    let path = PathBuf::from(windir)
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    if path.is_file() {
        Some(path)
    } else {
        None
    }
}

#[cfg(not(windows))]
fn windows_powershell_path() -> Option<PathBuf> {
    None
}

fn window_state_payload(window: &WebviewWindow) -> HermesWindowState {
    HermesWindowState {
        is_fullscreen: window.is_fullscreen().unwrap_or(false),
        native_overlay_width: 138,
        window_button_position: None,
    }
}

fn emit_window_state(window: &WebviewWindow) {
    let _ = window.emit("hermes:window-state-changed", window_state_payload(window));
}

fn build_app_menu<R: tauri::Runtime>(app: &tauri::AppHandle<R>) -> tauri::Result<Menu<R>> {
    let updates = MenuItem::with_id(
        app,
        MENU_OPEN_UPDATES_ID,
        "Проверить обновления",
        true,
        None::<&str>,
    )?;
    let help = Submenu::with_items(app, "Справка", true, &[&updates])?;
    Menu::with_items(app, &[&help])
}

fn emit_open_updates_requested<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
    let _ = app.emit("hermes:open-updates", ());
}

fn install_window_state_events(window: &WebviewWindow) {
    emit_window_state(window);
    let event_window = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Focused(_)
        | WindowEvent::Resized(_)
        | WindowEvent::ScaleFactorChanged { .. }
        | WindowEvent::ThemeChanged(_) => emit_window_state(&event_window),
        _ => {}
    });
}

fn parse_deep_link_url(raw_url: &str) -> Option<DeepLinkPayload> {
    let parsed = reqwest::Url::parse(raw_url).ok()?;
    if parsed.scheme() != "hermes" {
        return None;
    }
    let kind = parsed.host_str().unwrap_or("").to_string();
    let raw_name = parsed.path().trim_start_matches('/');
    let name = percent_encoding::percent_decode_str(raw_name)
        .decode_utf8_lossy()
        .to_string();
    let params = parsed
        .query_pairs()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
    Some(DeepLinkPayload { kind, name, params })
}

fn deliver_deep_link_payload(app: &tauri::AppHandle, payload: DeepLinkPayload) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = window.emit("hermes:deep-link", payload);
    }
}

fn handle_deep_link_payload(app: &tauri::AppHandle, payload: DeepLinkPayload) {
    let deep_links = app.state::<DeepLinkState>();
    let ready = deep_links.ready.lock().map(|ready| *ready).unwrap_or(false);
    if ready {
        deliver_deep_link_payload(app, payload);
    } else if let Ok(mut pending) = deep_links.pending.lock() {
        *pending = Some(payload);
    }
}

fn handle_deep_link_url(app: &tauri::AppHandle, raw_url: &str) {
    if let Some(payload) = parse_deep_link_url(raw_url) {
        handle_deep_link_payload(app, payload);
    }
}

fn handle_deep_link_args(app: &tauri::AppHandle, args: &[String]) {
    for arg in args {
        if arg.starts_with("hermes://") {
            handle_deep_link_url(app, arg);
            break;
        }
    }
}

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            backend: Arc::new(Mutex::new(None)),
            backend_pool: Arc::new(Mutex::new(HashMap::new())),
            boot_progress: Arc::new(Mutex::new(default_boot_progress())),
            oauth_sessions: Arc::new(Mutex::new(None)),
            preview_watchers: Arc::new(Mutex::new(HashMap::new())),
            terminals: Arc::new(Mutex::new(HashMap::new())),
        })
        .manage(DeepLinkState {
            pending: Mutex::new(None),
            ready: Mutex::new(false),
        })
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            handle_deep_link_args(app, &argv);
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_deep_link::init())
        .menu(build_app_menu)
        .on_menu_event(|app, event| {
            if event.id() == MENU_OPEN_UPDATES_ID {
                emit_open_updates_requested(app);
            }
        })
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                install_window_state_events(&window);
            }
            let app_handle = app.handle().clone();
            app.deep_link().on_open_url(move |event| {
                for url in event.urls() {
                    handle_deep_link_url(&app_handle, url.as_str());
                }
            });
            if let Ok(Some(urls)) = app.deep_link().get_current() {
                for url in urls {
                    handle_deep_link_url(app.handle(), url.as_str());
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            apply_connection_config,
            backend_probe,
            backend_version,
            cancel_bootstrap,
            fetch_link_title,
            fetch_marketplace_themes,
            get_active_profile,
            get_boot_progress,
            get_connection,
            get_connection_config,
            get_default_project_dir,
            get_gateway_ws_url,
            get_recent_logs,
            get_version,
            git_root,
            hermes_api,
            oauth_login_connection_config,
            oauth_logout_connection_config,
            normalize_preview_target,
            open_new_session_window,
            open_session_window,
            open_external,
            pick_default_project_dir,
            probe_connection_config,
            read_dir,
            read_file_data_url,
            read_file_text,
            repair_bootstrap,
            revalidate_connection,
            reveal_logs,
            sanitize_workspace_cwd,
            save_clipboard_image,
            save_image_buffer,
            save_image_from_url,
            save_connection_config,
            search_marketplace_themes,
            select_paths,
            set_active_profile,
            set_default_project_dir,
            set_native_theme,
            set_title_bar_theme,
            set_translucency,
            signal_deep_link_ready,
            start_backend,
            reset_bootstrap,
            stop_preview_file_watch,
            terminal_dispose,
            terminal_resize,
            terminal_start,
            terminal_write,
            test_connection_config,
            touch_backend,
            uninstall_run,
            uninstall_summary,
            updates_apply,
            updates_check,
            updates_get_branch,
            updates_set_branch,
            watch_preview_file,
            worktrees,
            write_clipboard,
        ])
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_dialog::init())
        .run(tauri::generate_context!())
        .expect("failed to run Hermes RU Iola");
}
