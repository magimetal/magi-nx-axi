use assert_cmd::Command;
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    os::unix::{fs::PermissionsExt, net::UnixListener},
    path::Path,
    thread,
};

fn workspace() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("nx.json"),
        r#"{"nxCloudId":"cloud-id","nxCloudAccessToken":"workspace-token"}"#,
    )
    .unwrap();
    fs::write(
        temp.path().join("package.json"),
        r#"{"devDependencies":{"@nx/js":"1.0.0"}}"#,
    )
    .unwrap();
    let bin = temp.path().join("node_modules/.bin");
    fs::create_dir_all(&bin).unwrap();
    let script = bin.join("nx");
    fs::write(
        &script,
        r##"#!/bin/sh
cat <<'JSON'
{"graph":{"nodes":{"app":{"data":{"root":"apps/app","projectType":"application","tags":["type:app"],"targets":{"build":{"executor":"@nx/js:tsc"}},"description":"application"}},"lib":{"data":{"root":"libs/lib","projectType":"library","tags":["type:lib"],"targets":{"test":{"executor":"@nx/jest:jest"}}}}},"dependencies":{"app":[{"source":"app","target":"lib","type":"static"}],"lib":[]}}}
JSON
"##,
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    temp
}

fn command(root: &Path) -> Command {
    let mut command = Command::cargo_bin("magi-nx-axi").unwrap();
    command.args(["--format", "json", "--workspace"]);
    command.arg(root);
    command
}

fn json_output(mut command: Command) -> Value {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    serde_json::from_slice(&output.stdout).unwrap()
}

fn read_http(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut chunk = [0; 4096];
    loop {
        let count = stream.read(&mut chunk).unwrap();
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..count]);
        if let Some(end) = bytes.windows(4).position(|x| x == b"\r\n\r\n") {
            let header_end = end + 4;
            let headers = String::from_utf8_lossy(&bytes[..header_end]);
            let length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            while bytes.len() < header_end + length {
                let count = stream.read(&mut chunk).unwrap();
                if count == 0 {
                    break;
                }
                bytes.extend_from_slice(&chunk[..count]);
            }
            break;
        }
    }
    String::from_utf8(bytes).unwrap()
}

fn respond(stream: &mut TcpStream, body: &[u8], content_type: &str) {
    write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len()).unwrap();
    stream.write_all(body).unwrap();
}

#[test]
fn home_outside_workspace_is_useful() {
    let temp = tempfile::tempdir().unwrap();
    let mut command = Command::cargo_bin("magi-nx-axi").unwrap();
    command.args(["--format", "json"]).current_dir(temp.path());
    let value = json_output(command);
    assert_eq!(value["workspace"]["found"], false);
    assert!(value["bin"].as_str().unwrap().contains("magi-nx-axi"));
}

#[test]
fn help_version_and_usage_have_axi_exit_contract() {
    Command::cargo_bin("magi-nx-axi")
        .unwrap()
        .arg("--help")
        .assert()
        .success();
    Command::cargo_bin("magi-nx-axi")
        .unwrap()
        .arg("--version")
        .assert()
        .success();
    let output = Command::cargo_bin("magi-nx-axi")
        .unwrap()
        .args(["--format", "json", "--bad"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stderr.is_empty());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["error"]["type"], "usage");
}

#[test]
fn home_reports_workspace_when_local_nx_is_not_installed() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("nx.json"), "{}").unwrap();
    let mut command = Command::cargo_bin("magi-nx-axi").unwrap();
    command.args(["--format", "json"]).current_dir(temp.path());
    let value = json_output(command);
    assert_eq!(value["workspace"]["found"], true);
    assert_eq!(value["workspace"]["analysis"], "unavailable");
}

#[test]
fn workspace_lists_graph_dependencies_and_nx_json() {
    let root = workspace();
    let mut cmd = command(root.path());
    cmd.arg("workspace");
    let value = json_output(cmd);
    assert_eq!(value["projects"]["total"], 2);
    assert_eq!(value["dependencies"]["app"][0]["target"], "lib");
    assert_eq!(value["nxJson"]["nxCloudId"], "cloud-id");
}

#[test]
fn workspace_filter_and_selection_are_applied() {
    let root = workspace();
    let mut cmd = command(root.path());
    cmd.args(["workspace", "--filter", "tag:type:lib", "--select", "root"]);
    let value = json_output(cmd);
    assert_eq!(value["projects"]["count"], 1);
    assert_eq!(value["projects"]["items"][0]["value"], "libs/lib");
}

#[test]
fn project_detail_and_missing_selection_are_unambiguous() {
    let root = workspace();
    let mut cmd = command(root.path());
    cmd.args(["project", "app", "--select", "targets.build.executor"]);
    assert_eq!(json_output(cmd)["project"], "@nx/js:tsc");
    let mut bad = command(root.path());
    bad.args(["project", "app", "--select", "missing"]);
    let output = bad.output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("not found")
    );
}

