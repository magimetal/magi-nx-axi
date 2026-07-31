use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub fn find_workspace(start: &Path) -> Result<PathBuf, String> {
    let mut path = start
        .canonicalize()
        .map_err(|e| format!("cannot resolve {}: {e}", start.display()))?;
    loop {
        if path.join("nx.json").is_file() {
            return Ok(path);
        }
        if !path.pop() {
            return Err(format!(
                "no Nx workspace found at or above {}",
                start.display()
            ));
        }
    }
}

fn executable(root: &Path) -> Result<(PathBuf, Vec<String>), String> {
    let bin = root.join("node_modules/.bin/nx");
    if bin.is_file() {
        return Ok((bin, Vec::new()));
    }
    let script = root.join("node_modules/nx/bin/nx.js");
    if script.is_file() {
        return Ok((
            PathBuf::from("node"),
            vec![script.to_string_lossy().into_owned()],
        ));
    }
    Err("local Nx executable not found; install workspace dependencies (this AXI never downloads Nx)".into())
}

pub fn run(root: &Path, args: &[String]) -> Result<Value, String> {
    let (executable, prefix) = executable(root)?;
    let output = Command::new(executable)
        .args(prefix)
        .args(args)
        .current_dir(root)
        .env("NX_INTERACTIVE", "false")
        .output()
        .map_err(|e| format!("cannot start local Nx: {e}"))?;
    if !output.status.success() {
        return Err("local Nx could not create workspace graph; run `nx graph --print` in workspace for diagnostics".into());
    }
    if output.stdout.len() > 32 * 1024 * 1024 {
        return Err("Nx graph exceeded 32 MiB safety limit".into());
    }
    serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("local Nx returned invalid graph JSON: {e}"))
}

pub fn graph(root: &Path) -> Result<Value, String> {
    let raw = run(root, &["graph".into(), "--print".into()])?;
    Ok(raw.get("graph").cloned().unwrap_or(raw))
}

pub fn projects(root: &Path) -> Result<Vec<Value>, String> {
    let graph = graph(root)?;
    let nodes = graph
        .get("nodes")
        .and_then(Value::as_object)
        .ok_or("Nx graph contains no project nodes")?;
    let dependencies = graph.get("dependencies").and_then(Value::as_object);
    let mut projects = Vec::with_capacity(nodes.len());
    for (name, node) in nodes {
        let mut project = node.get("data").cloned().unwrap_or_else(|| node.clone());
        let object = project
            .as_object_mut()
            .ok_or("Nx project node is not an object")?;
        object.insert("name".into(), json!(name));
        object.insert(
            "dependencies".into(),
            dependencies
                .and_then(|all| all.get(name))
                .cloned()
                .unwrap_or_else(|| json!([])),
        );
        projects.push(project);
    }
    projects.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    Ok(projects)
}

fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))
}

pub fn nx_json(root: &Path) -> Result<Value, String> {
    read_json(&root.join("nx.json"))
}

fn remote_json(variable: &str, default: &str) -> Result<Value, String> {
    let raw = std::env::var(variable).unwrap_or_else(|_| default.into());
    let url = url::Url::parse(&raw).map_err(|e| format!("invalid plugin catalog URL: {e}"))?;
    let loopback = matches!(
        url.host_str(),
        Some("localhost") | Some("127.0.0.1") | Some("::1")
    );
    if url.scheme() != "https" && !(url.scheme() == "http" && loopback) {
        return Err(
            "plugin catalog URL must use HTTPS; HTTP is allowed only for loopback tests".into(),
        );
    }
    let response = ureq::get(&raw)
        .call()
        .map_err(|e| format!("fetch Nx plugin catalog: {e}"))?;
    response
        .into_body()
        .read_json()
        .map_err(|e| format!("Nx plugin catalog returned invalid JSON: {e}"))
}

