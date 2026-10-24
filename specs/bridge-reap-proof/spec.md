# #179 follow-up: prove bridge reaping through its owning handle

Created: 2026-10-23. Updated: 2026-10-24. Status: residual design draft.
Story: epic #192, #179 correctness-review residual only.
Historical baseline: reviewed #179 source replay 86c851f on main df8630f.
Current preservation baseline: c8364ed208612b72955cf271c2341e7c16f40c3f.

The original design below explains the implementation now present at
`c8364ed`. The dated residual clarification defines the remaining work;
AC1–AC8 stay unchanged and binding. Historical implementation instructions
do not authorize reimplementing or refactoring the completed candidate.

## Original feature and rationale (2026-10-23)

Make `never_exiting_bridge_child_is_timed_out_killed_and_reaped` prove that
`McpChild::finish_with_deadline` has completed the reap of its own real
child before returning. The current test saves a numeric PID, calls
`try_wait`, then probes that PID with raw `waitpid(WNOHANG)`. After reaping,
another parallel test can own a new child with the same PID; that probe
can observe or reap the unrelated child. Removing only the raw probe would
also leave a proof gap: test-side `try_wait` could perform a missing reap.

The replacement observes the original Tokio handle's completed state
before any test-side operation can complete it, then checks its cached
SIGKILL exit status. This preserves the original kill-and-reap obligation
without a second lookup by reusable process number.

## Original panel synthesis and rulings (2026-10-23)

Read both run-local positions, `simplicity.md` and `robustness.md`, and
intake `epic-192-179-follow-up-remove-th-91e4bdaa.md` under
`.brokkr/state/`. These are design evidence, not execution receipts.

- Adopt simplicity's single owner and local ordering correction: remove
  the saved PID and raw waitpid/ECHILD evidence; observe `id().is_none()`
  before reading status. Reject pidfd, process-table matching, new FFI,
  dependencies, serialization and forced PID-reuse stress. The accepted
  cost is reliance on the pinned Tokio completion contract rather than an
  independent OS query; no claim to audit Tokio internals is made.
- Adopt robustness's capture-cleanup-fail discipline: retain the finish
  result without unwrapping it, capture completion immediately, and on a
  false observation perform bounded cleanup before reporting failure.
  Diagnostic assertions must not hide the lifecycle failure. This costs a
  few explicit local failure-path lines; a bare panic is insufficient.
- Both positions converge on the essential mechanism; there is no genuine
  architectural conflict to average. Resolve intake's optional extra
  control in favor of robustness's explicit mode matrix: require both named missing-reap and
  live-child mutants, each in both fixture modes. The cost is four red
  executions and restoration bookkeeping, justified by the two explicit
  failure requirements and the loop's early-panic behavior.
- Keep robustness's broader mail/storage failure analysis as preservation
  context only. Reject any implementation expansion into those owners;
  this test cannot prove crash durability, webhook delivery or arbitrary
  descendant cleanup. Simplicity's narrow boundary governs the patch.

## Scope and accepted authority

For this residual, final tracked changes are restricted to this directory's
design artifacts: the chief commits exactly spec.md, plan.md and tasks.md;
subsequent work edits only plan.md for directly necessary evidence and
clarifications. Temporary mutation of `tests/binary/helper_reliability.rs`
is permitted only for missing proof and must be restored byte-for-byte to
`c8364ed`. Reviewed helpers in `tests/binary.rs`, common/protocol tests,
production,
dependencies, decisions, recipes, scripts and CI remain unchanged. Other
lanes' reservations from intake remain binding.

This corrects a test's evidence, not a frozen guarantee, public contract
or accepted ruling. No decision is proposed or accepted. Preserve 0001–0005
and apply accepted [0020](../../docs/decisions/0020-concurrency-tests-and-the-coverage-gate.md),
[0050](../../docs/decisions/0050-delivery-gates.md),
[0056](../../docs/decisions/0056-signing-policy.md) and
[0057](../../docs/decisions/0057-codex-delivery-recipes.md).

## Acceptance Criteria