fn generator_fixture(root: &Path) {
    let generator = root.join("tools/generators/widget");
    fs::create_dir_all(&generator).unwrap();
    fs::write(root.join("tools/generators/collection.json"), r#"{"generators":{"widget":{"description":"Create widget","schema":"widget/schema.json","aliases":["w"]}}}"#).unwrap();
    fs::write(
        generator.join("schema.json"),
        r#"{"$schema":"x","$id":"y","type":"object","properties":{"name":{"type":"string"}}}"#,
    )
    .unwrap();
    fs::write(generator.join("examples.md"), "nx g workspace:widget demo").unwrap();
}

#[test]
fn generators_schema_alias_and_examples_use_filesystem() {
    let root = workspace();
    generator_fixture(root.path());
    let mut list = command(root.path());
    list.arg("generators");
    assert_eq!(
        json_output(list)["generators"][0]["name"],
        "workspace:widget"
    );
    let mut schema = command(root.path());
    schema.args(["generator-schema", "w"]);
    let schema = json_output(schema);
    assert_eq!(schema["schema"]["name"], "workspace:widget");
    assert!(schema["schema"].get("$schema").is_none());
    let mut examples = command(root.path());
    examples.args(["generator-examples", "workspace:widget"]);
    assert!(
        json_output(examples)["examples"]
            .as_str()
            .unwrap()
            .contains("nx g")
    );
}

#[test]
fn docs_uses_direct_endpoint_and_caps_default_sections() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_http(&mut stream);
        assert!(request.contains("query-ai-embeddings") && request.contains("workspace graph"));
        let sections: Vec<_> = (0..5)
            .map(|i| json!({"heading":format!("h{i}"),"content":"c","similarity":1}))
            .collect();
        respond(
            &mut stream,
            serde_json::to_string(&json!({"context":{"pageSections":sections}}))
                .unwrap()
                .as_bytes(),
            "application/json",
        );
    });
    let mut command = Command::cargo_bin("magi-nx-axi").unwrap();
    command
        .args(["--format", "json", "docs", "workspace graph"])
        .env("NX_AXI_DOCS_URL", endpoint);
    let value = json_output(command);
    assert_eq!(value["count"], 4);
    assert_eq!(value["total"], 5);
    handle.join().unwrap();
}

#[test]
fn plugin_catalog_combines_installed_official_and_community() {
    let root = workspace();
    let package = root.path().join("node_modules/@nx/js");
    fs::create_dir_all(&package).unwrap();
    fs::write(
        package.join("package.json"),
        r#"{"description":"JS","generators":"generators.json"}"#,
    )
    .unwrap();
    fs::write(package.join("generators.json"), r#"{"generators":{}}"#).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        for body in [
            r#"[{"name":"react","description":"React"}]"#,
            r#"[{"name":"acme-plugin","description":"Acme","url":"https://example.com"}]"#,
        ] {
            let (mut stream, _) = listener.accept().unwrap();
            read_http(&mut stream);
            respond(&mut stream, body.as_bytes(), "application/json");
        }
    });
    let mut cmd = command(root.path());
    cmd.arg("plugins")
        .env("NX_AXI_OFFICIAL_PLUGINS_URL", format!("{base}/official"))
        .env("NX_AXI_COMMUNITY_PLUGINS_URL", format!("{base}/community"));
    let value = json_output(cmd);
    assert!(
        value["plugins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"] == "@nx/react")
    );
    assert!(
        value["plugins"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["name"] == "acme-plugin")
    );
    handle.join().unwrap();
}

#[test]
fn ide_task_output_reads_one_framed_response_and_pages_unicode() {
    let root = workspace();
    let socket_dir = root.path().join("socket");
    fs::create_dir_all(&socket_dir).unwrap();
    let listener = UnixListener::bind(socket_dir.join("nx-console.sock")).unwrap();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut header = Vec::new();
        let mut byte = [0; 1];
        while !header.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            header.push(byte[0]);
        }
        let length: usize = String::from_utf8(header)
            .unwrap()
            .lines()
            .find_map(|line| {
                line.strip_prefix("Content-Length:")
                    .and_then(|x| x.trim().parse().ok())
            })
            .unwrap();
        let mut body = vec![0; length];
        stream.read_exact(&mut body).unwrap();
        assert!(
            String::from_utf8(body)
                .unwrap()
                .contains("ide/getRunningTasks")
        );
        let response = serde_json::to_vec(&json!({"jsonrpc":"2.0","id":1,"result":{"runningTasks":{"app:build":{"name":"app:build","status":"Success","output":"héllo","continuous":false}}}})).unwrap();
        write!(stream, "Content-Length: {}\r\n\r\n", response.len()).unwrap();
        stream.write_all(&response).unwrap();
    });
    let mut cmd = command(root.path());
    cmd.args(["task-output", "build"])
        .env("NX_SOCKET_DIR", &socket_dir);
    let value = json_output(cmd);
    assert_eq!(value["task"], "app:build");
    assert_eq!(value["output"]["content"], "héllo");
    handle.join().unwrap();
}

