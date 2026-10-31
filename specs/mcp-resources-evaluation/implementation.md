# Implementation receipt — MCP resources evaluation (#158)

Implements [tasks.md](tasks.md) under proposed
[0106](../../docs/decisions/0106-mcp-resources-evaluation.md). Baseline `d892acb`.

**The wait-tool versus subscription metric remains NOT MEASURED** (paid trials
are operator-owned and forbidden here; no baseline arm exists). This receipt
does not claim "no gain" and does not close #158; closing is the operator's act.

## Tasks

- **T1** — index row for 0106 added after 0105 in `docs/decisions/README.md`
  ("proposed — awaiting the operator"; "provisional number"). Evals `boundary`
  test green.
- **T2** — witness `resources_and_tasks_methods_are_refused_by_name` in
  `tests/mcp_revisions.rs`: for each of the three revisions, `initialize`
  advertises exactly `{"tools":{}}` and `resources/list`, `resources/read`,
  `resources/templates/list`, `resources/subscribe`, `resources/unsubscribe`,
  `tasks/get`, `tasks/list`, `tasks/result`, `tasks/cancel` answer `-32601`
  carrying the request id over HTTP, and the binary stdio bridge returns the
  identical reply. Existing pins (`revision_negotiation_matrix`, `tests/mcp.rs`)
  unchanged and green.
- **T3** — dated risk note appended to `evals/run-plan.md` (60 s client tool
  timeout equals the server's wait cap; observed in configuration, not
  reproduced; for the baseline's `wait_timeout` rows).
- **T4** — no path under `src/`, `evals/src/`, `benches/`, `charts/`, no
  `Cargo.toml`, lockfile or `deny.toml` changed; the only Rust change is the
  test file of T2 (see the diff stat at the commit).

## Gates (locked, on the working tree)

| Command | Exit |
|---|---|
| `cargo fmt --all -- --check` | 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 0 |
| `cargo test --workspace --locked` | 0 |
| evals: `cargo fmt --all -- --check`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked` | 0 (no failures) |
| `bash scripts/coverage-gate.sh` | 0 — exact 100/100/100: lines 19096/19096, branches 1320/1320, functions 1950/1950 |
| `bash scripts/coverage-gate.sh selftest` | 0 |
| `bash scripts/publish-guard-test.sh` | 0 |
| `bash scripts/seal-branch-test.sh` | 0 |
| `cargo deny check licenses` | 0 |
| `brokkr compile --bundle .brokkr/recipes/self` | **not run**: the pinned brokkr binary (`tooling.json`, 0.12.0, sha `29dc3309…`) is not provisioned in the implementer sandbox and there is no network; `brokkr: command not found`. The verify seat and CI own this proof. No bundle file was changed. |
