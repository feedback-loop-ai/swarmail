# Independent rate and subprocess proof

Created: 2026-10-28. Amended: 2026-10-29 (operator continuation). Status: draft
design. Epic #164, issue #152.
Baseline under proof: `7f2e2129e36577f4c65703c70e20d35a484e6baf`.
Design commit parent: `ca3ad63418c7915f8f5f92208599d056be10d5ad` (tree
`339ec7ea42b98f6233b4362021102bb286114c56`; `src/` and `tests/` byte-identical
to the baseline, the only difference being this directory).
Runs: `follow-up-epic-164-issue-152-on--ed4c6918` (first design),
`issue-152-operator-continuation--3537f6b5` (this amendment).

## Feature and rationale

Repair two independently observed gaps in the candidate's proof. The
deterministic rate test derives its expected count and interval from the
production constants `ToolRate::CAP` and `ToolRate::WINDOW`
(`src/mcp.rs:942-957`, `src/api.rs:3411-3455`), so a changed numerical
contract could remain green. The C6 malformed-reply tests use real TCP but run
the bridge in-process: `tests/stdio.rs::Running::spawn` (`:902`) calls
`swarmail::stdio::bridge` in a tokio task over duplex streams (`:911`), so the
executable path `main` → `run_stdio_bridge` → OS stdin/stdout → exit code has
no malformed-upstream witness. `tests/mcp_revisions.rs::Binary` spawns the real
binary only against a real swarmail server, which never sends a malformed
envelope, and `tests/binary.rs` has no malformed-upstream case. Add independent
literal expectations and real subprocess evidence in the existing test owners.
Production behavior is unchanged.

Framing: `.brokkr/state/tasks/` is absent in this workspace; the framing is
the intake result `.forge/results/4ff86cb1-57cb-4fc2-8048-ab7596b0b127.json`,
the task block and the two supplied evidence files. The chief checked both
manifests with `sha256sum --check --strict` from the repository root:

| Evidence | SHA-256 | Attribution |
| --- | --- | --- |
| `.forge/evidence/issue152-options-review-stop/review-event.json` | `f0e5f8b484967490f7e90dbbea7e37a3102b0cf70f0f1b1a92e41420bba27b44` | Run 88f22317, this baseline; HIGH security-relevant integrity concern and MEDIUM rate-proof weakness. |
| `.forge/evidence/issue152-mixed-review-park/panel-event.json` | `a5639682f6f1e34cc458ed8d276def28d55339e867b68e3815df24ef9467447c` | Different candidate aece916; indeterminate panel; a member identifies missing C6 subprocess evidence there. |

Checksums establish agreement with the supplied bytes, not review authenticity.
The panel event prompted a local inspection only; the C6 gap is established for
this candidate by reading this candidate's tests, and no verdict transfers
across trees. The inherited HIGH remains OPEN, security-relevant and blocking
until independently resolved. A completed panel, this design, checksums and
green application gates cannot resolve it.

This is a proof-only successor to [mcp-engine-followup](../mcp-engine-followup/spec.md).
All its AC1–AC11 remain required, including the original issue's client matrix.
No public contract, frozen guarantee or ruling is changed; no new decision is
declared. Existing proposed [0079](../../docs/decisions/0079-mcp-version-negotiation.md),
[0080](../../docs/decisions/0080-mcp-source-reconciliation.md),
[0081](../../docs/decisions/0081-glm-flash-seats.md),
[0085](../../docs/decisions/0085-deliverability-report.md),
[0101](../../docs/decisions/0101-mcp-options-admission.md),
[0091](../../docs/decisions/0091-mixed-work-seats.md) and
[0092](../../docs/decisions/0092-mcp-candidate-proof.md) retain their meaning
and status. The historical specs and reconciliation ledger remain unmodified.
Should a semantic need appear, the then-next free number must not collide with
0098 and 0099 on `repair/claude-careful-recipe`; only the operator accepts, and
accepted-ruling conflicts wait under
[0045 §8](../../docs/decisions/0045-delivery-repairs.md).

## Clarifications

### 2026-10-29 — operator continuation

- **Roster.** The run continues on the zero-Codex `claude-careful` recipe
  (decision 0099, accepted by the operator 2026-10-29 together with 0098, both
  committed on `repair/claude-careful-recipe` and not present in this tree):
  Haiku 5.5 intake; Opus 5.5, official DeepSeek Flash and Fable 5.1 design;
  Sonnet 5.5 implementation; review by Fable 5.1 correctness, official DeepSeek
  Flash security, Flash guarantees, local Spark GLM-5.3-Flash tests and
  independent Opus 5.5. Codex credits are exhausted; no seat uses Codex. The
  2026-10-28 roster (Terra 6.1 intake, Opus 5.5 implementation, Astra
  correctness/guarantees, the mixed route) is historical. Security and
  guarantees now share one model family (Flash); that is recorded, not repaired
  here.
