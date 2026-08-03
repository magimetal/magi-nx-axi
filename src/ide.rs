use serde_json::Value;
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Command;

fn socket_paths(root: &Path) -> Vec<PathBuf> {
    for key in ["NX_SOCKET_DIR", "NX_DAEMON_SOCKET_DIR"] {
        if let Ok(value) = std::env::var(key) {
            let path = PathBuf::from(value);
            return vec![if path.is_absolute() {
                path.join("nx-console.sock")
            } else {
                root.join(path).join("nx-console.sock")
            }];
        }
    }
    let mut paths = Vec::new();
    let script = "const {hashArray}=require('nx/src/native');process.stdout.write(hashArray([process.argv[1].toLowerCase(),'nx-console']))";
    if let Some(hash) = Command::new("node")
        .args(["-e", script, &root.to_string_lossy()])
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .filter(|hash| !hash.trim().is_empty())
    {
        paths.push(
            std::env::temp_dir()
                .join(hash.trim())
                .join("nx-console.sock"),
        );
    }
    paths.push(root.join(".nx/workspace-data/d/nx-console.sock"));
    paths
}

pub fn call(
    root: &Path,
    method: &str,
    params: Option<Value>,
    request: bool,
) -> Result<Value, String> {
    let paths = socket_paths(root);
    let mut stream = paths
        .iter()
        .find_map(|path| UnixStream::connect(path).ok())
        .ok_or_else(|| {
            format!(
                "Nx Console IDE is unavailable for this workspace; checked {}. Start Nx Console, then retry",
                paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ")
            )
        })?;
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .map_err(|e| format!("configure IDE timeout: {e}"))?;
    let mut message = serde_json::json!({"jsonrpc":"2.0","method":method});
    if request {
        message["id"] = 1.into();
    }
    if let Some(params) = params {
        message["params"] = params;
    }
    let body = serde_json::to_vec(&message).map_err(|e| e.to_string())?;
    write!(stream, "Content-Length: {}\r\n\r\n", body.len()).map_err(|e| e.to_string())?;
    stream
        .write_all(&body)
        .map_err(|e| format!("write IDE request: {e}"))?;
    if !request {
        return Ok(serde_json::json!({"sent":true,"method":method}));
    }
    let mut header = Vec::new();
    let mut byte = [0u8; 1];
    while !header.ends_with(b"\r\n\r\n") {
        stream
            .read_exact(&mut byte)
            .map_err(|e| format!("read IDE response header: {e}"))?;
        header.push(byte[0]);
        if header.len() > 8192 {
            return Err("IDE response header exceeded 8 KiB".into());
        }
    }
    let header = String::from_utf8(header).map_err(|_| "IDE response header is not UTF-8")?;
    let length: usize = header
        .lines()
        .find_map(|line| {
            line.split_once(':')
                .filter(|(name, _)| name.eq_ignore_ascii_case("Content-Length"))
        })
        .and_then(|(_, value)| value.trim().parse().ok())
        .ok_or("IDE response has no valid Content-Length")?;
    if length > 16 * 1024 * 1024 {
        return Err("IDE response exceeded 16 MiB".into());
    }
    let mut body = vec![0; length];
    stream
        .read_exact(&mut body)
        .map_err(|e| format!("read IDE response: {e}"))?;
    let value: Value =
        serde_json::from_slice(&body).map_err(|e| format!("invalid IDE JSON-RPC response: {e}"))?;
    if value.get("id") != Some(&json!(1)) {
        return Err("IDE JSON-RPC response ID did not match request".into());
    }
    if let Some(error) = value.get("error") {
        return Err(format!(
            "IDE rejected request: {}",
            error
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
        ));
    }
    Ok(value)
}

use serde_json::json;

pub fn notify_graph(
    root: &Path,
    kind: &str,
    project: Option<&str>,
    task: Option<&str>,
) -> Result<Value, String> {
    let (method, params) = match kind {
        "full-project-graph" => ("ide/showFullProjectGraph", json!({})),
        "project" => (
            "ide/focusProject",
            json!({"projectName": project.ok_or("--project is required for project graph")?}),
        ),
        "project-task" => (
            "ide/focusTask",
            json!({
                "projectName": project.ok_or("--project is required for project-task graph")?,
                "taskName": task.ok_or("--task is required for project-task graph")?
            }),
        ),
        _ => return Err(format!("unknown graph kind: {kind}")),
    };
    call(root, method, Some(params), false)
}

pub fn task_output(value: &Value, requested: &str) -> Result<(String, Value), String> {
    let tasks = value
        .pointer("/result/runningTasks")
        .and_then(Value::as_object)
        .ok_or("IDE returned no running-task map")?;
    if let Some(task) = tasks.get(requested) {
        return Ok((requested.into(), task.clone()));
    }
    let matches: Vec<_> = tasks
        .iter()
        .filter(|(id, task)| {
            id.contains(requested)
                || task
                    .get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|name| name.contains(requested))
        })
        .collect();
    match matches.as_slice() {
        [(id, task)] => Ok(((*id).clone(), (*task).clone())),
        [] => Err(format!("no running task matches {requested}")),
        many => Err(format!(
            "task reference is ambiguous: {requested}; matches {}",
            many.iter()
                .map(|(id, _)| id.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}
