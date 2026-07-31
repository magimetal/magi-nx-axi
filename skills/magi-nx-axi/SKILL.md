---
name: magi-nx-axi
description: Use when inspecting an Nx workspace, resolving project targets/dependencies, finding plugins or generators, reading current Nx docs, diagnosing local/CI tasks, visualizing graphs in Nx Console, or reviewing and deciding Nx Cloud self-healing fixes.
---

# NX AXI

Use `magi-nx-axi`. Default stdout is TOON; use `--format json` when strict JSON simplifies processing. Branch on exit: `0` success/empty/no-op, `1` workspace/IDE/network/provider failure, `2` invalid invocation. Expected errors are structured stdout; never scrape stderr.

## Orient

```sh
magi-nx-axi --format json
magi-nx-axi workspace --format json
magi-nx-axi workspace --filter 'tag:type:app,!*-e2e' --select targets.build --format json
magi-nx-axi project <name> --format json
```

Use exact project names returned by `workspace`. Read project target configuration before recommending or invoking Nx commands. `--select` supports dotted paths and array indexes. Use `--full` only when output says content was truncated.

## Current Nx knowledge and generators

```sh
magi-nx-axi docs "<question about current Nx behavior>" --format json
magi-nx-axi plugins --format json
magi-nx-axi generators --format json
magi-nx-axi generator-schema <collection:generator> --format json
magi-nx-axi generator-examples <collection:generator> --format json
```

Always use `docs` for Nx configuration/option questions instead of relying on remembered versions. Resolve generator identity and schema before any separate `nx generate` mutation. AXI generator commands inspect only; they never execute generators.

## IDE tasks and graph

Requires Nx Console running for same workspace.

```sh
magi-nx-axi tasks --format json
magi-nx-axi task-output <task-id> --format json
magi-nx-axi task-output <task-id> --page 1 --format json
magi-nx-axi graph project --project <name> --format json
magi-nx-axi graph project-task --project <name> --task <target> --format json
magi-nx-axi graph full-project-graph --format json
```

Task output is newest-first. Follow `nextPage` for older output. Partial task references must resolve uniquely.

## Nx Cloud CI and self-healing

```sh
magi-nx-axi cipes --branch <branch> --format json
magi-nx-axi ci --branch <branch> --format json
magi-nx-axi ci --url <cipe-or-run-url> --format json
magi-nx-axi ci-task-output <task-id> --run <run-id> --format json
```

Before mutation, run `ci`, inspect fix/diff metadata, and capture explicit `aiFixId` or `shortLink`. Then perform one decision:

```sh
magi-nx-axi self-healing --id <ai-fix-id> APPLY --format json
magi-nx-axi self-healing --short-link <fix-suggestion> REJECT --format json
magi-nx-axi self-healing --id <ai-fix-id> RERUN_ENVIRONMENT_STATE --format json
```

Inspect success and resolved `aiFixId`; do not blindly retry mutation. Cloud commands accept current branch when selector omitted, but explicit branch/run/URL is safer for cross-branch work.

## Setup and safety

`magi-nx-axi setup` explicitly installs project-scoped session integrations; ordinary commands never modify hooks. Workspace graph creation executes workspace Nx plugins, so inspect only trusted repositories. AXI never downloads Nx, runs generators, or shells subprocess arguments.