#[test]
fn graph_sends_exact_ide_notification() {
    let root = workspace();
    let socket_dir = root.path().join("socket");
    fs::create_dir_all(&socket_dir).unwrap();
    let listener = UnixListener::bind(socket_dir.join("nx-console.sock")).unwrap();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut text = String::new();
        stream.read_to_string(&mut text).unwrap();
        assert!(text.contains("ide/focusTask") && text.contains("app") && text.contains("build"));
    });
    let mut cmd = command(root.path());
    cmd.args([
        "graph",
        "project-task",
        "--project",
        "app",
        "--task",
        "build",
    ])
    .env("NX_SOCKET_DIR", &socket_dir);
    assert_eq!(json_output(cmd)["visualization"]["opened"], true);
    handle.join().unwrap();
}

#[test]
fn cloud_recent_sends_scope_and_credentials_without_echoing_secret() {
    let root = workspace();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_http(&mut stream);
        assert!(
            request.contains("ci-pipeline-executions")
                && request.to_ascii_lowercase().contains("workspace-token")
                && request.contains("main"),
            "{request}"
        );
        respond(
            &mut stream,
            br#"{"ciPipelineExecutions":[]}"#,
            "application/json",
        );
    });
    let mut cmd = command(root.path());
    cmd.args(["--cloud-url", &endpoint, "cipes", "--branch", "main"]);
    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("workspace-token"));
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["count"],
        0
    );
    handle.join().unwrap();
}

#[test]
fn self_healing_maps_action_before_single_mutation() {
    let root = workspace();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let request = read_http(&mut stream);
        assert!(
            request.contains("update-suggested-fix")
                && request.contains("APPLIED")
                && request.contains("fix-id"),
            "{request}"
        );
        respond(&mut stream, br#"{"success":true}"#, "application/json");
    });
    let mut cmd = command(root.path());
    cmd.args([
        "--cloud-url",
        &endpoint,
        "self-healing",
        "--id",
        "fix-id",
        "APPLY",
    ]);
    assert_eq!(json_output(cmd)["success"], true);
    handle.join().unwrap();
}

#[test]
fn self_healing_rejects_provider_declared_failure() {
    let root = workspace();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        read_http(&mut stream);
        respond(
            &mut stream,
            br#"{"success":false,"error":{"message":"not allowed"}}"#,
            "application/json",
        );
    });
    let mut cmd = command(root.path());
    cmd.args([
        "--cloud-url",
        &endpoint,
        "self-healing",
        "--id",
        "fix-id",
        "REJECT",
    ]);
    let output = cmd.output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        value["error"]["message"]
            .as_str()
            .unwrap()
            .contains("not allowed")
    );
    handle.join().unwrap();
}

fn terminal_archive() -> Vec<u8> {
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    {
        let mut archive = tar::Builder::new(&mut gzip);
        let content = b"\x1b[31mfailed\x1b[0m\nlast line\n";
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        archive
            .append_data(&mut header, "terminalOutputs/output.log", &content[..])
            .unwrap();
        archive.finish().unwrap();
    }
    gzip.finish().unwrap()
}

#[test]
fn cloud_task_output_downloads_extracts_strips_and_pages_artifact() {
    let root = workspace();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let artifact_url = format!("{endpoint}/artifact.tgz");
    let archive = terminal_archive();
    let handle = thread::spawn(move || {
        let (mut first, _) = listener.accept().unwrap();
        let request = read_http(&mut first);
        assert!(request.contains("terminal-outputs") && request.contains("app:build"));
        respond(
            &mut first,
            serde_json::to_string(&json!({"artifactUrl":artifact_url}))
                .unwrap()
                .as_bytes(),
            "application/json",
        );
        let (mut second, _) = listener.accept().unwrap();
        assert!(read_http(&mut second).contains("artifact.tgz"));
        respond(&mut second, &archive, "application/gzip");
    });
    let mut cmd = command(root.path());
    cmd.args([
        "--cloud-url",
        &endpoint,
        "ci-task-output",
        "app:build",
        "--run",
        "run-1",
    ]);
    let output = json_output(cmd);
    assert_eq!(output["output"]["content"], "failed\nlast line\n");
    handle.join().unwrap();
}

#[test]
fn setup_preserves_config_and_is_byte_idempotent() {
    let root = workspace();
    fs::create_dir_all(root.path().join(".claude")).unwrap();
    fs::write(
        root.path().join(".claude/settings.json"),
        r#"{"theme":"dark"}"#,
    )
    .unwrap();
    for _ in 0..2 {
        let mut cmd = command(root.path());
        cmd.args(["setup", "--claude"]);
        assert!(cmd.output().unwrap().status.success());
    }
    let bytes = fs::read(root.path().join(".claude/settings.json")).unwrap();
    let value: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(value["theme"], "dark");
    assert_eq!(value["hooks"]["SessionStart"].as_array().unwrap().len(), 1);
    let command = value["hooks"]["SessionStart"][0]["hooks"][0]["command"]
        .as_str()
        .unwrap();
    assert!(command.contains("--workspace") && command.starts_with('\''));
}
