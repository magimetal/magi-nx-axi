# AGENTS.md

## Project

Rust 2024 agent-facing Nx CLI. Binary `magi-nx-axi`; default stdout TOON, JSON via `--format json`.

## Contracts

- Keep stdout structured for success and expected errors. stderr remains empty unless unexpected diagnostics are explicitly added.
- Exit 0 success/empty/no-op, 1 operational failure, 2 usage failure.
- Never use `npx`, global Nx, shell-interpolated subprocess arguments, MCP transport, or Nx Console TypeScript libraries.
- Workspace analysis uses trusted workspace-local Nx. IDE methods use direct JSON-RPC. Cloud/docs/plugin catalogs use validated direct HTTPS; loopback HTTP exists only for tests.
- Preserve all 13 Nx MCP-equivalent command capabilities plus `cipes` and explicit `setup`.
- Keep README, `docs/plans/nx-axi-contract.md`, parser help, and `skills/magi-nx-axi/SKILL.md` synchronized.

## Layout

- `src/lib.rs`: clap contract, dispatch, AXI rendering/errors/home.
- `src/nx.rs`: workspace graph, plugins, generators, schemas.
- `src/ide.rs`: socket discovery and framed JSON-RPC.
- `src/cloud.rs`: Nx Cloud auth, requests, artifacts, self-healing.
- `src/setup.rs`: atomic project agent integrations.
- `tests/integration.rs`: real-binary hermetic acceptance tests.

## Required gate

```sh
cargo fmt --check
cargo test --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo build --release --locked
```

For protocol changes, add hermetic request/socket/file evidence. Never require developer credentials or live provider state in CI.
