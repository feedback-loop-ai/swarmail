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

---

# Restart framing — run issue-161-ship-agent-setup-guide-564da145

Restart of #161 from main `5e8d9d8` (a4f552d merged in), on the zero-Codex
`claude-careful` recipe (decisions 0098, 0099; review residuals per 0102).
The branch keeps its design (2104a31) and implementation (66ded9c). Design
revises the spec, then guide, skill and suite. Sound parts stay. This section
supersedes the 52194034 framing where they differ.

## Goal

Ship copy-paste setup guides for Claude Code, Codex, Cursor and dsh, a swarmail
skill and an AGENTS.md block that teaches clear → act → wait → extract, where a
root suite in CI drives every shipped snippet and proves the taught loop on the
wire, including when the flow mailed the agent before it started.

## Reading taken (from current main; design re-derives exact rules)

- **Inbox routing, 0004 + 0112.** The SMTP AUTH username names the inbox
  (0004 §1); an unauthenticated session lands in `default`. A server started
  with `--route-domain` / `SWARMAIL_ROUTE_DOMAIN` (0112; `src/config.rs`,
  `src/smtp.rs` ~1382) routes a recipient `<local>@<label>.<route-domain>` to
  inbox `<label>`, and a routed recipient outranks AUTH. A recipient under the
  route domain that names no inbox is refused `550 5.1.3`. Routed labels are
  `[a-z0-9-]{1,63}` after lowercasing, so the guide's inbox examples stay in
  that alphabet to work on both routes. `swarmail_create_inbox` mints
  `<hex>@<inbox>.<route-domain>` and needs a route domain; without one it
  returns an isError naming `--route-domain`. The default server has no route
  domain. Design reads `specs/recipient-inbox-routing/spec.md` (routing and
  AC lists) and `src/smtp.rs` before writing the taught SMTP route.
- **The flow may mail before the agent starts (F2).** The Straight tasks'
  fixture delivers before the agent spawns (`evals/src/fixture.rs`;
  `evals/src/tasks.rs` scenario comments), and their prompts say the
  application already emailed you. Clear-before-act therefore deletes the mail
  the agent needs in those flows.
- **Tool surface (F-tools).** The live MCP tools are read from `tools/list` at
  run time; the check does not use a hard-coded list. The issue names 18 tools,
  including `swarmail_create_inbox`, `swarmail_list_attachments`,
  `swarmail_get_attachment` and the focused waits `swarmail_wait_for_code` /
  `swarmail_wait_for_link`. The live count is confirmed at design.
- **Timeout near misses (0111, proposed; code on main in `src/mcp/near_miss.rs`
  and `src/mcp/focused.rs`).** A guide that names a near-miss field of a timed-out
  wait must use the name the live output uses, asserted by a real timed-out
  wait on the wire.
- **Selectors (F3).** `swarmail_extract_links` returns plain URLs. Anchor text,
  host and path belong to `swarmail_wait_for_link`'s selectors. The schema
  names are asserted from live `tools/list` at design.

## Frozen guarantees near the work

- 0001 (accept == stored), 0002 (wait registers before it scans) and 0005
  (extraction at ingest) are what the loop's wait and extract steps rely on.
  The guides state them and pin the sentences. Docs only.
- 0003 is not touched. 0004 and 0112 are read as authority for the inbox route.
  Neither is amended.
- The "already mailed, do not clear" rule restates 0001, 0002 and 0005. It
  changes no ruling.

**Blocked: no.** No request contradicts a frozen guarantee. Nothing needs a
decision document: no production change, no dependency, no CI edit. Design
still needs to re-verify the routing rules and the live tool names before
the spec is written.

## Acceptance criteria (restart; each one can go red)

The AC-1 to AC-9 set in `specs/agent-setup-guides/spec.md` stays, except AC-2
(below). Added or changed by the restart:

- **AC-R1 (F1, the route).** The guide says how the application under test
  reaches the inbox, in two taught forms: SMTP AUTH username equal to the inbox
  (`smtp://<inbox>:<any>@127.0.0.1:<smtp port>`), and, on a route-domain server,
  a recipient `<local>@<inbox>.<route-domain>`. The suite sends a real message
  over SMTP to the ephemeral port the way the guide says, and the taught
  `swarmail_wait_for_email` on that inbox returns it. The default SMTP port
  comes from the parsed `Cli` defaults, never a second copy of `1025`. Red
  control, deterministic and without a timeout: a message sent with no AUTH and
  no route lands in `default`, and it is not in the named inbox.
- **AC-R2 (F1, assignment).** The inbox step says to use an inbox the
  environment assigns when one is given, and otherwise to mint one with
  `swarmail_create_inbox` on a route-domain server. The sentence is pinned.
  Red control: the old "pick an inbox name that is yours alone" wording alone
  is refused.
- **AC-R3 (F2, already mailed).** The `markdown` block says that if the flow has
  already mailed you, do not clear; wait, or read, straight away. The behaviour
  is proved, not only the text: the suite delivers a code over real SMTP before
  any clear and runs the taught read path (wait, then extract) to get it. A
  control clears after the delivery and must find the mail gone, which shows
  why the rule exists. Red control: a block with the rule removed is refused by
  the pin.
- **AC-R4 (AC-2 replaced).** The round trip for the taught loop delivers over
  real SMTP as the guide tells the application to send, not through
  `swarmail_seed_email` with an explicit inbox after the clear. The seed tool
  keeps its own case, because the guide teaches it as the seeding step. Red
  control: the F1 variant with AUTH username not equal to the inbox fails the
  round trip.
- **AC-R5 (F3).** The wrong-link advice names `swarmail_wait_for_link`'s
  `anchor_text`, `host` and `path` selectors. The suite asserts those property
  names in that tool's live `inputSchema`. Red control: the link advice that
  puts those selectors on `swarmail_extract_links` is refused.
- **AC-R6 (F4).** The README sentence claims only what is proven, in one
  sentence: the wiring snippets are written into a fresh project and driven by
  a root test in CI, and install commands are marked not-executed. Red control:
  the old "every snippet in it" claim is refused.
- **AC-R7 (F5).** A `~~~` fence and an indented fence (`  ~~~json`,
  `  ```json`) are refused. Each has a red control in the scanner's negative
  tests.
- **AC-R8 (tool names and timeouts).** Every `swarmail_[a-z_]+` token in both
  guide files is in the live `tools/list`, which includes the 18 tools above. A
  real timed-out wait over the wire shows the near-miss field the guide names.
  A misspelt tool name and a made-up near-miss field are each red controls.
- **AC-R9 (AC-3, not measured).** The one-real-model criterion stays open. No
  paid trial is run, and the PR and report say it is not measured. The issue is
  not claimed fully closed.

## Verify gate (house rules § Gates, locked)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test --locked --test agent_setup_docs
cargo build --locked --bin swarmail
cd evals && cargo fmt --all -- --check
cd evals && cargo clippy --all-targets --locked -- -D warnings
cd evals && SWARMAIL_BIN=<candidate target/debug/swarmail> cargo test --locked
brokkr compile --bundle .brokkr/recipes/self
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
```

The root and evals suites run on this tree, and the verify seat runs them
offline. Coverage stays 100% exact (no `src/` line is added). Commit unsigned,
never push, merge, publish or accept a decision.
