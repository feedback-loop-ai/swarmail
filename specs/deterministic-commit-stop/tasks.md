# Tasks — deterministic commit at stop

Ordered implementation obligations for [spec.md](spec.md). Check a task only
after recording its paired proof; this design has not executed those proofs.

- [ ] **T1 — Consolidate the existing per-store gate after shutdown admission**
  in `Store::record`; remove the clear-specific duplicate calls and correct
  the seam comment. **Proof:** existing persistence gate ordering and both
  panic-symmetry tests pass; inspect that all seam code remains cfg(test),
  with one invocation after the stopping check and before the mirror (AC1, AC8).
- [ ] **T2 — Move and repair the named lifecycle regression** in the existing
  persistence harness, with local bounded real SMTP plumbing, entered/release
  handshake, release-on-unwind guard, subscribe-before-send, stopped/empty
  assertions and exact post-release mail/counter comparisons. Remove only its
  old integration copy and correct any direct comment pointer. **Proof:** the
  exact full test name runs once and passes AC1–AC5; the persistence harness
  and remaining lifecycle integration suite pass without serialization.
- [ ] **T3 — Prove stop cannot join the held mutation.** Temporarily make
  shutdown join the live SMTP session instead of aborting it. **Proof:** the
  repaired test fails at its named stop deadline while the gate is held;
  cleanup exits the process, then the restored tree passes (AC2, AC5, AC6).
- [ ] **T4 — Prove admitted work must complete after release.** Temporarily
  discard/refuse the admitted mutation after gate release before SQLite write.
  **Proof:** the repaired test fails at its named completion/required-row
  assertion, not compilation or teardown; restore and pass (AC4–AC6).
- [ ] **T5 — Prove no half-landing or premature acceptance.** Run two separate
  mutants: expose memory/counters before the gated mirror call; acknowledge
  DATA before awaiting the commit. **Proof:** the first fails early visibility
  and the second fails no-DATA-250, with the intended diagnostics and clean
  unwind; restore and pass after each (AC2, AC3, AC5, AC6).
- [ ] **T6 — Demonstrate stability under host CPU load.** Start tracked busy
  workers covering every available CPU and record 20 consecutive exact-test
  passes with no selected retries. Keep load active for one full instrumented
  coverage gate. **Proof:** transcripts identify source revision, core/worker
  count, commands and every verdict; coverage has zero missed lines/branches/
  functions, and all workers/fixtures are cleaned up on exit (AC5, AC7).
- [ ] **T7 — Verify the restored final implementation.** Run the plan's full
  house inventory, the existing five-second busy-bound/default-override pins,
  persistence refusal/ordering tests and queued-stop regressions. **Proof:**
  green final-source receipts, warning-free recorded pinned compile in verify,
  fail-closed native-tls absence, both script proofs and exact coverage. State
  every unrun obligation honestly; no unrelated repairs without a concrete
  verify/review finding (AC8).
- [ ] **T8 — Append signed implementation history and hand off.** Record the
  named #143 test and each red/loaded proof in the commit body and ignored
  evidence; retain the other batch fixes. **Proof:** `git verify-commit` passes,
  the diff matches the plan, branch is `epic120-delivery-constitution`, tracked
  status is clean, and no mutant/load worker remains. No push, PR, merge,
  squash or issue closure (AC9).
