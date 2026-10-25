# Tasks — retain failed-cleanup drains

Implementation progress is recorded below. Gate completion is recorded separately
from the causal regression and negative-control evidence.

- [x] T1 — Add scoped holds and non-owning observations to the existing runner
  probe family; implement `cleanup_drain_timeout_retains_helper_stdout` and
  `cleanup_drain_timeout_retains_cargo_stdout` through the actual spawn/output
  branches. Test: each compiles against d973b0e production logic, reaches the real
  bounded timeout after reaping/group/stderr cessation, and fails specifically at
  admission/private-lifetime retention with the runtime still alive (AC2).
- [x] T2 — Implement `cleanup_drain_timeout_retains_unjoined_suffix`, retaining no
  masking lifetime references. Test: first pending join expires; after it finishes,
  the later pending task must still hold admission/private state. Record baseline
  failure at that invariant and final release after both finish (AC3).
- [x] T3 — Restore timed-out and all unjoined handles to the existing cleanup owner
  before returning error. Test: T1/T2 pass, cached error remains identical after
  completion, admission becomes available and private state disappears only after
  actual task completion/owner disposal; no joined handle is polled twice (AC1–3/5).
- [x] T4 — Extend the failed-observation/forced-expiry probe to survive its failure
  verdict and drop unrelated owners. Test: both cases of
  `cleanup_observation_error_and_forced_expiry_fail_with_live_owned_processes`
  assert exact refusal/private-state retention while work lives, then safely clean
  up and prove release. A temporary early-release control fails that refusal
  assertion in each mode; restore it before final verification (AC4).
- [ ] T5 — Prove compatibility and resolve the same-path INFO. Test: matched
  `runner_cleanup_await_pending_and_bypass_controls_cover_signals_and_faults`,
  `cleanup_handshake_`, `sequential_admission_library_and_cleanup_lifetime` and
  `unproved_worker_cleanup_stops_matrix_with_full_refusal_suffix` pass, preserving
  the existing signal, cached-error, finalization and full-suffix assertions (AC5).
- [x] T6 — Record exact red/green test names, commands, matched counts, observed
  assertions and restored-control diff checks in this artifact; commit authorized
  implementation/tests unsigned. Test: diff audit shows only the scoped ownership
  repair/regression support and design progress, no temporary bypasses, altered
  decisions (especially proposed 0076), dependencies, gates or routing (AC6).
- [ ] T7 — Verify the committed implementation candidate with every local house
  gate, exact coverage/selftest, publish/seal proofs, pinned engine compile and
  script integrity, design/delivery proofs and fail-closed native-tls absence.
  Test: fresh command exits and literal zero missed lines/branches/functions bind
  to the candidate HEAD/tree; failures are recorded, not relabeled green (AC6).
- [ ] T8 — Run standalone eval fmt, locked all-target clippy and full workspace
  locked tests with default threading, including the new focused tests and all
  real-protocol controls. Test: complete suite and matched focused filters pass;
  preserve privacy/provenance/trap/sequential results. Record candidate identity
  and local evidence without claiming CI/operator-owned work (AC5/6).

## Evidence to record during implementation

For T1/T2: baseline production revision, test-only patch identity, exact command,
matched case, intended timeout/ordering acknowledgments, failure assertion and
fixed-candidate success. For T4: each temporary early-release mutation, each
failure assertion and proof of restoration. For T7/T8: HEAD, tree, commands,
exit statuses and receipt paths. An implementation gate failure on an unrelated
review residual must name its scope conflict; it cannot complete the task.

The chief commits only spec.md, plan.md and tasks.md in this directory, leaves
run-local positions uncommitted, then checks HEAD with the pinned design validator.
Implementation, independent verify and protected review remain later-phase work;
the engine owns phase selection and the operator owns sealing and acceptance.

## Implementation evidence — 2026-10-25

The only production behavior change restores the timed-out drain followed by its
unvisited iterator suffix to `OwnedGroup::drains` before returning the existing
error. The original failure owner then joins them. Absolute budgets, JoinError
treatment, cached verdict and public categories are unchanged. All added holds,
progress events and weak process-group observations are Linux `cfg(test)` only,
scoped to a spawn or the existing isolated probe. No seam captures admission,
private runtime or a process owner. The failed-cleanup probes redirect only the
surviving descendant's output, so ordinary drains cannot mask missing retention.
No decision, dependency, routing, root production or gate file changes.

Receipts live under `.forge/epic192/issue199/ac3-evidence/` (run-local, ignored).
All commands below use `cargo test --manifest-path evals/Cargo.toml --locked`.

- Baseline is the unchanged production cleanup from d973b0e / design HEAD
  79a1726. The final test-only patch is `final-baseline-test-support.patch`,
  SHA-256 `43c657bf6c6c4cd2a2f9a643ab58317c03357337b5b91160853e99f49a4bceb1`.
  `--lib cleanup_drain_timeout_retains_` exits 101: exactly three matched tests,
  all fail at “pending stdout must retain admission after the failed cleanup
  owner progresses (owner-disposed)” (`drains-final-baseline.log`). Each uses
  real child pipes, successful reaping, empty membership, completed stderr and
  the actual ten-second bounded-drain timeout. No setup/compile failure counts
  as the red proof.
- With the repair, `cleanup_drain_timeout_retains_` exits 0, all three tests pass
  (`drains-green.log`). Cases are `cleanup_drain_timeout_retains_helper_stdout`,
  `cleanup_drain_timeout_retains_cargo_stdout` and
  `cleanup_drain_timeout_retains_unjoined_suffix`. The stdout controls traverse
  the real capture/Cargo consumers. Non-owning weak/watch/task observers prove
  admission and private-state retention with only the failure owner remaining,
  then actual task/owner completion, directory disposal, later admission and
  unchanged cached error. The suffix case releases stdout first and observes
  the later task still pending before making the same assertions again.
- A separate suffix-only mutation changes `retained.extend(drains)` to
  `drop(drains)`. `--lib cleanup_drain_timeout_retains_unjoined_suffix` exits 101
  at “the unvisited suffix must retain admission after stdout finishes”
  (`suffix-early-release.log`). This isolates suffix loss from current-handle
  loss. The mutation was restored.
- `cleanup_observation_error_and_forced_expiry_fail_with_live_owned_processes`
  passes both modes with the runtime retained after the returned row
  (`failed-owners-green.log`). The probe waits until only background failure
  owners remain and ordinary drains/reapers have finished, then the parent
  asserts exact contention and private-state presence. Identity-aware teardown
  stops surviving work; admission and private-state disposal are checked before
  the runtime is released.
- Two separate negative controls skip the failed-owner retention only for the
  selected injected fault. The same `--lib` test exits 101 at “failed cleanup
  must retain admission with live owned work: observation_error”, then at the
  corresponding `forced_expiry` assertion in the second run
  (`observation_error-early-release.log`, `forced_expiry-early-release.log`).
  The first mode passes normally before the second mode's negative assertion.
  Both controls were restored in a `finally` block and exact candidate bytes
  checked: runner SHA-256
  `b9ebb820dec6242e5d4525a98fff12f5676014cc529260621033ab2d10514cb1`.
- Standalone locked all-target clippy passes after restoration
  (`eval-clippy-initial.log`). Fresh committed-candidate full gates and eval
  suite remain T5/T7/T8; historical port receipts are not substituted.