pub fn plugins(root: Option<&Path>) -> Result<Vec<Value>, String> {
    let mut plugins = BTreeMap::<String, Value>::new();
    if let Some(root) = root {
        let package = read_json(&root.join("package.json"))?;
        for section in ["dependencies", "devDependencies"] {
            if let Some(items) = package.get(section).and_then(Value::as_object) {
                for (name, version) in items {
                    let metadata = root.join("node_modules").join(name).join("package.json");
                    let package_meta = read_json(&metadata).unwrap_or_else(|_| json!({}));
                    if package_meta.get("generators").is_some()
                        || package_meta.get("schematics").is_some()
                        || name == "nx"
                        || name.starts_with("@nx/")
                        || name.starts_with("@nrwl/")
                    {
                        plugins.insert(name.clone(), json!({
                            "name": name,
                            "source": "installed",
                            "version": version,
                            "description": package_meta.get("description").cloned().unwrap_or(Value::Null)
                        }));
                    }
                }
            }
        }
        for project in projects(root)? {
            let name = project["name"].as_str().unwrap_or_default();
            if project["projectType"] == "plugin" || name.contains("plugin") {
                plugins.insert(
                    name.into(),
                    json!({
                        "name": name,
                        "source": "local",
                        "version": Value::Null,
                        "description": project.get("description").cloned().unwrap_or(Value::Null)
                    }),
                );
            }
        }
    }
    let official = remote_json(
        "NX_AXI_OFFICIAL_PLUGINS_URL",
        "https://raw.githubusercontent.com/nrwl/nx/master/docs/packages.json",
    )?;
    if let Some(items) = official.as_array() {
        for item in items {
            let Some(short_name) = item.get("name").and_then(Value::as_str) else {
                continue;
            };
            if [
                "add-nx-to-monorepo",
                "cra-to-nx",
                "create-nx-plugin",
                "create-nx-workspace",
                "make-angular-cli-faster",
                "tao",
            ]
            .contains(&short_name)
            {
                continue;
            }
            let name = format!("@nx/{short_name}");
            plugins.entry(name.clone()).or_insert_with(|| {
                json!({
                    "name":name,
                    "source":"official",
                    "version":Value::Null,
                    "description":item.get("description").cloned().unwrap_or(Value::Null)
                })
            });
        }
    }
    if let Ok(community) = remote_json(
        "NX_AXI_COMMUNITY_PLUGINS_URL",
        "https://raw.githubusercontent.com/nrwl/nx/master/astro-docs/src/content/approved-community-plugins.json",
    ) && let Some(items) = community.as_array()
    {
        for item in items {
            let Some(name) = item.get("name").and_then(Value::as_str) else {
                continue;
            };
            plugins.entry(name.into()).or_insert_with(|| {
                json!({
                    "name":name,
                    "source":"community",
                    "version":Value::Null,
                    "description":item.get("description").cloned().unwrap_or(Value::Null),
                    "url":item.get("url").cloned().unwrap_or(Value::Null)
                })
            });
        }
    }
    Ok(plugins.into_values().collect())
}

#[derive(Clone)]
struct Generator {
    display: String,
    collection: String,
    name: String,
    description: Value,
    aliases: Vec<String>,
    schema: PathBuf,
}

fn add_collection(
    out: &mut Vec<Generator>,
    collection_name: &str,
    collection_path: &Path,
) -> Result<(), String> {
    if !collection_path.is_file() {
        return Ok(());
    }
    let value = read_json(collection_path)?;
    let entries = value
        .get("generators")
        .or_else(|| value.get("schematics"))
        .and_then(Value::as_object)
        .ok_or_else(|| {
            format!(
                "{} has no generators or schematics",
                collection_path.display()
            )
        })?;
    let parent = collection_path.parent().ok_or("collection has no parent")?;
    for (name, entry) in entries {
        let Some(entry) = entry.as_object() else {
            continue;
        };
        if entry.get("hidden").and_then(Value::as_bool) == Some(true) {
            continue;
        }
        let Some(schema) = entry.get("schema").and_then(Value::as_str) else {
            continue;
        };
        let schema = parent
            .join(schema)
            .canonicalize()
            .map_err(|e| format!("resolve schema for {collection_name}:{name}: {e}"))?;
        if !schema.starts_with(parent.canonicalize().map_err(|e| e.to_string())?) {
            return Err(format!(
                "generator schema escapes collection: {collection_name}:{name}"
            ));
        }
        let aliases = entry
            .get("aliases")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        out.push(Generator {
            display: format!("{collection_name}:{name}"),
            collection: collection_name.into(),
            name: name.into(),
            description: entry.get("description").cloned().unwrap_or(Value::Null),
            aliases,
            schema,
        });
    }
    Ok(())
}

