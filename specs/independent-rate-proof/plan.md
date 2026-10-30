# Plan — Independent rate and subprocess proof

Amended 2026-10-29 for the operator continuation; 2026-10-30 supersession note in the Quickstart; the rulings are in
[spec.md](spec.md) § Panel adjudication.

## Technical context

One Rust crate with real-protocol integration suites and deterministic
`cfg(test)` owner seams. No `.specify/templates/plan-template.md` exists; this
plan supplies the requested sections directly. No research beyond source
reads, no dependency, runtime, schema, engine, route or permission change.
The chief commits only this directory; Sonnet 5.5 implements in the next phase.

## Constitution check

Preserve 0001–0005, accepted 0039 concurrency as amended, 0043 guard/method
precedence, 0045 §8 proposal boundaries, 0050 pinned proof identity and 0056
unsigned authorship with operator seal. Follow 0020: the branch-stable owner
assertions stay inline and measured; the subprocess witnesses are public-API,
wire-shaped tests in `tests/`. No coverage exclusion, mock transport,
sleep-as-sync, shared global mutable state, fixed port or mid-band chaos. Exact
coverage and the permissive-only tree are unchanged because no production line
or dependency changes.

## Approach

1. **Owner test** (`src/mcp.rs::tool_rate_uses_exact_monotonic_windows`).
   Replace the body with the literal sequence of AC1: `for _ in 0..1024`,
   attempt 1025 refused with `taken() == 1024`, `t + Duration::from_nanos(999_999_999)`
   refused with `taken() == 1024`, `t + Duration::from_secs(1)` admitted, 1023
   more admitted, next refused with `taken() == 1024`. Inline literals, no
   `const EXPECTED`. Run the two temporary mutations (CAP 2048, WINDOW 2 s),
   observe red, revert, record in the implementation notes.
2. **Dispatch and router witnesses** (`src/mcp.rs` exhausted test;
   `src/api.rs` router test). One-token edits: `ToolRate::CAP` → `1024` at the
   exhausted seed, the router seed, the `taken()` assertion and the refill seed;
   `mcp::ToolRate::WINDOW` → `Duration::from_secs(1)` in the refill
   `checked_sub`. Update the two comments that explain the seed so they do not
   name the constants as the oracle. Nothing else moves.
3. **Scripted peer** in `tests/binary.rs`, beside `McpChild`. A `ScriptedPeer`
   holding a `TcpListener` on `127.0.0.1:0`, a `JoinHandle`, and an
   `Arc<Mutex<Vec<Recorded>>>` where `Recorded` is `{method, id, version_header: Option<String>}`.
   It accepts one connection per request, reads the head through `\r\n\r\n`,
   parses `Content-Length`, reads exactly that body, records, and answers from a
   `VecDeque<Reply>` script (`Reply::Json(Value)` → 200 with
   `Content-Type: application/json`, `Content-Length`, `Connection: close`;
   `Reply::Accepted` → 202 empty). An empty script or a method mismatch answers a
   valid JSON-RPC error (`-32603`, id echoed) and records the request. All waits
   use `common::HELPER_DEADLINE`. `stop()` drops the listener and joins the
   task with a bounded timeout, aborting on expiry, then returns the trace.
4. **Cases** in `tests/binary.rs`, two tests, each table-driven over the two
   defect fixtures of AC3:
   - `mcp_binary_relays_malformed_modern_initialize_and_recovers`: the AC3
     sequence, one launch per defect. Expected trace per launch:
     `initialize`(1, no header), `ping`(no header), `initialize`(new id, no
     header), `notifications/initialized`(header 2025-11-25),
     `tools/call`(header 2025-11-25). Nothing else.
   - `mcp_binary_relays_malformed_ordinary_replies`: per defect, one implicit
     March launch (`tools/call` → malformed reply forwarded; trace: one
     `tools/call` without header) and one modern launch (valid initialize,
     initialized, `tools/call` malformed reply forwarded; trace with the header).
   Each launch: `McpChild::spawn_command` with `["mcp", "--url", url]`; reply
   values compared with `assert_eq!` on `serde_json::Value`; `finish()` asserts
   exit success and an empty remainder; then `peer.stop()` and the trace
   comparison. Assertions that could leave a live child are placed after
   `finish()` or routed through the helper's kill/reap paths.
5. **Gates.** Run the focused filters below and the full inventory on the final
   candidate. A failure is inspected before any existing-owner correction; a
   failure never authorizes changing C6 forwarding or adding a runtime guard.
   Protected provider dispatch belongs to the engine's review phase.

## Project structure and files touched

| Path | Treatment |
| --- | --- |
| `specs/independent-rate-proof/{spec,plan,tasks}.md` | The only tracked paths in the chief's unsigned design commit. |
| `src/mcp.rs` | Inline test bodies only: owner oracle rewrite; one literal in the exhausted seed; comment. |
| `src/api.rs` | Router test only: four literals and one comment. |
| `tests/binary.rs` | `ScriptedPeer`, `Recorded`, `Reply`, two defect fixtures, two tests. |
| `tests/stdio.rs`, `tests/mcp_revisions.rs`, `src/mcp/batch_tests.rs` | Unchanged; executed. |
| `tests/binary/*.rs`, `tests/delivery_*.rs` | Unchanged; executed. |
| Production sources, Cargo files, `deny.toml`, bundles, adapters, agents, shims, docs/decisions, historical specs | Untouched. |
| `.forge/evidence/`, result paths | Ignored receipts through the workspace tool; never staged. |

