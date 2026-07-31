# magi-nx-axi

Self-contained agent-facing CLI implementing Nx MCP capability surface without MCP transport or Nx Console TypeScript libraries. Rust 2024; supported platforms: macOS and Linux. Windows is unsupported in this release because IDE transport uses Unix sockets.

## Install

```sh
cargo install --path . --locked
magi-nx-axi --version
```

Release tags produce macOS/Linux binaries and SHA-256 checksums.

## AXI contract

- No arguments: compact directory-scoped home state, not help.
- stdout: one structured TOON document by default; `--format json` for strict JSON.
- stderr: empty for expected success, usage, and operational errors.
- exit `0`: success, explicit empty result, safe no-op; `1`: operational failure; `2`: usage failure.
- Lists include count/total/scope. Long strings truncate recursively at 1,000 Unicode scalar values; `--full` restores semantic content.
- Unknown flags fail before workspace subprocess, IDE, or network operations.
- `--help` and `--version` require no workspace, auth, or network.

## Commands

| Nx MCP capability | Command |
|---|---|
| Nx docs context | `docs <query>` |
| available plugins | `plugins` |
| workspace path | `workspace-path` |
| workspace graph/config/filter/select | `workspace [--filter ...] [--select ...]` |
| project details | `project <name> [--select ...]` |
| generator discovery | `generators` |
| generator schema/examples | `generator-schema <name>`, `generator-examples <name>` |
| IDE graph visualization | `graph full-project-graph`, `graph project --project <name>`, `graph project-task --project <name> --task <target>` |
| IDE task state/output | `tasks`, `task-output <id> [--page N]` |
| Nx Cloud CI | `ci [--branch ...|--url ...] [--select ...]` |
| Nx Cloud task output | `ci-task-output <task> [--run ...|--url ...|--branch ...]` |
| self-healing decision | `self-healing [--id ...|--short-link ...|--branch ...] APPLY|REJECT|RERUN_ENVIRONMENT_STATE` |
| recent CIPE resources | `cipes [--branch ...]` |

Focused examples and flags: `magi-nx-axi <command> --help`.

## Workspace behavior

Nearest ancestor containing `nx.json` wins unless `--workspace`/`NX_WORKSPACE_PATH` is set. Graph inspection runs only trusted workspace-local `node_modules/.bin/nx` or `node_modules/nx/bin/nx.js`; no `npx`, global Nx, shell, or implicit download. Nx project plugins execute while Nx creates its graph: inspect only trusted repositories.

`workspace --filter` accepts comma-separated names, `*` globs, roots, `tag:<pattern>`, and `!` exclusions. `--select` accepts dotted paths and array indexes. Generator commands read package/local collection and schema files directly; they never execute a generator.

## Nx Cloud

Cloud endpoint precedence: `--cloud-url` > `NX_AXI_CLOUD_URL`/`NX_CLOUD_API` > `nx.json.nxCloudUrl` > task-runner URL > `https://cloud.nx.app`.

Credential precedence:

- access token: `NX_CLOUD_AUTH_TOKEN` > `NX_CLOUD_ACCESS_TOKEN` > `nxCloudAccessToken` > task-runner `accessToken`;
- cloud ID: `NX_CLOUD_ID` > `nxCloudId` > task-runner `nxCloudId`;
- personal token: `NX_CLOUD_PERSONAL_ACCESS_TOKEN`.

Remote endpoints and artifacts require HTTPS. Loopback HTTP exists only for hermetic tests. Task logs use bounded gzip/tar extraction, only `terminalOutputs/` regular files, ANSI removal, and newest-first 10,000-character/120-line pages. Cloud API paths are reverse-engineered from pinned Nx Console source and may change; provider shape errors become structured operational errors.

## IDE

`graph`, `tasks`, and `task-output` directly use Nx Console JSON-RPC methods over workspace socket. Socket precedence: `NX_SOCKET_DIR` > `NX_DAEMON_SOCKET_DIR` > Nx native workspace hash under temp directory > `.nx/workspace-data/d`. Start Nx Console for same workspace before invoking IDE commands.

## Agent integrations

```sh
magi-nx-axi setup                     # Claude Code + Codex + OpenCode
magi-nx-axi setup --claude --opencode # selected targets only
```

Setup is explicit, project-scoped, preserving, atomic, idempotent, and repairs managed executable paths. It installs SessionStart hooks for Claude Code/Codex and an OpenCode system-context plugin. Installable on-demand skill lives at `skills/magi-nx-axi/SKILL.md`; hook and skill are complementary.

## Security boundaries

- Subprocess arguments never pass through shell. Hook command strings single-quote executable and workspace paths.
- Credentials never appear in normal output or hooks and are redacted from transport errors.
- Endpoint credentials and non-HTTPS remote URLs are rejected before requests.
- Nx output: 32 MiB; IDE message: 16 MiB; Cloud response/artifact: 32 MiB compressed and 64 MiB extracted.
- Self-healing sends exactly one mutation and never retries automatically. Read `ci` first and verify returned `aiFixId` afterward.

## Development

```sh
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo build --release --locked
```

Hermetic integration tests use fake local Nx, loopback docs/cloud servers, gzipped terminal artifacts, Unix sockets, and isolated setup files. No developer credentials or provider network required.
