use serde::Serialize;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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
fn start_backend(host: Option<String>, port: Option<u16>) -> Result<BackendProcess, String> {
    let python = find_python().ok_or_else(|| "Python 3.11-3.14 не найден".to_string())?;
    can_import_hermes(&python)?;

    let host = host.unwrap_or_else(|| "127.0.0.1".to_string());
    let port = port.unwrap_or(9119);
    let mut child = Command::new(&python);
    child.args([
        "-m",
        "hermes_cli.main",
        "dashboard",
        "--no-open",
        "--tui",
        "--host",
        &host,
        "--port",
        &port.to_string(),
    ]);
    child.stdin(Stdio::null());
    child.stdout(Stdio::null());
    child.stderr(Stdio::null());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        child.creation_flags(0x08000000);
    }

    let child = child
        .spawn()
        .map_err(|error| format!("Не удалось запустить Hermes dashboard: {error}"))?;

    Ok(BackendProcess {
        pid: child.id(),
        python: python.to_string_lossy().into_owned(),
        url: format!("http://{host}:{port}"),
    })
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

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            backend_probe,
            backend_version,
            start_backend
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Hermes RU Iola Tauri");
}