1. **AC1 — Real deterministic experiment.** Both `bridge-open` and
   `bridge-closed` retain flushed JSON readiness, initial live-child
   observation, one Pending poll, and the immediate expired deadline. The
   closed mode additionally observes stdout EOF while the child stays
   alive. No sleep synchronization, elapsed-time validity assertion,
   forced PID allocation, suite serialization or replacement fake child.
2. **AC2 — Helper-owned reap.** Immediately after the final finish call
   resolves, capture `bridge.child.id().is_none()` before unwrapping its
   result, diagnostic assertions, test-side `try_wait`/`wait`, or cleanup.
   False always fails, even if later cleanup succeeds. Only after true may
   the test read the same handle's cached `Some(status)` via `try_wait` and
   require `status.signal() == Some(libc::SIGKILL)`. Preserve timeout and
   successful-cleanup diagnostics as additional checks.
3. **AC3 — No reusable-PID evidence.** Remove the saved PID and post-reap
   raw waitpid/ECHILD check. Introduce no post-reap numeric-PID observation
   or signal. A local comment explains both the completion-before-status
   ordering and why it prevents observer-assisted reaping.
4. **AC4 — Failure hygiene.** If the captured completion observation is
   false, call existing bounded `kill_and_reap` on that same handle before
   failing with `bridge helper returned before completing child reap`,
   fixture mode and cleanup outcome. Never replace the captured failure
   with a new successful observation. Cleanup failure remains failure;
   kill-on-drop is only the existing unwind fallback. Both negative
   controls must demonstrate successful explicit cleanup, with no leaked
   fixture children.
5. **AC5 — Falsifiable proof.** Temporary named mutants
   `bridge_timeout_returns_without_reaping` (request kill but omit wait)
   and `bridge_timeout_returns_with_live_child` (omit both) each compile
   and fail at AC4's lifecycle diagnostic for each fixture mode, before
   any diagnostic-string failure. Capture commands, exit codes, mode,
   lifecycle text and cleanup result. Record exact source hashes before
   mutation and after restoration; restore byte-identical final source
   before focused green tests and all final gates. Mutants stay entirely
   in the permitted test file and are never committed.
6. **AC6 — Preserve #179.** Startup silence/partial-line/chatter bounds,
   stderr/stdout/source/exit diagnostics, and HTTP read-error rejection
   after a parseable prefix continue to pass. Unrelated reliability tests
   and all previously reviewed helpers remain byte-identical. No new
   dependency, production seam, coverage exclusion or denominator change.
