# magi-nx-axi

Agent-native Rust CLI for inspecting Nx workspaces, resolving projects and generators, querying current Nx documentation, controlling Nx Console graph views, and diagnosing Nx Cloud CI. It provides Nx MCP-equivalent capabilities through direct, non-interactive commands without MCP transport or Nx Console TypeScript libraries.

[![Crates.io](https://img.shields.io/crates/v/magi-nx-axi.svg)](https://crates.io/crates/magi-nx-axi)
[![docs.rs](https://docs.rs/magi-nx-axi/badge.svg)](https://docs.rs/magi-nx-axi)
[![License: ISC](https://img.shields.io/badge/license-ISC-blue.svg)](LICENSE)
[![Rust 1.87+](https://img.shields.io/badge/rust-1.87%2B-orange.svg)](Cargo.toml)

> **Quick path:** install with Cargo, run `magi-nx-axi` inside an Nx workspace, then use `workspace`, `project`, and focused command help. Default output is compact [TOON](https://toonformat.dev/); use `--format json` when a downstream tool requires JSON.

## Contents

- [Quick start](#quick-start)
- [AXI contract](#axi-contract)
- [Workspace discovery and Nx execution](#workspace-discovery-and-nx-execution)
- [Command guide](#command-guide)
- [Nx Cloud configuration](#nx-cloud-configuration)
- [Agent integrations](#agent-integrations)
- [Reliability and security boundaries](#reliability-and-security-boundaries)
- [Platforms and release artifacts](#platforms-and-release-artifacts)
- [Development](#development)

## Quick start

Requirements:

- Rust 1.87 or newer with Cargo;
- macOS or Linux;
- an Nx workspace for workspace, project, generator, IDE, and Cloud commands;
- installed workspace dependencies when a command needs the project graph;
- Nx Console running for IDE graph and task commands.

Install latest published release:

```sh
cargo install magi-nx-axi --locked
magi-nx-axi --version
```

Install from this checkout while developing:

```sh
cargo install --path . --locked
```

Orient inside a workspace:

```sh
magi-nx-axi
magi-nx-axi workspace-path
magi-nx-axi workspace --format json
magi-nx-axi project <project-name> --format json
```

Inspect current Nx guidance and available generators:

```sh
magi-nx-axi docs "How do affected tasks work?"
magi-nx-axi plugins
magi-nx-axi generators
magi-nx-axi generator-schema @nx/react:component --full
```

Install compact session-start context for supported coding agents:

```sh
magi-nx-axi setup
```

Run focused help before unfamiliar operations:

```sh
magi-nx-axi --help
magi-nx-axi workspace --help
magi-nx-axi self-healing --help
```

## AXI contract

`magi-nx-axi` follows an agent-facing command contract:

- No arguments returns compact, directory-scoped home state instead of help.
- Commands never prompt. Inputs come from arguments, flags, environment, workspace files, or current directory.
- Success data and expected errors use stdout. stderr stays empty for expected success, usage, and operational failures.
- Default structured format is TOON. `--format json` emits strict JSON.
- Lists include count, total, and scope where applicable. Empty results are explicit and exit successfully.
- Strings longer than 1,000 Unicode scalar values are recursively truncated. `--full` restores semantic content; bounded task-log paging remains in effect.
- Exit `0`: success, explicit empty result, or safe no-op. Exit `1`: workspace, IDE, network, provider, config, or output failure. Exit `2`: command or argument usage failure.
- Unknown commands and flags fail before workspace subprocesses, IDE connections, or network requests.
- `--help` and `--version` require no workspace, credentials, subprocess, IDE, or network.

```text
magi-nx-axi [--workspace <path>] [--cloud-url <url>]
            [--format toon|json] [--full]
            <command> [args]
```

Global environment variables:

| Variable | Purpose |
| --- | --- |
| `NX_WORKSPACE_PATH` | Explicit workspace start path |
| `NX_FORMAT` | Default output format: `toon` or `json` |
| `NX_AXI_CLOUD_URL` | Nx Cloud endpoint override |

## Workspace discovery and Nx execution

Workspace precedence is `--workspace`, then `NX_WORKSPACE_PATH`, then current directory. From that starting point, nearest ancestor containing `nx.json` wins.

Graph inspection runs only trusted workspace-local Nx:

1. `node_modules/.bin/nx`
2. `node node_modules/nx/bin/nx.js`

The CLI never uses `npx`, a global Nx installation, shell-interpolated subprocess arguments, or implicit downloads. Nx executes project plugins while creating its graph, so use graph-backed commands only in trusted repositories. Generator commands read collection, schema, and examples files; they never execute generators.

`workspace --filter` accepts comma-separated project names, `*` globs, roots, `tag:<pattern>`, and `!` exclusions. `--select` accepts dotted object paths and array indexes. Read exact project names from `workspace` before calling `project`.

```sh
magi-nx-axi workspace --filter 'tag:type:app,!*-e2e'
magi-nx-axi workspace --select projects.0.targets.build
magi-nx-axi project web --select targets.build
```

## Command guide

### Documentation and plugins

| Job | Command |
| --- | --- |
| Query current Nx documentation | `docs <query>` |
| Combine official, community, installed, and local plugins | `plugins` |

`docs` sends the query directly to Nx documentation search and returns at most four sections by default. `plugins` combines current remote catalogs with workspace package and graph information. Both commands require network access; installed/local plugin detection also requires a workspace.

```sh
magi-nx-axi docs "configure named inputs" --format json
magi-nx-axi plugins --format json
```

### Workspace and projects

| Job | Command |
| --- | --- |
| Resolve workspace root | `workspace-path` |
| Read graph, projects, dependencies, and `nx.json` | `workspace` |
| Read one project and its graph dependencies | `project <name>` |

```sh
magi-nx-axi workspace --limit 25 --page 0
magi-nx-axi workspace --filter 'app*,!*-e2e' --select targets.test
magi-nx-axi project api --select dependencies
```

### Generators

| Job | Command |
| --- | --- |
| Discover installed and local generators | `generators` |
| Read complete generator JSON schema | `generator-schema <collection:generator>` |
| Read generator `examples.md` | `generator-examples <collection:generator>` |

Resolve generator identity before reading schema or examples. Unique aliases are accepted when unambiguous.

```sh
magi-nx-axi generators --format json
magi-nx-axi generator-schema @nx/react:component --full
magi-nx-axi generator-examples workspace:widget --full
```

### Nx Console IDE

`graph`, `tasks`, and `task-output` use direct framed JSON-RPC over the Nx Console Unix socket. Start Nx Console for the same workspace first.

```sh
magi-nx-axi graph full-project-graph
magi-nx-axi graph project --project web
magi-nx-axi graph project-task --project web --task build
magi-nx-axi tasks --format json
magi-nx-axi task-output web:build --page 0
```

Task references may be exact IDs or unique partial matches. Output pages are newest-first; follow `nextPage` for older content. Socket precedence is `NX_SOCKET_DIR`, `NX_DAEMON_SOCKET_DIR`, Nx native workspace hash under temporary directory, then `.nx/workspace-data/d/nx-console.sock`.

### Nx Cloud CI and self-healing

| Job | Command |
| --- | --- |
| List recent pipeline executions | `cipes [--branch <branch>]` |
| Inspect pipeline/run data | `ci [--branch <branch> | --url <url>]` |
| Read task output artifact | `ci-task-output <task> [--run <id> | --url <url> | --branch <branch>]` |
| Decide one self-healing fix | `self-healing [--id <id> | --short-link <link> | --branch <branch>] <action>` |

```sh
magi-nx-axi cipes --branch main --format json
magi-nx-axi ci --branch main --format json
magi-nx-axi ci --url https://cloud.nx.app/runs/<run-id> --select tasks
magi-nx-axi ci-task-output web:test --run <run-id> --page 0
```

Self-healing actions are `APPLY`, `REJECT`, and `RERUN_ENVIRONMENT_STATE`. Read `ci`, inspect fix metadata, and retain explicit `aiFixId` or short link before mutation:

```sh
magi-nx-axi self-healing --id <ai-fix-id> APPLY --format json
magi-nx-axi self-healing --short-link <fix-suggestion> REJECT --format json
```

The command sends exactly one mutation and never retries automatically. Inspect returned action and resolved `aiFixId`; do not blindly retry after success or ambiguous transport failure.

## Nx Cloud configuration

Cloud endpoint precedence, highest first:

1. `--cloud-url`
2. `NX_AXI_CLOUD_URL`
3. `NX_CLOUD_API`
4. `nx.json` `nxCloudUrl`
5. task-runner `options.url`
6. `https://cloud.nx.app`

Credential precedence:

- access token: `NX_CLOUD_AUTH_TOKEN`, then `NX_CLOUD_ACCESS_TOKEN`, then `nxCloudAccessToken`, then task-runner `accessToken`;
- cloud ID: `NX_CLOUD_ID`, then `nxCloudId`, then task-runner `nxCloudId`;
- personal token: `NX_CLOUD_PERSONAL_ACCESS_TOKEN`.

Empty explicit credentials fail. Remote endpoints and artifact URLs require HTTPS and must not contain URL credentials. Loopback HTTP is accepted only to support hermetic tests. Nx Cloud endpoints are reverse-engineered from pinned Nx Console behavior and may change; provider shape errors become structured operational errors.

## Agent integrations

`setup` installs all targets when no target flag is supplied:

```sh
magi-nx-axi setup
magi-nx-axi setup --claude --codex
magi-nx-axi setup --opencode
```

Managed project files:

| Target | File and mechanism |
| --- | --- |
| Claude Code | `.claude/settings.json`, `hooks.SessionStart` |
| Codex | `.codex/hooks.json`, `hooks.SessionStart` |
| OpenCode | `.opencode/plugins/magi-nx-axi.js`, system-context transform |

Setup is explicit, project-scoped, preserving, atomic per file, idempotent, and repairs managed executable paths. It records resolved executable and workspace paths, preserves unrelated valid configuration, and does not store Cloud credentials. Restart target agent after setup.

Installable on-demand Agent Skill lives at [`skills/magi-nx-axi/SKILL.md`](skills/magi-nx-axi/SKILL.md). Skill guidance and ambient hooks are complementary: skill provides operating procedure when invoked; setup provides compact session-start discovery.

## Reliability and security boundaries

- Subprocesses use fixed executables and argument arrays; no shell handles Nx arguments.
- Setup shell command strings single-quote executable and workspace paths.
- Credentials never appear in normal output or hooks and are redacted from transport errors.
- Remote endpoint validation occurs before credentials are sent.
- Nx graph output is capped at 32 MiB. IDE messages are capped at 16 MiB, with 8 KiB headers and five-second read timeout.
- Nx Cloud responses and compressed artifacts are capped at 32 MiB; extracted terminal output is capped at 64 MiB.
- Artifact extraction accepts only regular files under `terminalOutputs/`, strips ANSI sequences, and renders newest-first pages bounded to 10,000 characters and 120 lines.
- Cloud and docs/catalog operations require provider network access. Tests use loopback services and fake credentials.
- Windows remains unsupported because IDE transport uses Unix sockets.

## Platforms and release artifacts

Official tagged releases publish the crate to crates.io and attach compressed binaries plus SHA-256 checksum files to GitHub Releases for:

- `x86_64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`

Other macOS/Linux architectures can install from source with Cargo when their Rust dependency graph supports the target. Windows is not supported in this release.

## Development

CI and release gates use Rust 1.87.0 with committed lockfile:

```sh
cargo +1.87.0 fmt --check
cargo +1.87.0 check --locked
cargo +1.87.0 test --locked
cargo +1.87.0 clippy --all-targets --all-features --locked -- -D warnings
cargo +1.87.0 build --release --locked
cargo deny check
cargo +1.87.0 package --locked
cargo +1.87.0 publish --locked --dry-run
```

Hermetic integration tests use fake local Nx, loopback docs/Cloud servers, generated terminal archives, Unix sockets, and isolated setup files. No developer credentials or live provider state are required.

Maintainer and project references:

- [Release checklist](docs/release.md)
- [Nx AXI contract](docs/plans/nx-axi-contract.md)
- [Agent Skill](skills/magi-nx-axi/SKILL.md)
- [Changelog](CHANGELOG.md)
- [Third-party notices](THIRD_PARTY_NOTICES.md)
- [ISC license](LICENSE)
