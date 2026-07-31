use serde_json::{Value, json};
use std::{fs, path::Path};

const MARK: &str = "magi-nx-axi";

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<bool, String> {
    if path.exists()
        && fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))? == bytes
    {
        return Ok(false);
    }
    let parent = path.parent().ok_or("setup path has no parent")?;
    fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    let temporary = parent.join(format!(".magi-nx-axi-{}.tmp", std::process::id()));
    fs::write(&temporary, bytes).map_err(|e| format!("write {}: {e}", temporary.display()))?;
    fs::rename(&temporary, path).map_err(|e| format!("replace {}: {e}", path.display()))?;
    Ok(true)
}

fn merge_hook(path: &Path, hook: Value) -> Result<bool, String> {
    let mut root: Value = if path.exists() {
        let text = fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
        serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?
    } else {
        json!({})
    };
    let object = root
        .as_object_mut()
        .ok_or("agent hook config must be a JSON object")?;
    let hooks = object
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or("hooks must be an object")?;
    let session = hooks
        .entry("SessionStart")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or("SessionStart must be an array")?;
    session.retain(|value| value.get("_magi_nx_axi").and_then(Value::as_str) != Some(MARK));
    session.push(hook);
    let mut bytes = serde_json::to_vec_pretty(&root).map_err(|e| e.to_string())?;
    bytes.push(b'\n');
    write_atomic(path, &bytes)
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub fn install(root: &Path, claude: bool, codex: bool, opencode: bool) -> Result<Value, String> {
    let (claude, codex, opencode) = if !claude && !codex && !opencode {
        (true, true, true)
    } else {
        (claude, codex, opencode)
    };
    let executable = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let executable_text = executable.to_string_lossy();
    let root_text = root.to_string_lossy();
    let command = format!(
        "{} --workspace {}",
        shell_quote(&executable_text),
        shell_quote(&root_text)
    );
    let hook = json!({
        "_magi_nx_axi":MARK,
        "matcher":"",
        "hooks":[{"type":"command","command":command,"statusMessage":"Loading Nx workspace context"}]
    });
    let mut integrations = Vec::new();
    if claude {
        let path = root.join(".claude/settings.json");
        let changed = merge_hook(&path, hook.clone())?;
        integrations.push(
            json!({"target":"claude","path":path,"state":if changed{"updated"}else{"unchanged"}}),
        );
    }
    if codex {
        let path = root.join(".codex/hooks.json");
        let changed = merge_hook(&path, hook)?;
        integrations.push(
            json!({"target":"codex","path":path,"state":if changed{"updated"}else{"unchanged"}}),
        );
    }
    if opencode {
        let path = root.join(".opencode/plugins/magi-nx-axi.js");
        let executable = serde_json::to_string(&executable_text).map_err(|e| e.to_string())?;
        let workspace = serde_json::to_string(&root_text).map_err(|e| e.to_string())?;
        let plugin = format!(
            "export const MagiNxAxi = async () => ({{\n  \"experimental.chat.system.transform\": async (_input, output) => {{\n    const process = Bun.spawn([{}, \"--workspace\", {}], {{ stdout: \"pipe\", stderr: \"ignore\" }});\n    const context = await new Response(process.stdout).text();\n    output.system.push(context);\n  }}\n}});\n",
            executable, workspace
        );
        let changed = write_atomic(&path, plugin.as_bytes())?;
        integrations.push(
            json!({"target":"opencode","path":path,"state":if changed{"updated"}else{"unchanged"}}),
        );
    }
    Ok(json!({
        "workspace":root,
        "count":integrations.len(),
        "integrations":integrations,
        "restartRequired":true
    }))
}

#[cfg(test)]
mod tests {
    use super::shell_quote;

    #[test]
    fn shell_quote_blocks_substitution() {
        assert_eq!(shell_quote("a'$(touch pwn)"), "'a'\\''$(touch pwn)'");
    }
}
