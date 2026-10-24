# Gate evals against the candidate build

- Status: implemented; local gates and pinned verify passed; protected review pending; 2026-10-24.
- Scope: epic #192, issue #201; fixture provenance follow-up for #149.
- Original baseline: `dc1822a45e4d2ae8326b44baf22818187a8320e9`.
- Continuation candidate: `26df727`, after inherited design `ae4d2bd`.
- Intake: `continue-epic-192-issue-201-on-c-3c99fb84`, classified `engine`;
  continues stopped run `epic-192-issue-201-on-current-or-cbc7d3d2`.

## Feature and rationale

CI and hermetic verify must exercise the standalone eval crate against the
Swarmail executable produced by that candidate's successful root test build.
Contributors and operators need eval-only defects to prevent green results,
and verbose children must not grow readiness retention or block pipe draining.

At `26df727`, the CI and pinned verifier wiring, artifact selection, boundary
proofs, one-shot readiness and documentation corrections already exist. This
continuation preserves them and closes proven gaps instead of rebuilding the
feature. In particular, the retained-server regression has a blocking FIFO
open and an unbounded child wait before its marker deadline. A drainer
regression can prevent the test from ever reaching that deadline. Repair the
proof locally and reconcile inherited evidence with the final candidate.
This is a source-observed failure-path gap, not a reproduced runtime deadlock.

### Dated clarification — 2026-10-24 continuation

The original design's statements that CI excludes evals, guidance calls 0055
proposed, and server stdout retains a growing queue describe `dc1822a`, not
`26df727`. The original accumulation affected retained server receivers;
agent launch dropped its receiver. Keep AC1–AC11 as the feature contract,
strengthen AC7's bounded-failure proof, and reassess outstanding tasks from
evidence. Unchecked boxes do not prove missing code.

The previous implementation handoff claims all gates passed. Run-local
`validation/gates.json` records earlier root/coverage timeouts;
`validation/full-tests.json` records later root/eval exit 0 and an absolute
artifact; `validation/coverage-full.json` records later coverage/tamper exit 0.
These are distinct inherited receipts, not fresh proofs of this continuation.
Receipts without candidate/pin/command identity cannot alone certify the final
candidate. This design claims neither full gate success nor protected review.

## Authority and scope

[0055](../../docs/decisions/0055-agent-eval-harness.md) is accepted by the
operator on 2026-10-19. Its conditional gate deferral ends with the already
landed batch-5 proof ownership in accepted
[0050](../../docs/decisions/0050-delivery-gates.md). This design implements
that follow-up and repairs internal buffering; it adds no public task,
result schema, protocol, frozen guarantee or amendment to an accepted ruling.
No new decision is declared or accepted. If implementation requires a
contradiction of an accepted ruling, return the design instead of silently
expanding this authorization.

Preserve synchronous accept/store, subscribe-before-check, oldest-only
pruning, ingest extraction, AUTH inbox isolation and the current MCP Origin
boundary. Evals remains an unpublished standalone workspace, outside root
coverage and packaging, with dependencies drawn from root-locked package
versions and sources. No root production code, dependency, license allowance,
new workflow route or model measurement is requested. Offline verify installs
nothing. Protected review, security hold, engine identity and shim integrity
remain mandatory under [0050](../../docs/decisions/0050-delivery-gates.md) and
[0057](../../docs/decisions/0057-codex-delivery-recipes.md).

## Panel synthesis and explicit rulings

Both continuation positions were read from the run-local simplicity and
robustness files. Their common recommendation is preservation with evidence;
the rulings below resolve their different thresholds for additional work.

1. **Adopt simplicity's existing owners and robustness's executed proof.**
   Retain CI quality and both OS test jobs, canonical pinned verify, explicit
   commands and the existing outer `eval-dispatch` proof mode. Reject a new
   workflow, gate registry, shim, helper crate or generic proof framework.
   Structural wiring tests alone lose to executed refusal evidence. Cost:
   command duplication, both-platform eval runtime and isolated proof runs.
2. **Retain the implemented artifact selector.** One successful root-test
   invocation supplies canonical package/ordinary-binary/non-test artifact
   identity. Keep the current small Python selectors, failure propagation and
   quoted path handling. Reject rewriting to jq merely to match the old plan,
   a second build pass, guessed paths or combining pinned selection with the
   local eval fallback resolver. Cost: two maintained selectors and explicit
   negative controls; provenance does not prevent hostile local replacement.
