# AGENTS.md — Swarmail

Entry point for agent runtimes (Codex, Claude, DSH) working in this repository.

**Read [`docs/house-rules.md`](docs/house-rules.md) first** — it is the realm
charter: the frozen guarantees, the gates, the dependency policy and the
delivery workflow. **Read [`docs/decisions/`](docs/decisions/)** — the
accepted rulings that explain why the code is shaped the way it is; a change
that weakens a frozen guarantee is refused in review no matter how green the
tests are.

## The non-negotiables, in one paragraph

`Store::insert` is synchronous — an SMTP `250` means the mail is queryable
(decision 0001). `await` subscribes before it checks (0002). Pruning is
oldest-only (0003). Extraction happens at ingest, not query time (0005).
Leave `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test`, `brokkr compile --bundle .`, `scripts/coverage-gate.sh` and
`cargo deny check licenses` clean before reporting success. Never report
`complete` while required work remains; never widen the dependency allowlist
without a defensible reason.

## Where things live

| Path | Purpose |
|---|---|
| `docs/house-rules.md` | the realm charter (start here) |
| `docs/decisions/` | accepted rulings (ADR discipline; proposed status needs the operator) |
| `docs/releases/` | release notes per version |
| `bundle.json` + `policy.json` | the delivery constitution — compiled by brokkr |
| `agents/` + `adapters/` | seat charters and the pinned model adapter |
| `tests/` | real-protocol integration suites (`burst`, `p2`, `rate`) |
