# #179 follow-up: prove bridge reaping through its owning handle

Created: 2026-10-23. Status: draft for implementation.
Story: epic #192, #179 correctness-review residual only.
Baseline: reviewed #179 source replay 86c851f on current main df8630f.

## Feature and rationale

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

## Panel synthesis and rulings

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

Implementation may change only `tests/binary/helper_reliability.rs` and
these directly necessary #179 design notes. The design commit itself
contains exactly this directory's spec.md, plan.md and tasks.md. Reviewed
helpers in `tests/binary.rs`, common/protocol tests, production,
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

## Completion boundary

This design specifies future implementation and proof. Neither mutant
execution, implementation gates nor independent review is claimed by the
chief. The sequence's pinned deterministic check alone rules this design
attempt; subsequent seats must discharge the acceptance criteria on the
resulting candidate.
