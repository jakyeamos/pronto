//! Bounded adapter to the installed canonical Compass producer. No repository code runs.
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const MAX_OUTPUT: u64 = 1024 * 1024;

fn helper_path() -> Result<PathBuf, String> {
    // An explicit operator override supports isolated candidate verification.
    let path = std::env::var_os("PRONTO_COMPASS_HELPER")
        .map(PathBuf::from)
        .or_else(|| {
            dirs::home_dir()
                .map(|home| home.join(".agents/skills/project-compass/scripts/project_compass.py"))
        })
        .ok_or("Cannot resolve canonical Compass helper")?;
    if !path.is_absolute()
        || !path.is_file()
        || path.extension().and_then(|v| v.to_str()) != Some("py")
    {
        return Err(
            "Canonical Compass helper missing; install the versioned project-compass skill".into(),
        );
    }
    Ok(path)
}

pub fn invoke(operation: &str, repository: &Path, arguments: &[String]) -> Result<Value, String> {
    if !matches!(operation, "family" | "change-context" | "assess" | "gate") {
        return Err(
            "Pronto Compass supports family, change-context, gate, and assess operations"
                .into(),
        );
    }
    let helper = helper_path()?;
    let python = std::env::var("PRONTO_PYTHON").unwrap_or_else(|_| {
        if Path::new("/usr/local/bin/python3").is_file() {
            "/usr/local/bin/python3".into()
        } else {
            "python3".into()
        }
    });
    let mut child = Command::new(python)
        .arg(helper)
        .arg(operation)
        .arg(repository)
        .args(arguments)
        .arg("--json")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("Cannot start Compass producer: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or("Compass producer has no output stream")?;
    // Drain concurrently: a pipe-sized family must not deadlock the deadline loop.
    let reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout
            .take(MAX_OUTPUT + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes)
    });
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Compass producer exceeded deadline or could not be waited for".into());
            }
        }
    };
    let bytes = reader
        .join()
        .map_err(|_| "Compass output reader failed")?
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_OUTPUT {
        return Err("Compass projection exceeds 1 MiB; narrow the requested scope".into());
    }
    let payload: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Invalid Compass output: {error}"))?;
    if !status.success() && status.code() != Some(2) {
        return Err(payload["error"]
            .as_str()
            .unwrap_or("Compass producer failed")
            .into());
    }
    let expected = match operation {
        "family" if arguments.iter().any(|arg| arg == "--summary") => "compass-family-summary/v1",
        "family" => "compass-family/v1",
        "change-context" | "gate" => "compass-change-context/v1",
        _ => "compass-modernization-assessment/v1",
    };
    let artifact_requested = arguments
        .iter()
        .any(|arg| arg == "--packet-output" || arg.starts_with("--packet-output="));
    let additive_schema = (operation == "gate" && payload["schema"] == "compass-gate-summary/v1")
        || (operation == "change-context"
            && artifact_requested
            && payload["schema"] == "compass-packet-artifact/v1");
    if payload["schema"] != expected && !additive_schema {
        return Err(format!("Unsupported projection; expected {expected}"));
    }
    Ok(payload)
}

pub fn run_cli(arguments: &[String]) {
    let result = (|| {
        let operation = arguments.first().ok_or("Usage: pronto compass <family|change-context|gate|assess> <workspace> [helper options] --json")?;
        let repository = arguments
            .get(1)
            .ok_or("Compass requires an explicit workspace path")?;
        let extras: Vec<String> = arguments[2..]
            .iter()
            .filter(|arg| arg.as_str() != "--json")
            .cloned()
            .collect();
        invoke(operation, Path::new(repository), &extras)
    })();
    match result {
        Ok(payload) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_default()
            );
            if payload["phase"] == "completion" && payload["eligible"] != true {
                std::process::exit(2);
            }
        }
        Err(error) => {
            println!(
                "{}",
                serde_json::json!({"status":"unavailable", "error":error, "execution_authority":false})
            );
            std::process::exit(1);
        }
    }
}

#[tauri::command]
pub fn open_compass_source(workspace: String, source_ref: String) -> Result<(), String> {
    let root = Path::new(&workspace)
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let path = root
        .join(&source_ref)
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let compass_root = root
        .join(".project-compass")
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if !compass_root.starts_with(&root)
        || !path.starts_with(&compass_root)
        || !path.is_file()
        || path.extension().and_then(|v| v.to_str()) != Some("json")
    {
        return Err("Compass sources must be existing files inside this workspace's .project-compass directory".into());
    }
    #[cfg(target_os = "macos")]
    {
        let status = Command::new("/usr/bin/open")
            .arg(path)
            .status()
            .map_err(|error| error.to_string())?;
        if !status.success() {
            return Err("Could not open Compass source".into());
        }
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err("Source opening is available on macOS".into())
    }
}
