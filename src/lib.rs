mod cloud;
mod ide;
mod nx;
mod setup;

use clap::{Parser, Subcommand, error::ErrorKind};
use serde_json::{Value, json};
use std::{path::PathBuf, process::ExitCode};

#[derive(Parser, Debug)]
#[command(
    name = "magi-nx-axi",
    version,
    about = "Agent-native Nx workspace CLI",
    disable_help_subcommand = true,
    after_help = "Examples:\n  magi-nx-axi workspace --format json\n  magi-nx-axi project app --select targets.build\n  magi-nx-axi graph project --project app"
)]
pub struct Cli {
    #[arg(long, env = "NX_WORKSPACE_PATH", global = true)]
    pub workspace: Option<PathBuf>,
    #[arg(long, env="NX_FORMAT", global=true, default_value="toon", value_parser=["toon","json"])]
    pub format: String,
    #[arg(long, global = true)]
    pub full: bool,
    #[arg(long, env = "NX_AXI_CLOUD_URL", global = true)]
    pub cloud_url: Option<String>,
    #[command(subcommand)]
    pub command: Option<CommandKind>,
}
#[derive(Subcommand, Debug)]
pub enum CommandKind {
    #[command(
        about = "Search current Nx documentation",
        after_help = "Examples:\n  magi-nx-axi docs \"How do affected tasks work?\"\n  magi-nx-axi --format json docs \"configure named inputs\""
    )]
    Docs { query: String },
    #[command(
        about = "List official, community, installed, and local Nx plugins",
        after_help = "Examples:\n  magi-nx-axi plugins\n  magi-nx-axi --format json plugins"
    )]
    Plugins,
    #[command(
        about = "Inspect project graph, dependencies, and nx.json",
        after_help = "Examples:\n  magi-nx-axi workspace\n  magi-nx-axi workspace --filter 'tag:type:app,!*-e2e' --select targets.build"
    )]
    Workspace {
        #[arg(long)]
        filter: Option<String>,
        #[arg(long)]
        select: Option<String>,
        #[arg(long, default_value_t = 0)]
        page: usize,
        #[arg(long)]
        limit: Option<usize>,
    },
    #[command(
        about = "Resolve current Nx workspace root",
        after_help = "Examples:\n  magi-nx-axi workspace-path\n  magi-nx-axi --workspace ../repo workspace-path"
    )]
    WorkspacePath,
    #[command(
        about = "Inspect one Nx project",
        after_help = "Examples:\n  magi-nx-axi project app\n  magi-nx-axi project app --select targets.build"
    )]
    Project {
        name: String,
        #[arg(long)]
        select: Option<String>,
        #[arg(long, default_value_t = 0)]
        page: usize,
    },
    #[command(
        about = "List installed and local generators",
        after_help = "Examples:\n  magi-nx-axi generators\n  magi-nx-axi --format json generators"
    )]
    Generators,
    #[command(
        about = "Read complete generator JSON schema",
        after_help = "Examples:\n  magi-nx-axi generator-schema @nx/react:component\n  magi-nx-axi generator-schema workspace:widget --full"
    )]
    GeneratorSchema { name: String },
    #[command(
        about = "Read generator examples.md",
        after_help = "Examples:\n  magi-nx-axi generator-examples @nx/react:component\n  magi-nx-axi generator-examples workspace:widget --full"
    )]
    GeneratorExamples { name: String },
    #[command(
        about = "Open Nx graph in connected Nx Console IDE",
        after_help = "Examples:\n  magi-nx-axi graph full-project-graph\n  magi-nx-axi graph project-task --project app --task build"
    )]
    Graph {
        #[arg(value_parser=["project","project-task","full-project-graph"])]
        kind: String,
        #[arg(long)]
        project: Option<String>,
        #[arg(long)]
        task: Option<String>,
    },
    #[command(
        about = "List IDE-known running and recent Nx tasks",
        after_help = "Examples:\n  magi-nx-axi tasks\n  magi-nx-axi --format json tasks"
    )]
    Tasks,
    #[command(
        about = "Read newest IDE task output page",
        after_help = "Examples:\n  magi-nx-axi task-output app:build\n  magi-nx-axi task-output build --page 1"
    )]
    TaskOutput {
        task: String,
        #[arg(long, default_value_t = 0)]
        page: usize,
    },
    #[command(
        about = "Inspect Nx Cloud CI pipeline information",
        after_help = "Examples:\n  magi-nx-axi ci --branch main\n  magi-nx-axi ci --url https://cloud.nx.app/runs/<id> --select tasks"
    )]
    Ci {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        branch: Option<String>,
        #[arg(long)]
        select: Option<String>,
        #[arg(long, default_value_t = 0)]
        page: usize,
    },
    #[command(
        about = "Read newest Nx Cloud task output page",
        after_help = "Examples:\n  magi-nx-axi ci-task-output app:build --run <run-id>\n  magi-nx-axi ci-task-output app:test --branch main --page 1"
    )]
    CiTaskOutput {
        task: String,
        #[arg(long)]
        run: Option<String>,
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        branch: Option<String>,
        #[arg(long, default_value_t = 0)]
        page: usize,
    },
    #[command(
        about = "Apply, reject, or rerun a self-healing fix",
        after_help = "Examples:\n  magi-nx-axi self-healing --id <id> APPLY\n  magi-nx-axi self-healing --short-link <fix-suggestion> REJECT"
    )]
    SelfHealing {
        #[arg(long, conflicts_with_all=["short_link","branch"])]
        id: Option<String>,
        #[arg(long, conflicts_with_all=["id","branch"])]
        short_link: Option<String>,
        #[arg(long, conflicts_with_all=["id","short_link"])]
        branch: Option<String>,
        #[arg(value_parser=["APPLY","REJECT","RERUN_ENVIRONMENT_STATE"])]
        action: String,
    },
    #[command(
        about = "List recent Nx Cloud pipeline executions",
        after_help = "Examples:\n  magi-nx-axi cipes --branch main\n  magi-nx-axi --format json cipes"
    )]
    Cipes {
        #[arg(long)]
        branch: Option<String>,
    },
    #[command(
        about = "Install or repair opt-in agent session integrations",
        after_help = "Examples:\n  magi-nx-axi setup\n  magi-nx-axi setup --claude --opencode"
    )]
    Setup {
        #[arg(long)]
        claude: bool,
        #[arg(long)]
        codex: bool,
        #[arg(long)]
        opencode: bool,
    },
}