3. **Require the narrow regression repair now.** Simplicity permits no code
   change if proof is adequate; robustness identifies the unbounded FIFO open
   and child wait in the retained-server control. Source inspection confirms
   that gap, so the zero-change option loses here even without a reproduced
   hang. Require cancellable/deadline-aware rendezvous, bounded child waiting
   and cleanup on failure in that test's owner. A timeout around a blocking
   worker alone loses because cancellation leaves its thread blocked. Cost:
   local test complexity and a deliberately stalled-drainer control. Reject
   a production lifecycle redesign or generic supervisor.
4. **Preserve the stdout fix, not tracking-only closure.** Retain one-shot
   parsed readiness, log-only agents, continuous draining and discard through
   newline for oversized startup lines. Reject awaited bounded sends, lossy
   announcement queues, configurable streams and RSS-based bound claims.
   Cost: truncated old logs and rejected oversized announcements; there is no
   whole-process memory/confinement promise. AC6–AC7 remain mandatory.
5. **Keep boundary proof separate from protocol expansion.** Adopt the exact
   real-server MCP shape assertion and existing decoder refusal controls;
   retain harmless-field/block tolerance and the existing lock tuple
   predicate. Reject SSE, negotiation, structured-only support and another
   parser/dependency policy. Cost: current-protocol scope only, and tuple
   subset does not prove feature-graph equivalence.
6. **Reconcile evidence and authority without rewriting history.** Adopt both
   panels' already-accepted 0055 status, actual wired-gate guidance and
   canonical fixture path. Reuse this spec directory; preserve dated history
   and genuine deferred capabilities. Reject handoff prose as fresh proof,
   marking inherited tasks done without evidence, or an invented external
   #149 edit. Cost: final-candidate verification and an explicit operator
   follow-up while external wording remains unavailable.

Robustness's store/crash/webhook/cap/hostile-input/TLS discussion defines the
limits of eval evidence, not additional implementation scope. Preserve the
root proof owners and accepted boundaries: commit-before-reply can leave an
uncertain acknowledgment and duplicate retry; notifications remain
best-effort; later insertions can evict oldest mail; unconfined processes and
wire-byte bounds are not a global resource budget. Reject turning this
continuation into a broad service audit. Its cost is no new assurance for
those distinct behaviors from eval green alone.

## Acceptance Criteria

- **AC1 — Mandatory execution.** CI quality runs eval fmt and locked,
  all-target clippy with warnings denied; both existing OS test jobs run the
  full locked eval suite after successful root tests. Pinned verify runs the
  same checks offline. No path-based skip or allowed failure; publication
  prerequisites continue to require these jobs. Job deadlines are finite.
  An eval-only defect fails the job/seat.
- **AC2 — Binary provenance.** Gates resolve exactly one distinct executable
  from successful root-test Cargo artifact messages, matching the canonical
  root package, binary target `swarmail`, non-test profile and non-null
  executable. The resulting path is absolute and executable. Missing,
  ambiguous, wrong-package, wrong-target, relative, non-executable or
  test-harness-only artifacts refuse. A failed root test/build cannot be
  masked by capture or parsing. A custom target
  directory containing spaces succeeds; a decoy inherited `SWARMAIL_BIN`
  is overwritten, and eval tests do not invoke fallback builds.
- **AC3 — Lock and workspace boundary.** The full eval suite executes the
  existing unpublished/standalone/package exclusion and no-native-tls/openssl
  tests. Independent foreign name, version and source mutations fail the
  same subset predicate; unchanged inputs pass. Root manifest, lock,
  packaging allowlist, license allowlist and coverage denominator are intact.
  Package-tuple subset is not feature-graph equivalence.
- **AC4 — MCP boundary.** Real Swarmail initialize and tool responses prove
  protocol `2025-03-26`, correlated ids, one `type: text` block containing
  JSON, `isError: false` on successful tools, and no `structuredContent`.
  Wrong id/version, JSON-RPC error, missing result/text, non-JSON text and
  missing/non-false `isError` refuse through the existing client decoding
  path. Structured-only data cannot bypass missing text. Harmless additional
  fields/blocks retain existing client tolerance; acceptance still traverses
  real SMTP/HTTP/MCP with no fake server replacing it.
- **AC5 — Harness truth.** All 16 oracle cases pass; all 16 null cases fail
  with their declared idle classes; two overlapping isolation trials pass.
  Existing scoring/report, failed-delivery and lifecycle controls pass.
  Infrastructure failures remain harness errors, never ordinary agent
  failures. Trials retain owned loopback port-zero servers and AUTH inboxes.
