# Nx AXI contract

Mode: New-Build  
Repository: `magi-axi-nx`  
Runtime: Rust 2024, MSRV 1.85  
Platforms: macOS/Linux; Windows IDE socket N/A in v0.1  
Distribution: Cargo source install and release binaries/checksums  
Capability source: local Nx Console checkout, commit inspected during build  

## Evidence ledger

| Area | Source fact | Decision | Verification |
|---|---|---|---|
| Docs | `shared/llm-context/docs.ts` POSTs `/api/query-ai-embeddings`, reads `context.pageSections`, keeps 4 | Direct HTTPS POST; no workspace required | loopback request/body and 5→4 test |
| Plugins | `plugins.ts` combines official/community catalogs, installed dependencies, local plugins | Fixed HTTPS catalogs + package/graph inspection | dual catalog + installed test |
| Workspace | `nx-workspace.ts` exposes graph nodes/dependencies, nx.json, filters/select | trusted local `nx graph --print`, compact structured lists | fake Nx graph/filter/select tests |
| Project | project detail classifies graph dependencies and supports dot/index select | graph node data plus dependency edges | detail and missing-select tests |
| Generators | workspace provider reads collections/schema/examples and resolves aliases | filesystem collection discovery; no generator execution | collection/alias/schema/examples test |
| IDE graph | methods: `ide/focusProject`, `ide/focusTask`, `ide/showFullProjectGraph` | direct framed JSON-RPC notification | Unix socket method/params test |
| IDE tasks | `ide/getRunningTasks`; exact then partial lookup; newest 10k paging | direct framed request and normalized rows | open-connection Unicode test |
| Cloud CI | recent/pipeline/run endpoints in `shared/nx-cloud` | direct validated HTTP and selectors | loopback request/auth test |
| Cloud logs | endpoint returns artifact URL; gzip/tar `terminalOutputs/`; ANSI strip | bounded extraction and newest 120-line pages | generated archive integration test |
| Self-healing | actions map to `APPLIED`, `REJECTED`, `RERUN_REQUESTED` | resolve ID then one POST, no retry | body/action test |
| CIPE resources | MCP registers recent CIPE resources | `cipes` command with explicit recent scope | empty count/scope test |

## Command and output matrix

| Command | Scope/input | Success | Empty | Next action |
|---|---|---|---|---|
| no args | cwd/optional workspace | bin, description, workspace, ≤8 projects | `found:false` | workspace/docs |
| `docs` | query | count/total, 4 sections | count 0 | none |
| `plugins` | workspace + catalogs | count, name/source/version/description | count 0 | generators |
| `workspace` | filter/select/page/limit | root, projects, dependencies, nx.json | count 0 with scope | project |
| `project` | exact name/select | full detail or selected value | N/A | none |
| `generators` | workspace collections | count + compact rows | count 0 | schema |
| `generator-schema/examples` | exact or unique alias | complete schema/examples | explicit no examples | none |
| `graph` | validated kind/project/target | IDE confirmation | N/A | none |
| `tasks` | IDE workspace | count + 4-field rows | count 0 | task-output |
| `task-output` | exact/unique task + page | status + newest page | empty page | older page |
| `cipes`/`ci` | branch/URL/select | provider data + scope | count 0 | task output |
| `ci-task-output` | task + run/URL/branch | ANSI-free newest page | empty page | older page |
| `self-healing` | one ID/short-link/branch + action | resolved ID, action, result | no fix = operational error | verify CI |
| `setup` | selected/default targets | per-target path/state | repeated = unchanged | restart session |

Default stdout TOON; JSON opt-in. stdout carries success/errors/help arrays; expected stderr empty. Exit 0 success/empty/no-op, 1 operational, 2 usage. Recursive truncation at 1,000 Unicode scalar values; `--full` disables it. Log paging remains bounded even with `--full`.

## Resolution and security

Workspace: CLI > `NX_WORKSPACE_PATH` > nearest ancestor. Cloud endpoint: CLI > `NX_AXI_CLOUD_URL`/`NX_CLOUD_API` > nx.json > task runner > default. Access token: `NX_CLOUD_AUTH_TOKEN` > `NX_CLOUD_ACCESS_TOKEN` > nx.json > task runner. Cloud ID: environment > nx.json > task runner. Empty explicit credentials are errors.

Remote HTTP is rejected before credentials; loopback HTTP is test-only. Endpoint userinfo is rejected. Subprocesses use argument arrays and workspace-local Nx. Provider/child noise is translated. Distinctive secret absence is asserted. Artifact paths/types/sizes are bounded. Setup writes atomically and touches only managed entries.

## Session targets

| Target | Path | Event/mechanism | Merge/repair evidence |
|---|---|---|---|
| Claude Code | `.claude/settings.json` | SessionStart command hook | preservation/idempotence test |
| Codex | `.codex/hooks.json` | SessionStart command hook | shared managed merge |
| OpenCode | `.opencode/plugins/magi-nx-axi.js` | system transform plugin | generated executable invocation |

Session-end capture is N/A: Nx workspace/IDE/Cloud already own current state; duplicating transcript state has no bounded demonstrated benefit.

## Known provider constraints

- Nx Cloud endpoints are private/reverse-engineered and may change.
- IDE commands inherently require a running Nx Console socket for same workspace.
- Catalog/docs/Cloud reads inherently require network.
- Windows IDE named-pipe transport is not implemented; workspace/docs/cloud surfaces remain architecture-compatible but Windows is not a supported release target.