fn root(cli: &Cli) -> Result<PathBuf, String> {
    nx::find_workspace(
        &cli.workspace
            .clone()
            .unwrap_or(std::env::current_dir().map_err(|e| e.to_string())?),
    )
}
fn reverse_page(s: &str, p: usize, max_chars: usize, max_lines: usize) -> Value {
    let lines: Vec<&str> = s.split_inclusive('\n').collect();
    let mut pages = Vec::<String>::new();
    let mut end = lines.len();
    while end > 0 {
        let mut start = end;
        let mut chars = 0usize;
        while start > 0 && end - start < max_lines {
            let next = lines[start - 1].chars().count();
            if chars > 0 && chars.saturating_add(next) > max_chars {
                break;
            }
            chars = chars.saturating_add(next);
            start -= 1;
        }
        if start == end {
            start = end - 1;
        }
        pages.push(lines[start..end].concat());
        end = start;
    }
    let total = pages.len();
    let content = pages.get(p).cloned().unwrap_or_default();
    json!({"content":content,"page":p,"totalPages":total,"hasMore":p+1<total,"nextPage":(p+1<total).then_some(p+1)})
}
fn select(v: &Value, path: &str) -> Option<Value> {
    path.replace('[', ".")
        .replace(']', "")
        .split('.')
        .filter(|x| !x.is_empty())
        .try_fold(v, |x, k| x.get(k))
        .cloned()
}
fn docs(query: &str, full: bool) -> Result<Value, String> {
    let base = std::env::var("NX_AXI_DOCS_URL").unwrap_or_else(|_| "https://nx.dev".into());
    let url = url::Url::parse(&base).map_err(|e| format!("invalid docs endpoint: {e}"))?;
    let loopback = matches!(
        url.host_str(),
        Some("localhost") | Some("127.0.0.1") | Some("::1")
    );
    if url.scheme() != "https" && !(url.scheme() == "http" && loopback) {
        return Err("docs endpoint must use HTTPS; HTTP is allowed only for loopback tests".into());
    }
    let response = ureq::post(&format!(
        "{}/api/query-ai-embeddings",
        base.trim_end_matches('/')
    ))
    .header("Content-Type", "application/json")
    .send_json(json!({"messages":[{"role":"user","content":query}]}))
    .map_err(|e| format!("Nx docs request failed: {e}"))?;
    let value: Value = response
        .into_body()
        .read_json()
        .map_err(|e| format!("Nx docs returned invalid JSON: {e}"))?;
    let all = value
        .pointer("/context/pageSections")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let total = all.len();
    let sections: Vec<_> = if full {
        all
    } else {
        all.into_iter().take(4).collect()
    };
    let help = (sections.len() < total).then(|| vec!["magi-nx-axi docs \"<same query>\" --full"]);
    Ok(json!({"query":query,"count":sections.len(),"total":total,"sections":sections,"help":help}))
}