- **Engine.** The run context names the re-pinned engine a9893b4
  (`swarmail-engine-a9893b4-p3`, brokkr decision 0096). This tree's
  `bundles/self/tooling.json` and `.forge/bin/brokkr` pin and carry 731189f-p3,
  digest `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f`;
  the a9893b4 pin (digest `4873928c94f2677ef552c9e3ed04785a732851b89e8a413c13ce1434d6066be6`)
  exists only on `frozen/claude-careful-a9893b4` / `repair/engine-repin-a9893b4`.
  This design does not re-pin: verify binds to the candidate's own tooling.json
  and records the a9893b4 expectation as an unmet precondition. The inherited
  cross-attempt review-integrity residual is re-examined against whichever
  executable actually ran, by reads only, and recorded as evidence, never as
  closure.
- **C6 in this candidate.** The intake called the `tests/stdio.rs` witnesses
  "spawned-binary"; they are in-process (`Running::spawn`). The add-a-test
  branch of the intake's AC6 applies.
- **AC3/AC4 narrowed.** The binary witness no longer holds initialization while
  queuing stdin; see the adjudication below.

## Panel adjudication

Both positions agree on: literal `1024` and one second in every
contract-bearing assertion, a real spawned binary with a real loopback peer,
whole-JSON-value equality, trace-through-exit negatives, bounded cleanup, zero
production lines, no new ADR, no dependency, and the OPEN HIGH. Those are
adopted without further argument. The genuine conflicts are ruled here:

| Question | Ruling | Losing side and its cost |
| --- | --- | --- |
| Router test oracle | **Literal in both owners.** `src/api.rs` uses `set_window(never, 1024)`, `assert_eq!(rate.taken(), 1024)` and `checked_sub(Duration::from_secs(1))`. Adopted from both positions; the intake's AC4 permission to keep constants there is rejected. | The intake's leniency loses: a 512-cap limiter would still pass the ownership proof. Cost is three duplicated literals, which is the property being bought. |
| Second-window proof | **Full refill, then refusal.** After the one-second admission, the remaining 1023 admissions succeed and the next refuses at that same instant; `taken()` is asserted `1024` after each refusal (robustness). | Simplicity's two post-boundary admissions lose: a partial refill or a refusal that consumes would pass. Cost: a few more assertions on measured lines. |
| Hold/release in the binary | **Cut (simplicity).** The binary witness is a sequential scripted peer. Each stdin line is written after the previous owed stdout line has been read; no milestone channel, no second `HeldPeer`. | Robustness and the 2026-10-28 AC3 lose the queued-while-fenced choreography. What is lost: the subprocess never proves that lines queued in the OS pipe during the fence are read only afterwards. Robustness itself concedes pipe-write completion is not consumption, so that claim was never scheduling-exact in a subprocess; it stays owned by the in-process `Consumed` witnesses (`tests/stdio.rs:1559`, `:1680`), which are retained unchanged. The binary proves what only the binary adds: startup, `--url` binding, OS-pipe framing, verbatim relay through `main`, local refusal after failed negotiation, header behavior, recovery, EOF and exit code. |
| Post-failure controls | **Robustness's full set.** After each malformed modern initialize: `tools/call` and `tools/list` refused locally with no peer record; an effectful notification silent with no peer record; `ping` reaches the peer without `MCP-Protocol-Version`; a valid `initialize`/`initialized` recovers and a `tools/call` then reaches the peer with the negotiated header. | Simplicity's sequence omitted the notification; it is added. Cost: one more line per launch. |
| March failure shape | **Implicit March is an ordinary-exchange case only.** A malformed reply to a legacy initialize leaves the phase `Legacy` (no fence); asserting fencing there would over-claim. | Nothing lost; this is robustness's honesty constraint applied to simplicity's table. |
| Peer helper | **Local to `tests/binary.rs`, reusing `McpChild::spawn_command` and `binary()`.** Ephemeral `127.0.0.1:0`, `--url` on the child, one accepted connection per request, `Connection: close`, every unexpected request answered with a valid JSON-RPC error and recorded. | Robustness's `SWARMAIL_URL` env binding loses to the helper's existing `--url`; `tests/mcp_revisions.rs::Binary` is not grown; no shared `tests/support` crate. |
| Decision and spec directory | **Amend `specs/independent-rate-proof` in place; declare no decision.** | A fresh directory or ADR loses: a test-only proof repair has no semantic content, and a new number would collide with the recipe branch. |