fn discover_generators(root: &Path) -> Result<Vec<Generator>, String> {
    let package = read_json(&root.join("package.json"))?;
    let mut out = Vec::new();
    for section in ["dependencies", "devDependencies"] {
        if let Some(items) = package.get(section).and_then(Value::as_object) {
            for name in items.keys() {
                let package_root = root.join("node_modules").join(name);
                let metadata_path = package_root.join("package.json");
                let Ok(metadata) = read_json(&metadata_path) else {
                    continue;
                };
                if let Some(relative) = metadata
                    .get("generators")
                    .or_else(|| metadata.get("schematics"))
                    .and_then(Value::as_str)
                {
                    add_collection(&mut out, name, &package_root.join(relative))?;
                }
            }
        }
    }
    for (name, path) in [
        ("workspace", root.join("tools/generators/collection.json")),
        ("workspace", root.join("tools/schematics/collection.json")),
    ] {
        add_collection(&mut out, name, &path)?;
    }
    for project in projects(root)? {
        let Some(project_root) = project.get("root").and_then(Value::as_str) else {
            continue;
        };
        let package_root = root.join(project_root);
        let Ok(metadata) = read_json(&package_root.join("package.json")) else {
            continue;
        };
        let collection_name = metadata
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or(project_root);
        if let Some(relative) = metadata
            .get("generators")
            .or_else(|| metadata.get("schematics"))
            .and_then(Value::as_str)
        {
            add_collection(&mut out, collection_name, &package_root.join(relative))?;
        }
    }
    out.sort_by(|a, b| a.display.cmp(&b.display));
    Ok(out)
}

pub fn generators(root: &Path) -> Result<Vec<Value>, String> {
    Ok(discover_generators(root)?
        .into_iter()
        .map(|g| {
            json!({
                "name": g.display,
                "collection": g.collection,
                "generator": g.name,
                "description": g.description,
                "aliases": g.aliases
            })
        })
        .collect())
}

fn resolve_generator(root: &Path, requested: &str) -> Result<Generator, String> {
    let matches: Vec<_> = discover_generators(root)?
        .into_iter()
        .filter(|g| {
            g.display == requested
                || g.name == requested
                || g.aliases.iter().any(|alias| alias == requested)
                || g.aliases
                    .iter()
                    .any(|alias| format!("{}:{alias}", g.collection) == requested)
        })
        .collect();
    match matches.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(format!(
            "generator not found: {requested}; run `magi-nx-axi generators`"
        )),
        many => Err(format!(
            "generator name is ambiguous: {requested}; matches {}",
            many.iter()
                .map(|g| g.display.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

pub fn generator_schema(root: &Path, name: &str) -> Result<Value, String> {
    let generator = resolve_generator(root, name)?;
    let mut schema = read_json(&generator.schema)?;
    if let Some(object) = schema.as_object_mut() {
        object.remove("$schema");
        object.remove("$id");
        object.insert("name".into(), json!(generator.display));
    }
    Ok(schema)
}

pub fn generator_examples(root: &Path, name: &str) -> Result<String, String> {
    let generator = resolve_generator(root, name)?;
    let path = generator
        .schema
        .parent()
        .ok_or("generator schema has no parent")?
        .join("examples.md");
    if !path.is_file() {
        return Ok("No examples available".into());
    }
    fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))
}