fn filter_match(value: &str, pattern: &str) -> bool {
    if pattern.contains('*') {
        let mut parts = pattern.split('*');
        value.starts_with(parts.next().unwrap_or(""))
            && value.ends_with(parts.next_back().unwrap_or(""))
    } else {
        value.contains(pattern)
    }
}
fn project_matches(project: &Value, expression: &str) -> bool {
    let name = project
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let root = project
        .get("root")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let tags = project
        .get("tags")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    expression
        .split(',')
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .all(|pattern| {
            let negative = pattern.starts_with('!');
            let pattern = pattern.strip_prefix('!').unwrap_or(pattern);
            let matched = if let Some(tag) = pattern.strip_prefix("tag:") {
                tags.iter()
                    .filter_map(Value::as_str)
                    .any(|value| filter_match(value, tag))
            } else {
                filter_match(name, pattern) || filter_match(root, pattern)
            };
            if negative { !matched } else { matched }
        })
}

fn current_branch(root: &std::path::Path) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("cannot determine git branch: {e}"))?;
    if !output.status.success() {
        return Err("cannot determine git branch; pass --branch or --url".into());
    }
    let branch = String::from_utf8(output.stdout)
        .map_err(|_| "git branch is not UTF-8")?
        .trim()
        .to_owned();
    if branch.is_empty() {
        Err("cannot determine git branch; pass --branch or --url".into())
    } else {
        Ok(branch)
    }
}

fn cloud_url_parts(raw: &str) -> Result<(&'static str, String, Option<String>), String> {
    let url = url::Url::parse(raw).map_err(|_| "invalid Nx Cloud URL")?;
    let parts: Vec<_> = url.path_segments().map(|x| x.collect()).unwrap_or_default();
    if let Some(id) = parts
        .iter()
        .position(|part| *part == "cipes")
        .and_then(|index| parts.get(index + 1))
    {
        return Ok(("cipe", (*id).into(), None));
    }
    if let Some(id) = parts
        .iter()
        .position(|part| *part == "runs")
        .and_then(|index| parts.get(index + 1))
    {
        let task = parts
            .iter()
            .position(|part| *part == "task")
            .and_then(|i| parts.get(i + 1))
            .map(|x| (*x).into());
        return Ok(("run", (*id).into(), task));
    }
    Err(
        "unsupported Nx Cloud URL; expected /cipes/<id>, /runs/<id>, or /runs/<id>/task/<task>"
            .into(),
    )
}