## Acceptance Criteria

These are downstream obligations. `drafted` certifies only the committed
design; it certifies no implementation, gate, protected review or delivery.

1. **AC1 — Independent numeric boundary (owner test).** In
   `src/mcp.rs::tool_rate_uses_exact_monotonic_windows` (inline, `#[cfg(test)]`,
   measured, decision 0020 rule 1), with a generated monotonic `t` and
   `set_window(t, 0)`: exactly 1024 `take_at(t)` calls return true; the 1025th at
   `t` returns false and `taken() == 1024`; `take_at(t + 999_999_999 ns)` returns
   false and `taken() == 1024`; `take_at(t + 1 s)` returns true; 1023 further
   calls at `t + 1 s` return true; the next at `t + 1 s` returns false and
   `taken() == 1024`. Every expected count and instant is a test-owned literal.
   The test names no production constant, alias or helper derived from
   `ToolRate::CAP` or `ToolRate::WINDOW`; the only production symbol is the type
   under test and its seams. A cap of 512 or 2048, a window of 999 ms or 1001 ms
   or two seconds, a strict-greater boundary, a partial refill, a refusal that
   consumes or a refusal that restarts the window must fail a behavioral
   assertion. The temporary CAP=2048 and WINDOW=2 s mutations are run and reverted
   before commit, or reported as not executed. No sleep, no timed HTTP exhaustion
   loop, no clock framework, no constant-only or string-only assertion.
2. **AC2 — Literal contract in the dispatch and router witnesses.**
   `exhausted_tool_rate_refuses_before_effects_and_keeps_control_available` seeds
   `taken = 1024` at its generated future start (seam preserved).
   `the_router_owns_one_tool_rate_across_connections_revisions_and_members`
   seeds `set_window(never, 1024)`, asserts `taken() == 1024`, and refills from
   `Instant::now().checked_sub(Duration::from_secs(1))`. No other line of either
   test changes: the production `Arc` observed through
   `observe_next_router_tool_rate`, fresh connections, all three revisions, the
   silent notification 202, the March batch, the invalid-argument attempt that
   still counts, the `mutations == 0`/`subscriptions == 0` observers under
   refusal, the admitted positive controls after refill, unchanged chaos, and
   control/discovery availability are retained. Existing batch and binary
   wait/seed/ping concurrency tests are retained unchanged.
3. **AC3 — Binary malformed modern initialization.** In `tests/binary.rs`, for
   each complete HTTP-200 envelope defect — (a) missing `jsonrpc` with a valid
   `result`; (b) `jsonrpc`, `result` and `error` together — the test launches the
   Cargo-built `swarmail mcp --url http://127.0.0.1:<ephemeral>` through
   `McpChild::spawn_command` with piped stdin/stdout and nulled stderr, against
   a scripted loopback TCP peer local to the file. The defect is the only
   defect: ids match and `protocolVersion` `2025-11-25`, object `capabilities`
   and `serverInfo` name/version are otherwise valid. Sequence per launch:
   modern `initialize` → the whole malformed value reaches stdout as one line
   and equals the sent `serde_json::Value`; `tools/call` and `tools/list` →
   local `-32600` initialization refusals on stdout, no peer record; an effectful
   notification (`swarmail_clear_all`) → no stdout line, no peer record; `ping`
   → reaches the peer and its recorded request carries no `MCP-Protocol-Version`
   header; valid `initialize` (new id) → forwarded result; `notifications/initialized`
   → peer answers 202, no stdout line; `tools/call` → reaches the peer with
   `MCP-Protocol-Version: 2025-11-25` and its reply is forwarded. Each stdin line
   is written after the previous owed reply has been read; the test holds nothing.
4. **AC4 — Binary ordinary exchanges, trace and lifecycle.** For both defects,
   two further launches prove verbatim forwarding of a malformed `tools/call`
   reply under the implicit March session (no initialize; the recorded request
   has no version header) and under a negotiated modern session (header
   `2025-11-25`). Every launch ends by closing stdin, a bounded wait for stdout
   EOF and exit status 0 with no extra stdout line, then stopping the peer
   listener and joining its task; only then is the complete recorded request
   list compared to the expected list, in order, by method, id and
   version-header presence. The peer answers any unexpected request with a valid
   JSON-RPC error and records it, so a regression ends in an assertion, not a
   hang. Every accept, read, write, stdout read, exit wait and join is bounded by
   the existing helper deadline; failure kills and reaps the child and aborts or
   joins the peer. Existing June/November, legacy and in-process C6 coverage in
   `tests/stdio.rs` and `tests/mcp_revisions.rs` is retained unchanged. No
   whitespace or key-order contract is introduced; equality is on the parsed
   value.
