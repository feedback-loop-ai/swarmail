# Tasks — retain failed-cleanup drains

Implementation tasks remain unchecked at design handoff. Each task carries its
proof; no implementation or regression result is implied by this design commit.

- [ ] T1 — Add scoped holds and non-owning observations to the existing runner
  probe family; implement `cleanup_drain_timeout_retains_helper_stdout` and
  `cleanup_drain_timeout_retains_cargo_stdout` through the actual spawn/output
  branches. Test: each compiles against d973b0e production logic, reaches the real
  bounded timeout after reaping/group/stderr cessation, and fails specifically at
  admission/private-lifetime retention with the runtime still alive (AC2).
- [ ] T2 — Implement `cleanup_drain_timeout_retains_unjoined_suffix`, retaining no
  masking lifetime references. Test: first pending join expires; after it finishes,
  the later pending task must still hold admission/private state. Record baseline
  failure at that invariant and final release after both finish (AC3).
- [ ] T3 — Restore timed-out and all unjoined handles to the existing cleanup owner
  before returning error. Test: T1/T2 pass, cached error remains identical after
  completion, admission becomes available and private state disappears only after
  actual task completion/owner disposal; no joined handle is polled twice (AC1–3/5).
- [ ] T4 — Extend the failed-observation/forced-expiry probe to survive its failure
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
- [ ] T6 — Record exact red/green test names, commands, matched counts, observed
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