fn find_string(value: &Value, key: &str) -> Option<String> {
    match value {
        Value::Object(object) => object
            .get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| object.values().find_map(|value| find_string(value, key))),
        Value::Array(items) => items.iter().find_map(|value| find_string(value, key)),
        _ => None,
    }
}

fn list(v: Value, page: usize, limit: Option<usize>) -> Value {
    let a = v.as_array().cloned().unwrap_or_default();
    let total = a.len();
    let n = limit.unwrap_or(1000);
    let start = page.saturating_mul(n);
    let items = a.into_iter().skip(start).take(n).collect::<Vec<_>>();
    json!({"items":items,"count":items.len(),"total":total,"page":page,"limit":n,"hasMore":start.saturating_add(n)<total})
}
fn execute(c: &Cli) -> Result<Value, String> {
    let workspace = match c.command {
        Some(CommandKind::Docs { .. })
        | Some(CommandKind::Plugins)
        | Some(CommandKind::WorkspacePath)
        | None => nx::find_workspace(
            &c.workspace
                .clone()
                .unwrap_or(std::env::current_dir().map_err(|e| e.to_string())?),
        )
        .ok(),
        _ => Some(root(c)?),
    };
    match (&c.command, workspace) {
        (None, None) => Ok(json!({
            "bin": display_executable(),
            "description":"Inspect Nx workspaces, generators, IDE tasks, and Nx Cloud CI",
            "workspace":{"found":false,"cwd":std::env::current_dir().map_err(|e|e.to_string())?},
            "help":["magi-nx-axi --workspace <path> workspace","magi-nx-axi docs \"<nx question>\""]
        })),
        (None, Some(root)) => match nx::projects(&root) {
            Ok(projects) => Ok(json!({
                "bin":display_executable(),
                "description":"Inspect Nx workspaces, generators, IDE tasks, and Nx Cloud CI",
                "workspace":{"found":true,"path":root,"projects":projects.len(),"analysis":"ready"},
                "projects":projects.iter().take(8).filter_map(|p|p["name"].as_str()).collect::<Vec<_>>(),
                "help":["magi-nx-axi workspace","magi-nx-axi project <name>","magi-nx-axi ci"]
            })),
            Err(message) => Ok(json!({
                "bin":display_executable(),
                "description":"Inspect Nx workspaces, generators, IDE tasks, and Nx Cloud CI",
                "workspace":{"found":true,"path":root,"analysis":"unavailable","message":message},
                "help":["Install workspace dependencies, then run `magi-nx-axi workspace`.","magi-nx-axi docs \"install Nx workspace dependencies\""]
            })),
        },
        (Some(CommandKind::Docs { query }), _) => docs(query, c.full),
        (Some(CommandKind::WorkspacePath), root) => Ok(match root {
            Some(root) => json!({"found":true,"workspace":root}),
            None => json!({"found":false,"workspace":Value::Null}),
        }),
        (Some(CommandKind::Plugins), root) => {
            let plugins = nx::plugins(root.as_deref())?;
            Ok(json!({
                "scope":root,
                "count":plugins.len(),
                "plugins":plugins,
                "help":["magi-nx-axi generators"]
            }))
        }
        (
            Some(CommandKind::Workspace {
                filter,
                select: s,
                page,
                limit,
            }),
            Some(root),
        ) => {
            let mut projects = nx::projects(&root)?;
            if let Some(expression) = filter {
                projects.retain(|project| project_matches(project, expression));
            }
            if let Some(path) = s {
                let selected = projects
                    .iter()
                    .map(|project| {
                        let name = project["name"].clone();
                        select(project, path)
                            .map(|value| json!({"project":name,"value":value}))
                            .ok_or_else(|| {
                                format!(
                                    "selection `{path}` not found in project {}",
                                    name.as_str().unwrap_or("<unknown>")
                                )
                            })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let paged = list(json!(selected), *page, *limit);
                Ok(json!({"workspace":root,"selection":path,"projects":paged}))
            } else {
                let dependencies = nx::graph(&root)?
                    .get("dependencies")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let nx_json = nx::nx_json(&root)?;
                Ok(
                    json!({"workspace":root,"projects":list(json!(projects),*page,*limit),"dependencies":dependencies,"nxJson":nx_json,"help":["magi-nx-axi project <name>"]}),
                )
            }
        }
        (
            Some(CommandKind::Project {
                name,
                select: selection,
                page,
            }),
            Some(root),
        ) => {
            let projects = nx::projects(&root)?;
            let project = projects
                .into_iter()
                .find(|value| value.get("name").and_then(Value::as_str) == Some(name))
                .ok_or_else(|| format!("project not found: {name}"))?;
            let value = if let Some(path) = selection {
                select(&project, path)
                    .ok_or_else(|| format!("selection `{path}` not found in project {name}"))?
            } else {
                project
            };
            if *page > 0 {
                let serialized = serde_json::to_string(&value).map_err(|e| e.to_string())?;
                Ok(json!({"project":name,"page":reverse_page(&serialized,*page,10_000,usize::MAX)}))
            } else {
                Ok(json!({"project":value}))
            }
        }
        (Some(CommandKind::Generators), Some(root)) => {
            let generators = nx::generators(&root)?;
            Ok(
                json!({"workspace":root,"count":generators.len(),"generators":generators,"help":["magi-nx-axi generator-schema <name>"]}),
            )
        }
        (Some(CommandKind::GeneratorSchema { name }), Some(root)) => Ok(
            json!({"workspace":root,"generator":name,"schema":nx::generator_schema(&root,name)?}),
        ),
        (Some(CommandKind::GeneratorExamples { name }), Some(root)) => Ok(
            json!({"workspace":root,"generator":name,"examples":nx::generator_examples(&root,name)?}),
        ),
        (
            Some(CommandKind::Graph {
                kind,
                project,
                task,
            }),
            Some(root),
        ) => {
            let projects = nx::projects(&root)?;
            if let Some(name) = project {
                let project = projects
                    .iter()
                    .find(|value| value["name"].as_str() == Some(name.as_str()))
                    .ok_or_else(|| format!("project not found: {name}"))?;
                if let Some(task) = task {
                    if project.pointer(&format!("/targets/{task}")).is_none() {
                        return Err(format!("target not found: {name}:{task}"));
                    }
                }
            }
            ide::notify_graph(&root, kind, project.as_deref(), task.as_deref())?;
            Ok(
                json!({"visualization":{"type":kind,"project":project,"task":task,"ide":"connected","opened":true}}),
            )
        }
        (Some(CommandKind::Tasks), Some(root)) => {
            let response = ide::call(&root, "ide/getRunningTasks", None, true)?;
            let tasks = response
                .pointer("/result/runningTasks")
                .and_then(Value::as_object)
                .ok_or("IDE returned no running-task map")?;
            let rows: Vec<_> = tasks
                .values()
                .map(|task| {
                    json!({
                        "taskId":task.get("name"),
                        "status":task.get("status"),
                        "continuous":task.get("continuous"),
                        "runStatus":task.get("overallRunStatus")
                    })
                })
                .collect();
            Ok(
                json!({"workspace":root,"count":rows.len(),"tasks":rows,"help":["magi-nx-axi task-output <task-id>"]}),
            )
        }
        (Some(CommandKind::TaskOutput { task, page: p }), Some(root)) => {
            let response = ide::call(&root, "ide/getRunningTasks", None, true)?;
            let (id, task) = ide::task_output(&response, task)?;
            let output = task
                .get("output")
                .and_then(Value::as_str)
                .unwrap_or_default();
            Ok(
                json!({"task":id,"status":task.get("status"),"continuous":task.get("continuous"),"output":reverse_page(output,*p,10_000,usize::MAX)}),
            )
        }
        (
            Some(CommandKind::Ci {
                url,
                branch,
                select: selection,
                page,
            }),
            Some(root),
        ) => {
            let value = if let Some(url) = url {
                let (kind, id, _) = cloud_url_parts(url)?;
                if kind == "cipe" {
                    cloud::pipeline(&root, c.cloud_url.as_deref(), &id)?
                } else {
                    cloud::run(&root, c.cloud_url.as_deref(), &id)?
                }
            } else {
                let branch = branch
                    .clone()
                    .map(Ok)
                    .unwrap_or_else(|| current_branch(&root))?;
                cloud::recent(&root, c.cloud_url.as_deref(), Some(&branch))?
            };
            if let Some(path) = selection {
                let selected = select(&value, path)
                    .ok_or_else(|| format!("selection `{path}` not found in CI information"))?;
                if let Some(text) = selected.as_str() {
                    Ok(json!({"selection":path,"value":reverse_page(text,*page,10_000,usize::MAX)}))
                } else {
                    Ok(json!({"selection":path,"value":selected}))
                }
            } else {
                Ok(json!({"ci":value,"help":["magi-nx-axi ci-task-output <task> --run <run-id>"]}))
            }
        }
        (Some(CommandKind::Cipes { branch }), Some(root)) => {
            let branch = branch
                .clone()
                .map(Ok)
                .unwrap_or_else(|| current_branch(&root))?;
            let value = cloud::recent(&root, c.cloud_url.as_deref(), Some(&branch))?;
            let items = value
                .get("ciPipelineExecutions")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            Ok(json!({"scope":"recent","branch":branch,"count":items.len(),"executions":items}))
        }
        (
            Some(CommandKind::CiTaskOutput {
                task,
                run,
                url,
                branch,
                page,
            }),
            Some(root),
        ) => {
            let run = if let Some(run) = run {
                run.clone()
            } else if let Some(url) = url {
                let (_, id, task_from_url) = cloud_url_parts(url)?;
                if let Some(url_task) = task_from_url {
                    if url_task != *task {
                        return Err(format!("task URL identifies `{url_task}`, not `{task}`"));
                    }
                }
                id
            } else {
                let branch = branch
                    .clone()
                    .map(Ok)
                    .unwrap_or_else(|| current_branch(&root))?;
                let recent = cloud::recent(&root, c.cloud_url.as_deref(), Some(&branch))?;
                find_string(&recent, "linkId")
                    .or_else(|| find_string(&recent, "executionId"))
                    .ok_or("no run found for branch")?
            };
            let output = cloud::terminal(&root, c.cloud_url.as_deref(), task, &run)?;
            Ok(json!({"task":task,"run":run,"output":reverse_page(&output,*page,10_000,120)}))
        }
        (
            Some(CommandKind::SelfHealing {
                id,
                short_link,
                branch,
                action,
            }),
            Some(root),
        ) => {
            let id = if let Some(id) = id {
                id.clone()
            } else if let Some(short) = short_link {
                let fix = cloud::retrieve_fix(&root, c.cloud_url.as_deref(), short)?;
                find_string(&fix, "aiFixId").ok_or("Nx Cloud fix response contained no aiFixId")?
            } else {
                let branch = branch
                    .clone()
                    .map(Ok)
                    .unwrap_or_else(|| current_branch(&root))?;
                let recent = cloud::recent(&root, c.cloud_url.as_deref(), Some(&branch))?;
                find_string(&recent, "aiFixId").ok_or("no self-healing fix found for branch")?
            };
            let result = cloud::update(&root, c.cloud_url.as_deref(), &id, action)?;
            if result.get("success").and_then(Value::as_bool) != Some(true) {
                let message = result
                    .pointer("/error/message")
                    .and_then(Value::as_str)
                    .unwrap_or("Nx Cloud did not confirm self-healing mutation");
                return Err(message.into());
            }
            Ok(json!({"success":true,"changed":true,"aiFixId":id,"action":action,"result":result}))
        }
        (
            Some(CommandKind::Setup {
                claude,
                codex,
                opencode,
            }),
            Some(root),
        ) => setup::install(&root, *claude, *codex, *opencode),
        _ => Err("invalid command context".into()),
    }
}
fn display_executable() -> String {
    let path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("magi-nx-axi"));
    let text = path.to_string_lossy().into_owned();
    if let Some(home) = std::env::var_os("HOME") {
        let home = home.to_string_lossy();
        if let Some(rest) = text.strip_prefix(home.as_ref()) {
            return format!("~{rest}");
        }
    }
    text
}

fn truncate(value: &mut Value) -> bool {
    match value {
        Value::String(text) if text.chars().count() > 1000 => {
            let length = text.chars().count();
            *text = text.chars().take(1000).collect::<String>()
                + &format!("… [truncated, {length} chars; use --full]");
            true
        }
        Value::Array(items) => {
            let mut changed = false;
            for item in items {
                changed |= truncate(item);
            }
            changed
        }
        Value::Object(items) => {
            let mut changed = false;
            for item in items.values_mut() {
                changed |= truncate(item);
            }
            changed
        }
        _ => false,
    }
}
fn output(c: &Cli, mut value: Value) -> Result<(), String> {
    if !c.full {
        truncate(&mut value);
    }
    let rendered = if c.format == "json" {
        serde_json::to_string(&value).map_err(|e| e.to_string())?
    } else {
        toon_format::encode(&value, &toon_format::EncodeOptions::default())
            .map_err(|e| e.to_string())?
    };
    println!("{rendered}");
    Ok(())
}
pub fn main_exit() -> ExitCode {
    match Cli::try_parse() {
        Ok(c) => match execute(&c) {
            Ok(value) => match output(&c, value) {
                Ok(()) => ExitCode::SUCCESS,
                Err(message) => {
                    println!(
                        "{}",
                        serde_json::to_string(
                            &json!({"error":{"type":"output","code":1,"message":message}})
                        )
                        .unwrap()
                    );
                    ExitCode::from(1)
                }
            },
            Err(message) => {
                let value = json!({"error":{"type":"operational","code":1,"message":message}});
                let rendered = if c.format == "json" {
                    serde_json::to_string(&value).unwrap()
                } else {
                    toon_format::encode(&value, &toon_format::EncodeOptions::default()).unwrap()
                };
                println!("{rendered}");
                ExitCode::from(1)
            }
        },
        Err(e) => {
            let kind = e.kind();
            if matches!(kind, ErrorKind::DisplayHelp | ErrorKind::DisplayVersion) {
                print!("{e}");
                ExitCode::SUCCESS
            } else {
                let format = if std::env::args().any(|arg| arg == "json") {
                    "json"
                } else {
                    "toon"
                };
                let value = json!({"error":{"type":"usage","code":2,"message":e.to_string(),"help":"Run `magi-nx-axi --help` or `<command> --help`."}});
                if format == "json" {
                    println!("{}", serde_json::to_string(&value).unwrap());
                } else {
                    println!(
                        "{}",
                        toon_format::encode(&value, &toon_format::EncodeOptions::default())
                            .unwrap()
                    );
                }
                ExitCode::from(2)
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    #[test]
    fn cli_valid() {
        Cli::command().debug_assert();
    }
    #[test]
    fn truncates() {
        let mut v = json!({"x":"a".repeat(1001)});
        assert!(truncate(&mut v));
    }
    #[test]
    fn unicode_page() {
        assert_eq!(reverse_page("éé", 0, 10_000, usize::MAX)["content"], "éé");
    }
}
