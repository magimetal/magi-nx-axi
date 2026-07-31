use flate2::read::GzDecoder;
use regex::Regex;
use serde_json::{Value, json};
use std::{fs, io::Read, path::Path};

fn nx_json(root: &Path) -> Result<Value, String> {
    let path = root.join("nx.json");
    let text = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))
}

fn nested_runner<'a>(nx: &'a Value, key: &str) -> Option<&'a Value> {
    nx.get("tasksRunnerOptions")?
        .as_object()?
        .values()
        .find_map(|runner| runner.pointer(&format!("/options/{key}")))
}

fn endpoint(root: &Path, override_url: Option<&str>) -> Result<String, String> {
    let nx = nx_json(root)?;
    let raw = override_url
        .map(str::to_owned)
        .or_else(|| std::env::var("NX_CLOUD_API").ok())
        .or_else(|| {
            nx.get("nxCloudUrl")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .or_else(|| {
            nested_runner(&nx, "url")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "https://cloud.nx.app".into());
    if raw.trim().is_empty() {
        return Err("Nx Cloud endpoint cannot be empty".into());
    }
    let parsed = url::Url::parse(&raw).map_err(|e| format!("invalid Nx Cloud endpoint: {e}"))?;
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("Nx Cloud endpoint must not contain credentials".into());
    }
    let loopback = matches!(
        parsed.host_str(),
        Some("localhost") | Some("127.0.0.1") | Some("::1")
    );
    if parsed.scheme() != "https" && !(parsed.scheme() == "http" && loopback) {
        return Err(
            "Nx Cloud endpoint must use HTTPS; HTTP is allowed only for loopback tests".into(),
        );
    }
    Ok(raw.trim_end_matches('/').into())
}

#[derive(Default)]
struct Credentials {
    access_token: Option<String>,
    cloud_id: Option<String>,
    personal_token: Option<String>,
}

fn credentials(root: &Path) -> Result<Credentials, String> {
    let nx = nx_json(root)?;
    let access_token = std::env::var("NX_CLOUD_AUTH_TOKEN")
        .ok()
        .or_else(|| std::env::var("NX_CLOUD_ACCESS_TOKEN").ok())
        .or_else(|| {
            nx.get("nxCloudAccessToken")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .or_else(|| {
            nested_runner(&nx, "accessToken")
                .and_then(Value::as_str)
                .map(str::to_owned)
        });
    let cloud_id = std::env::var("NX_CLOUD_ID")
        .ok()
        .or_else(|| {
            nx.get("nxCloudId")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .or_else(|| {
            nested_runner(&nx, "nxCloudId")
                .and_then(Value::as_str)
                .map(str::to_owned)
        });
    let personal_token = std::env::var("NX_CLOUD_PERSONAL_ACCESS_TOKEN").ok();
    for (label, value) in [
        ("Nx Cloud access token", access_token.as_deref()),
        ("Nx Cloud ID", cloud_id.as_deref()),
        ("Nx Cloud personal token", personal_token.as_deref()),
    ] {
        if value == Some("") {
            return Err(format!("{label} cannot be empty"));
        }
    }
    Ok(Credentials {
        access_token,
        cloud_id,
        personal_token,
    })
}

fn request(
    root: &Path,
    override_url: Option<&str>,
    path: &str,
    body: Option<Value>,
) -> Result<Value, String> {
    let url = format!("{}{}", endpoint(root, override_url)?, path);
    let secrets = credentials(root)?;
    let response = match body {
        Some(body) => {
            let mut request = ureq::post(&url).header("Content-Type", "application/json");
            if let Some(value) = &secrets.access_token {
                request = request.header("Authorization", value);
            }
            if let Some(value) = &secrets.cloud_id {
                request = request.header("Nx-Cloud-Id", value);
            }
            if let Some(value) = &secrets.personal_token {
                request = request.header("Nx-Cloud-Personal-Access-Token", value);
            }
            request.send_json(body)
        }
        None => {
            let mut request = ureq::get(&url).header("Content-Type", "application/json");
            if let Some(value) = &secrets.access_token {
                request = request.header("Authorization", value);
            }
            if let Some(value) = &secrets.cloud_id {
                request = request.header("Nx-Cloud-Id", value);
            }
            if let Some(value) = &secrets.personal_token {
                request = request.header("Nx-Cloud-Personal-Access-Token", value);
            }
            request.call()
        }
    }
    .map_err(|error| {
        let mut message = format!("Nx Cloud request failed: {error}");
        for secret in [
            secrets.access_token,
            secrets.cloud_id,
            secrets.personal_token,
        ]
        .into_iter()
        .flatten()
        {
            message = message.replace(&secret, "[redacted]");
        }
        message
    })?;
    let bytes = response
        .into_body()
        .with_config()
        .limit(32 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| format!("read Nx Cloud response: {e}"))?;
    if bytes.len() > 32 * 1024 * 1024 {
        return Err("Nx Cloud response exceeded 32 MiB safety limit".into());
    }
    serde_json::from_slice(&bytes).map_err(|e| format!("Nx Cloud returned invalid JSON: {e}"))
}

pub fn recent(root: &Path, endpoint: Option<&str>, branch: Option<&str>) -> Result<Value, String> {
    request(
        root,
        endpoint,
        "/nx-cloud/nx-console/ci-pipeline-executions",
        Some(json!({"branches": branch.into_iter().collect::<Vec<_>>()})),
    )
}

pub fn pipeline(root: &Path, endpoint: Option<&str>, id: &str) -> Result<Value, String> {
    request(
        root,
        endpoint,
        &format!("/nx-cloud/mcp-context/pipeline-executions/{id}"),
        None,
    )
}

pub fn run(root: &Path, endpoint: Option<&str>, id: &str) -> Result<Value, String> {
    request(
        root,
        endpoint,
        &format!("/nx-cloud/mcp-context/runs/{id}"),
        None,
    )
}

fn safe_artifact_url(raw: &str) -> Result<String, String> {
    let parsed = url::Url::parse(raw).map_err(|e| format!("invalid Nx Cloud artifact URL: {e}"))?;
    let loopback = matches!(
        parsed.host_str(),
        Some("localhost") | Some("127.0.0.1") | Some("::1")
    );
    if parsed.scheme() != "https" && !(parsed.scheme() == "http" && loopback) {
        return Err(
            "Nx Cloud artifact URL must use HTTPS; HTTP is allowed only for loopback tests".into(),
        );
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err("Nx Cloud artifact URL must not contain credentials".into());
    }
    Ok(raw.into())
}

pub fn terminal(
    root: &Path,
    endpoint: Option<&str>,
    task: &str,
    run: &str,
) -> Result<String, String> {
    let response = request(
        root,
        endpoint,
        "/nx-cloud/nx-console/ci-pipeline-executions/terminal-outputs",
        Some(json!({"taskId":task,"linkId":run,"executionId":run})),
    )?;
    let artifact = response
        .get("artifactUrl")
        .and_then(Value::as_str)
        .ok_or("Nx Cloud returned no terminal-output artifact URL")?;
    let artifact = safe_artifact_url(artifact)?;
    let compressed = ureq::get(&artifact)
        .header("Accept", "*/*")
        .call()
        .map_err(|e| format!("download terminal output: {e}"))?
        .into_body()
        .with_config()
        .limit(32 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| format!("read terminal output: {e}"))?;
    if compressed.len() > 32 * 1024 * 1024 {
        return Err("terminal-output artifact exceeded 32 MiB compressed limit".into());
    }
    let decoder = GzDecoder::new(compressed.as_slice());
    let mut archive = tar::Archive::new(decoder);
    let mut output = String::new();
    for entry in archive
        .entries()
        .map_err(|e| format!("read terminal-output archive: {e}"))?
    {
        let entry = entry.map_err(|e| format!("read terminal-output entry: {e}"))?;
        let normalized = entry
            .path()
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        if normalized.starts_with("terminalOutputs/") && entry.header().entry_type().is_file() {
            let remaining = (64 * 1024 * 1024usize).saturating_sub(output.len());
            let mut bytes = Vec::new();
            entry
                .take(remaining.saturating_add(1) as u64)
                .read_to_end(&mut bytes)
                .map_err(|e| format!("decode terminal output: {e}"))?;
            if bytes.len() > remaining {
                return Err("terminal output exceeded 64 MiB decompressed limit".into());
            }
            output.push_str(&String::from_utf8(bytes).map_err(|_| "terminal output is not UTF-8")?);
        }
    }
    let ansi = Regex::new(r"\x1b\[[0-?]*[ -/]*[@-~]").map_err(|e| e.to_string())?;
    Ok(ansi.replace_all(&output, "").into_owned())
}

pub fn retrieve_fix(
    root: &Path,
    endpoint: Option<&str>,
    short_link: &str,
) -> Result<Value, String> {
    let parts: Vec<_> = short_link.split('-').collect();
    if parts.len() != 2 || parts.iter().any(|part| part.is_empty()) {
        return Err("short link must have `<fix>-<suggestion>` form".into());
    }
    request(
        root,
        endpoint,
        "/nx-cloud/retrieve-fix-diff",
        Some(json!({"fixShortLink":parts[0],"suggestionShortLink":parts[1]})),
    )
}

pub fn update(
    root: &Path,
    endpoint: Option<&str>,
    id: &str,
    action: &str,
) -> Result<Value, String> {
    let action = match action {
        "APPLY" => "APPLIED",
        "REJECT" => "REJECTED",
        "RERUN_ENVIRONMENT_STATE" => "RERUN_REQUESTED",
        _ => return Err(format!("invalid self-healing action: {action}")),
    };
    request(
        root,
        endpoint,
        "/nx-cloud/update-suggested-fix",
        Some(json!({"aiFixId":id,"action":action,"actionOrigin":"NX_CLI"})),
    )
}
