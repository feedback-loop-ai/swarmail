# Ordered tasks — eval measurement repairs

Implementation handoff for [spec.md](spec.md), following [plan.md](plan.md).
Every unchecked item is required work; this design does not claim it ran.

- [ ] **T1 — Pin the regression cases before implementation (AC1–AC6).**
  Add focused tests in existing scoring, fixture, runner and report suites
  plus the existing test helper as needed. **Proof:** tests compile against
  `4dac267`; isolated baseline runs fail on Failed/Parked scoring, final
  normal/deadline classification, replay exit and the missing-binary eight-row
  assertion. Save exact test names, commands and red logs as run-local evidence.
- [ ] **T2 — Restore shared snapshot eligibility (AC1, AC2, AC4).**
  Add final Failed/Parked rejection to `tasks::validate_snapshot`, retain its
  use by `score`, broaden comments/diagnostics and preserve `check`'s Err-to-2
  path. **Proof:** pure idle/satisfied/decoy tests and real post-provision
  transport-failure snapshot replay pass; Held and Delivered-after-451 controls
  remain ordinary; malformed snapshots still refuse.
- [ ] **T3 — Put final eligibility before budget (AC3, AC4).**
  Reorder runner final selection without removing early preparation refusal
  or changing process cleanup and saved statuses. **Proof:** real post-launch
  chaos at 100 produces a retained Parked/data/451 receipt; both normal-exit
  and deadline trials emit harness_error/exit 2 and replay agrees. Existing
  ordinary budget, success-after-nonzero-exit and lifecycle tests pass.
- [ ] **T4 — Preserve the matrix product on binary failure (AC5).**
  Keep once-only resolution, emit existing refused rows in the one product
  loop, log the shared error once and launch no child on that path. **Proof:**
  missing subprocess-local SWARMAIL_BIN produces the ordered 2×2×2 product,
  distinct IDs, expected labels and null/empty statuses, exit 2; invalid-input
  refusals and existing successful/mixed matrix tests remain green.
- [ ] **T5 — Clarify documentation and fix formatting (AC7, AC8).**
  Update evals README precedence/replay/accounting text and run
  `cargo fmt --all` from evals. **Proof:** evals fmt check passes; review the
  report assertion diff for unchanged meaning and check README against R1–R4.
- [ ] **T6 — Prove the repaired standalone crate (AC1–AC7).**
  Run evals fmt, clippy and full locked tests as listed in the plan.
  **Proof:** green gate exits, targeted red-to-green regression logs, all
  16 oracle tasks passing, all 16 null tasks failing in their declared classes,
  concurrent isolation and existing boundary/dependency tests passing.
- [ ] **T7 — Prove unchanged root guarantees and scope (AC7, AC8).**
  Run the complete root gate inventory in the plan, including exact coverage,
  coverage selftest, pinned bundle compile, publish guard, licenses and existing
  fail-closed native-tls proof. **Proof:** saved green exits, exact coverage
  missed counts zero, and diff/lockfile inspection shows no forbidden root or
  dependency edits. Record a blocker rather than asserting unavailable proof.
- [ ] **T8 — Land the repair without rewriting history (AC8).**
  Append signed implementation commit(s) on epic164-harness with plain subjects
  citing #148; never push or perform GitHub actions. **Proof:** signature
  verification, ancestry retaining fd66988 and 4dac267, intended-path-only
  commit inspection and clean tracked tree on the same branch. Signing failure
  blocks landing; keep all run-local positions, results and logs uncommitted.
