# Issue #161 — Ship agent setup guides and skills, exercised in CI

Framing by the intake seat, run `issue-161-ship-agent-setup-guide-52194034`,
from main `a3cff51`. Part of epic #164 (agent surface measured by the eval
harness). Priority P2.

## Goal

Ship copy-paste setup for Claude Code (`.mcp.json` plus a skill), Codex,
Cursor and dsh, and an AGENTS.md block that teaches the clear → act → wait →
extract loop and its traps, with every shipped snippet executed by a suite
that runs in CI.

## Reading taken

- The "AGENTS.md block" is copy-paste text for a consumer project's
  AGENTS.md, delivered in the guide. The repository's own `AGENTS.md` is not
  edited.
- The skill is a copy-paste `SKILL.md` under `docs/agents/`. Nothing is
  installed into the repository's own `.claude/`.

## Frozen guarantees near the work

- 0001 (accept == stored) and 0002 (subscribe before check): the loop's wait
  step relies on both. The guides must state them and must not teach a
  check-then-subscribe order.
- 0005 (extraction at ingest): the extract step documents
  `swarmail_extract_codes` and `swarmail_extract_links` as ingest-time
  results.
- 0003 is not touched.

No decision document is needed: the work is docs plus root `tests/`, with no
change to production behaviour or to the gate inventory. Decisions 0055,
0071 and 0076 govern the eval harness and are read-only context here.

## Acceptance criteria (checkable)

- **AC-1 oracle half.** Every Straight-tier task in `evals/src/tasks.rs`
  passes with `--agent oracle` through `swarmail-evals trial`, exit 0,
  outcome `pass`. Red control: the `null` agent fails the same tasks. This is
  seat-run evidence; evals CI wiring is deferred to #201 (0055). The oracle
  does not read the user's configuration, so AC-1 does not by itself prove
  the docs wiring. AC-2 does.
- **AC-2 wired from the docs.** A root test writes each shipped JSON/TOML
  snippet verbatim into a fresh temporary project and starts the real
  `swarmail` binary on the transport the snippet declares (stdio `swarmail
  mcp`, and HTTP on loopback). It then performs `initialize`, `tools/list`,
  and a seed → `swarmail_wait_for_email` → `swarmail_extract_codes`
  round-trip. Red control: change a port or argument in a snippet and the
  test goes red.
- **AC-3 one real model (open, not measurable here).** One real model passes
  the Straight tier with the docs wiring. This needs an authenticated, paid
  trial, which is operator-owned (`evals/README.md`) and out of scope under
  the no-paid-trials rule. It stays open and is reported as not measured. It
  must not be recorded as passed.
- **AC-4 snippet coverage.** Every fenced block in the guide files is either
  executed by the suite or carries an explicit
  `<!-- not-executed: <reason> -->` marker. An unmarked, unexecuted block goes
  red.
- **AC-5 tool-name drift.** Every `swarmail_*` name that appears in the
  guides or the skill exists in the live `tools/list`. Red control: a
  misspelt name goes red.
- **AC-6 AGENTS.md block.** The block names the four loop steps in order
  (clear, act, wait, extract) and each agent-caused failure class of the
  closed enum in 0071: `wrong_message`, `stale_code`, `wrong_link`,
  `wait_timeout`, `gave_up`, `tool_misuse`. The test asserts each name is
  present and is a member of the enum.
- **AC-7 guarantee text.** The guides state the 0001 and 0002 ordering rules
  the loop depends on. A documentary test pins those sentences, in the style
  of `tests/listener_contract_docs.rs`.
- **AC-8 client acceptance.** CI proves snippet parse and server transport
  only. Whether Claude Code, Codex, Cursor or dsh actually load a config is
  an operator-owned receipt, the same as the existing README note on client
  receipts. The guides and the report must not claim more.
- **AC-9 gates.** Every house gate in the verify list below is green,
  locked. The coverage floor stays at 100% exact. `Cargo.toml`,
  `Cargo.lock`, `deny.toml` and `evals/Cargo.lock` are unchanged; no new
  dependency. The root test suite may use std and the existing tree only.

## Design notes for the council (not blocking)

- Client config formats for Cursor, Codex and dsh must be sourced, not
  recalled. This realm has no web fetch. Use the operator packet or the
  clients' own shipped documentation, and mark any format that cannot be
  sourced as unverified.
- The repository's `evals/wrappers.md` describes dsh's HTTP MCP insert
  overlay for the eval harness. That is evidence of an interop surface, not
  a user-facing config spec.
- Whether a scripted, docs-following test agent should exist to test "wired
  from the docs" end to end without a model is a council decision. It must
  not add a dependency or a paid call.
- Adding evals to CI changes the gate inventory. That needs a decision and
  is outside this issue unless the operator rules otherwise.

## Verify gate (what must run green)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --locked --test <new agent-setup docs suite>
brokkr compile --bundle .brokkr/recipes/self
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
# evals is a standalone crate; run its suite offline and locked
cargo fmt --manifest-path evals/Cargo.toml -- --check
cargo clippy --manifest-path evals/Cargo.toml --all-targets --locked -- -D warnings
cargo test --manifest-path evals/Cargo.toml --offline --locked
```

Commit unsigned. Never push, merge, publish or accept a decision.