## Data model

No runtime or persisted data changes. Test-owned state: one monotonic instant,
literal `1024` and `Duration::from_secs(1)`, two malformed reply values, unique
request ids, one ordered per-launch trace. One `ToolRate` `Arc` per production
router, immutable revision per exchange and the bridge-owned fence are unchanged.

## Contracts

Rate: exactly 1024 attempts per fixed monotonic one-second window, charged
before staging; notifications and March members count; refusals neither consume
nor move the window; control and discovery stay available. The owner test proves
the arithmetic, the router test proves ownership and refusal-before-effects, the
retained concurrent tests prove scheduling.

C6: the complete JSON value is forwarded, including simultaneous result and
error. Revision installation separately requires a valid envelope and valid
metadata; a failed modern negotiation fences operations until a valid
initialize/initialized recovers them; a failed legacy initialize installs
nothing and does not fence. The binary witness proves relay, local refusal,
header behavior, recovery, EOF and exit through the real executable; the
in-process witnesses keep the interleaving proof.

## Quickstart and validation

Focused filters (each must match at least one test):

```bash
cargo test --locked --lib tool_rate_uses_exact_monotonic_windows
cargo test --locked --lib exhausted_tool_rate_refuses_before_effects_and_keeps_control_available
cargo test --locked --lib the_router_owns_one_tool_rate_across_connections_revisions_and_members
cargo test --locked --lib revision_admission_has_no_transient_mutations_or_waiters
cargo test --locked --test binary mcp_binary_relays_malformed
cargo test --locked --test binary
cargo test --locked --test stdio complete_malformed_peer_replies_are_forwarded_verbatim
cargo test --locked --test stdio
cargo test --locked --test mcp_revisions
cargo test --locked --test delivery_recipes
cargo test --locked --test delivery_constitution
```

### 2026-10-30 supersession of the engine binding

The block below was written before main was merged. For the current tree read
`.forge/bin/brokkr` as the pinned brokkr binary whose `sha256sum` equals
`binary_sha256` in `.brokkr/recipes/self/tooling.json` (a9893b4 + patches
0001–0004, `29dc3309925a2556b9713b7a9b5fb8089b8e4610e86215fc89554e75634e7cad`),
`bundles/self` as `.brokkr/recipes/self`, and "a9893b4 is absent / unmet
precondition" as satisfied by that pin. The cross-attempt residual is recorded
by decision 0081's 2026-10-30 four-patch note; see
[spec.md](spec.md) § Clarifications 2026-10-30. The original block is kept
for history.

Then the full house inventory, not a reduced copy:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
sha256sum .forge/bin/brokkr   # must equal tooling.json binary_sha256 before the next line
.forge/bin/brokkr compile --bundle bundles/self
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
```

Record actual outcomes and the executable digest. The a9893b4 engine named by
the run context is not in this tree; it is reported as an unmet precondition,
not substituted. No standalone eval without real host-wide nonoverlap and exact
binding. The inherited HIGH remains blocking whatever these commands return.

## Risks

| Risk | Mitigation and retained cost |
| --- | --- |
| Oracle still follows production values through an alias or helper | Inline literals in both owners; review greps the three tests for `CAP`/`WINDOW`; temporary mutation run reported executed or not. |
| Partial refill or consuming refusal escapes the boundary assertion | Full 1024 second-window refill, next refusal, `taken() == 1024` after every refusal. |
| In-process fixture mislabeled as a binary proof | `binary()` is `CARGO_BIN_EXE_swarmail`; `McpChild::spawn_command` with `--url`; the test name says binary; the trace proves the peer saw the child. |
| Late forbidden request after a negative assertion | Trace compared only after child EOF, exit 0 and peer stop/join; unexpected requests are answered and recorded. |
| Hanging child or orphaned peer | Existing kill/reap on every helper failure; peer stop is bounded and aborts on expiry; stderr nulled. |
| Lost queued-while-fenced proof in the subprocess | Accepted by ruling; owned by `tests/stdio.rs` `Consumed` witnesses, retained unchanged. |
| Legacy March over-claimed as fenced | March is an ordinary-exchange case only; fencing asserted for modern negotiation. |
| Test flakes under parallel suites | Ephemeral port per launch, per-test peer and child, no env inheritance, bounded waits. |
| Runtime deviation silently becomes repair scope | Zero production lines; a failing test is inspected, never patched in the bridge or limiter. |
| Green tests mistaken for review integrity | HIGH stays open and blocking; the executable digest that ran is recorded with the re-examination. |
| Engine pin mismatch asserted away | Verify binds to this tree's digest and reports a9893b4 as unmet. |
| Local isolation mistaken for host-wide eval exclusion | Full eval recorded unavailable/not run unless real nonoverlap and binding exist. |