7. **AC7 — Fresh verification.** The exact regression and the full
   `helper_reliability` suite pass with locked dependencies and ordinary
   parallel test execution after restoration. All gates in the
   [house inventory](../../docs/house-rules.md#gates--part-of-every-change),
   pinned engine identity/compile, exact 100/100/100 coverage and selftest,
   publish/seal proofs, licenses, fail-closed native-tls absence and actual
   outer offline verify must pass on this candidate. Old receipts are
   historical only; a structural design check is not implementation proof.
8. **AC8 — Delivery boundary.** Retain intake's `engine` classification
   and four reviewers: Qwen Max correctness, Claude Opus security,
   official DeepSeek Flash guarantees, DashScope GLM-5.3 tests. No fallback
   or DeepSeek Pro. Root owns integration, CI-only proofs and issue closure.
   Author commits are unsigned; this lane never pushes, merges, tags or
   performs registry operations.

## Residual clarification and panel ruling — 2026-10-24

Read both current positions under `.brokkr/state/design/positions/` and
intake `.brokkr/state/tasks/epic-192-issue-179-residual-foll-c304165b.md`.
Seq-375 correctness reported two information findings: the plan conflates a
historical engine digest with the operative pin, and this review could not
read AC5's predecessor-only raw receipts. This continuation repairs those
two findings without reopening the completed implementation.

- **Adopt simplicity's scope:** retain this `bridge-reap-proof` slug,
  correct the existing plan, recover verified receipts first, and rerun
  only unproved matrix cells. Reject a new spec directory, mutation runner,
  permanent selector, evidence service, archive migration or runtime fix.
  Cost: a small manual evidence index and explicit handoff per review;
  ignored evidence is not promised to survive arbitrary checkout transfer.
- **Adopt robustness's evidence threshold:** require provenance, actual
  selected execution, lifecycle failure, successful same-handle cleanup,
  exact restoration and readability in each dispatched reviewer workspace.
  Reject a digest-only edit or summary-only closure. Cost: provenance
  inspection and up to four independent red executions, even though the
  findings have info severity. A checksum alone does not authenticate origin.
- **Reconcile without invented disagreement:** both panels support this
  recover-first minimum. Valid historical reds need no new timestamp;
  historical greens cannot satisfy fresh AC7. Missing access or provenance
  leaves a cell unproved and invokes bounded regeneration after recording
  the attempted recovery. Neither inaccessible storage nor static reasoning
  can be called verified execution.
- **Rule on artifact scope:** simplicity's minimal implementation patch is
  retained, but the chief must update all three existing design artifacts
  to make the remaining work unambiguous and satisfy the requested design
  handoff. The cost is documentation churn in spec/tasks as well as plan;
  adding a second feature or relaxing the validator would cost more scope.
- **Reject broader hardening:** robustness's mail/storage/webhook analysis
  remains preservation context, not an expansion of this proof. No new
  claim about durability, arbitrary descendants, TLS or SMTP is made.
  Keep engine classification and the four policy-selected reviewers under
  accepted 0057; a documentation-only final diff does not downgrade risk.

### Residual acceptance checks (apply AC1–AC8; do not replace them)

1. **R1 / AC7:** The old full `e525babf…` digest and original source hashes
   remain explicitly historical in plan.md. A dated clarification names
   root re-pin `238372d`, the full current `ab935558…` digest and current
   source hashes. Quickstart requires the candidate-local executable's
   SHA-256 to equal current `bundles/self/tooling.json` before execution;
   no operative instruction requires the historical digest.
2. **R2 / AC5:** A search record names actual predecessor journal/worktree
   references inspected, results and access limits. Each recovered row is
   bound to its originating run, candidate/test/helper bytes, mutation,
   selected mode, command, nonzero selected count, exit, raw output and
   restoration evidence. Unavailable records are never fabricated. Any
   historical source difference must be explained and relevant equivalence
   established; otherwise regenerate that row.
3. **R3 / AC4–AC5:** The index contains all four required mutant/mode rows
   from plan.md, independently executed. Each compiled test exits nonzero
   at `bridge helper returned before completing child reap`, names the
   intended mode and shows successful explicit cleanup. Compiler/setup
   failure, zero selected tests, cleanup error, a first-mode-only panic or
   later diagnostic mismatch cannot count. Recover valid rows; regenerate
   only missing or unverifiable rows and preserve unsuccessful attempts.
4. **R4 / AC1–AC6:** After every temporary experiment and before any green
   verification, the mutated file equals its saved candidate bytes. Final
   hashes and diff prove both test files equal `c8364ed`, with no mutant,
   synthetic result or selector left. Keep readiness, closed-mode EOF,
   live observation, Pending poll and completion-before-status unchanged.
5. **R5 / AC5, AC8:** The current run's ignored artifact index points to raw
   receipts and hashes, distinguishes recovered historical reds from new
   reds and fresh greens, and records the existing handoff used to expose
   them. Each of the four actual reviewer workspaces must read and hash the
   indexed files; a source-workspace existence check is insufficient. Keep
   evidence available through review completion. If the existing handoff
   cannot expose it, leave this requirement outstanding and report the
   boundary; do not introduce an evidence subsystem or claim closure.
6. **R6 / AC7–AC8:** After restoration, fresh focused and helper-suite tests
   select nonzero tests under normal concurrency and pass. Every house
   gate, fail-closed native-tls proof and pinned outer offline verify pass
   on the resulting candidate before the policy-selected reviews. Keep
   CI-only obligations explicit and preserve the no-fallback provider
   assignments. No push, merge, tag or publication.

## Completion boundary

This residual design specifies evidence recovery/regeneration and fresh
verification around the completed implementation. Neither mutant execution,
implementation gates nor independent review is claimed by this chief. The sequence's pinned deterministic check alone rules this design
attempt; subsequent seats must discharge the acceptance criteria on the
resulting candidate.