5. **AC5 — Preserve issue #152 and the guarantees.** All predecessor AC1–AC11
   apply: guard-first OPTIONS 405/Allow/no-CORS, method/header and POST
   precedence, three revisions, immutable exchange revisions and lifecycle
   fences, modern-array pre-effect refusal, March concurrent members and reply
   order, the exact independent thirteen-tool catalogue (seven reads, six
   mutations), served descriptions and source attribution. C1–C6 and the
   separate OPTIONS MUST mismatch stay explicit; no full-conformance claim and no
   C6 rejection repair (the bridge relays, it does not repair). Zero production
   lines change; `Cargo.toml`, `Cargo.lock`, `deny.toml`, bundles, adapters,
   agents, shims and served descriptions are untouched. Synchronous storage
   before SMTP 250, subscribe-before-scan, oldest-only pruning, isolated inboxes
   and ingest extraction keep their owners. No accepted decision or historical
   spec is rewritten; the proposed statuses listed above stay proposed.
6. **AC6 — Fresh final-candidate verification.** Record commit, tree, clean
   state, executed commands, outcomes and executable identities. Run the entire
   current [house gate inventory](../../docs/house-rules.md#gates--part-of-every-change):
   locked fmt, clippy and tests; pinned compile; exact coverage with zero missed
   lines, branches and functions; coverage selftest; publish-guard and seal
   proofs; licenses; checked shims; fail-closed native-tls absence. Every
   focused filter in [plan.md](plan.md) must execute a nonzero intended count.
   Bind the absolute engine executable to this tree's `tooling.json` digest
   `ab93555806ee4ac402457e40c18250f0eead8e6a0588eb8c508682fe0c6a885f` before
   `brokkr compile --bundle bundles/self`; record a9893b4 as an unmet
   precondition if that is what the run context expects. No install, pin
   substitution or historical-pass reuse. A command that could not run is
   reported as not run. Any later tracked edit requires fresh successor-bound
   verification and review.
7. **AC7 — Protected review and the unresolved HIGH.** Review is the five
   separately authenticated final-candidate results of the 0099 roster: Fable
   5.1 correctness, official DeepSeek Flash security, Flash guarantees, local
   Spark GLM-5.3-Flash tests (effort not applicable) and independent Opus 5.5,
   with distinct invocation identities, read-only boundaries, exact providers,
   singleton chains, no unauthorized fallback, worst verdict, unknown-fact
   refusal and the hard security hold. No seat uses Codex. The inherited HIGH
   security-relevant protected-review-integrity finding stays open until
   independently and legitimately resolved; it is not reclassified, no attack is
   reproduced, no boundary is bypassed, no verdict is fabricated, and panel
   completion is not resolution. The re-examination against the engine in use
   is recorded as evidence with the executable digest that actually ran. An
   unresolved HIGH blocks delivery completion even with every gate green.
8. **AC8 — External obligations remain separate.** Full standalone eval
   requires actual host-wide nonoverlap through cleanup and exact executable
   binding under accepted 0063/0076; otherwise it is recorded unavailable/not
   run. No private lock or isolated worktree supplies that. The four owed
   real-client receipts (Claude Code, Codex, MCP Inspector, Claude Desktop), the
   #151 baseline/merge hold, #152-before-#153 sequencing, CI-only
   platform/network/Helm/Docker/RustSec/port-80 and release proofs, operator
   decision numbering, acceptance and seal all remain owed elsewhere. No paid
   trials, push, merge, publication, tag, issue closure or decision acceptance.

## Retained costs and limits

The shared per-router fixed window retains contention, adjacent-window bursts,
restart allowance and independent routers' allowances; no fairness or rolling
limit is promised. C6 forwarding remains a compatibility deviation. The binary
witness is an external lifecycle proof, not a scheduling proof; queued-while-fenced
ordering stays with the in-process `Consumed` tests. A committed mutation can
outlive a lost reply; retries may duplicate. Webhooks remain best effort.
Memory-only mail disappears on process death. Caps do not bound arbitrary inbox
counts. This proof adds no global resource, exactly-once or failure-recovery
guarantee, and it does not resolve the HIGH.
