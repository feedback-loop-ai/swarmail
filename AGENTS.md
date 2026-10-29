# AGENTS.md — Swarmail

Entry point for agent runtimes (Codex, Claude, DSH) working in this repository.

**Read [`docs/house-rules.md`](docs/house-rules.md) first** — it is the realm
charter: the frozen guarantees, the gates, the dependency policy and the
delivery workflow. **Read [`docs/decisions/`](docs/decisions/)** — the
accepted rulings, and the proposals awaiting the operator, that explain why
the code is shaped the way it is; a change
that weakens a frozen guarantee is refused in review no matter how green the
tests are.

## The non-negotiables, in one paragraph

`Store::insert` is synchronous — an SMTP `250` means the mail is queryable
(decision 0001). `await` subscribes before it checks (0002). Pruning is
oldest-only (0003). Extraction happens at ingest, not query time (0005).
Leave every gate in the one inventory,
[`docs/house-rules.md` § Gates](docs/house-rules.md#gates--part-of-every-change), clean before
reporting success — all of it, `--locked`, the coverage selftest and the
publish-guard proof included; that section also names what only CI
proves. This file keeps no copy of the list. Commit unsigned, never push,
and never stop for signing — the operator seals the branch before merge
(0056). Never report
`complete` while required work remains; never widen the dependency allowlist
without a defensible reason.

## Where things live

| Path | Purpose |
|---|---|
| `docs/house-rules.md` | the realm charter (start here) |
| `docs/decisions/` | accepted rulings and proposals awaiting the operator (ADR discipline; only the operator accepts) |
| `docs/releases/` | release notes per version |
| `.brokkr/recipes/self/` | the delivery constitution — compiled by brokkr |
| `.brokkr/agents/` + `.brokkr/adapters/` | seat charters and the pinned model adapter |
| `tests/` | real-protocol integration suites (`burst`, `p2`, `rate`) |