- **AC6 — Finite readiness retention.** No unbounded stdout-line channel
  remains in the runner. Each server can publish at most one parsed address
  pair; agents allocate no readiness forwarding queue. The drainer continues
  after delivery, timeout or receiver drop and never awaits a consumer.
  The existing 64-KiB shared per-process log tail, finite read buffer and an
  at-most-8192-byte startup line bound remain explicit. An oversized line is
  discarded through its newline; its suffix cannot become an announcement.
  This bounds retained output data, not child memory or allocator overhead.
- **AC7 — Readiness and cleanup proof.** Deterministic controls cover noise
  before readiness, one valid announcement amid noise, sustained server output
  after readiness while `Proc` stays alive, a chatty agent, exact-bound and
  oversized lines, a valid-looking oversized suffix, EOF/read error before
  readiness, duplicate announcements and receiver drop. Assertions prove
  bounded retained data and continued pipe progress. Existing startup/health/
  SMTP probes, deadlines, server-death reporting and terminate/kill/reap of
  children and descendants remain effective. Use markers/barriers and real
  deadlines, never sleep as synchronization or RSS timing as a bound proof.
  In the retained-server test, bound rendezvous, output completion and child
  exit from before the first potentially blocking operation. Keep `Proc`
  alive while the child crosses its 131072-byte stdout write. On failure,
  release/cancel owned I/O, terminate/kill/reap the process group and finish
  owned tasks without a stranded blocking thread. An isolated deliberately
  stalled-drainer control must report the intended deadline failure and
  finish cleanup within a declared finite outer budget; a restored control
  must pass. Outer harness termination alone is not a passing red proof.
- **AC8 — Executed gate refusals.** Isolated eval format, clippy and test
  defects each fail actual pinned verify dispatch at the relevant gate;
  restored controls pass it and a clean candidate passes the dispatch.
  Exercise binary selection refusals and space-containing target paths.
  CI-equivalent commands fail on the same defects. Keep shim-tamper and
  recorded-engine identity controls; candidate helper substitution cannot
  choose the verification algorithm. Preserve evidence with exit/result and
  decisive diagnostics, not only command-string assertions. Bind each receipt
  to candidate HEAD plus any diff, pinned engine/script identity and exact
  command. Reconcile earlier timeout and later success receipts explicitly;
  missing or invalidated proof must run again on the relevant final code.
- **AC9 — Documentation and fixture provenance.** Both older eval specs and
  `evals/README.md` cite accepted 0055 (operator, 2026-10-19), current gate
  wiring and candidate-binary prerequisite. Reconcile dependent plan/tasks
  and any remaining stale eval manifest comment without erasing dated
  history. Keep the deferred brokkr driver, unconfined execution, HTTP ICS download and
  HTTP support-reply limitations. Document #149's fixture as
  `evals/src/fixture.rs`, invoked with `swarmail-evals fixture`. Locate any
  erroneous repository path before replacing it. If external #149 text
  remains unavailable, record that precise outstanding correction, canonical
  replacement and operator follow-up; do not claim an external mutation.
- **AC10 — Inventory and full validation.** The single house-rule gate
  inventory retains eval fmt/clippy/tests and the test-build binary prerequisite.
  Retain every current gate, exact coverage/selftest, publish/seal proofs,
  license/native-TLS checks, pinned proofs and CI-only distinctions. Run the
  eval suite and complete inventory for implementation; record real failures
  or environmental blockers as outstanding. Design drafting and structural
  validation are not implementation success or a gate-pass receipt.
- **AC11 — Delivery isolation.** Changes stay in this checkout; concurrent
  worktrees are neither reset nor silently imported. Reconcile any later
  overlap deliberately and rerun affected proofs. Commit unsigned per
  [0056](../../docs/decisions/0056-signing-policy.md); never push, merge, close
  issues or accept decisions. This chief commit contains exactly this
  directory's spec, plan and tasks, and remains HEAD for design validation.

## Honest limits

Eval success proves these task paths, not TLS, persistence/restart durability,
pruning, webhook reliability, hostile-input CPU bounds or thousand-session
capacity. Root and CI-owned proofs retain those distinct responsibilities.
No queue-residual admission is assumed: if the selected fix cannot be proved,
return with its exact outstanding evidence and proposed follow-up for review;
tracking alone does not satisfy AC6–AC7 or authorize a clean completion claim.
