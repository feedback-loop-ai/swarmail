# Ordered implementation tasks — sequential eval trial port

Date: 2026-10-25. Each item names its proving test. These are implementation
obligations, not historical completion receipts. Follow [plan.md](plan.md) and
[spec.md](spec.md); preserve current main throughout the shipped-patch port.

- [ ] **T1 — Establish compatible contract integration (AC7).** Add exactly one
  proposed 0076 entry to docs/decisions/README.md, preserving 0077 and every
  existing status. Append dated scoped 0055 integration preserving accepted
  bodies and #198/#200 history; explain the already-v2 base and operator ruling.
  **Proof:** root store_contract_docs decision inventory/status/link tests and
  eval boundary document test check unique resolving entry and current statuses.
- [ ] **T2 — Restore shared admission and request validation (AC1–2).** Add the
  narrow admission module/lib registration and concurrency request/default/CLI
  parsing through current constructors. Preserve assigned IDs and refused-row
  provenance; no trial work before admission or unsupported-request refusal.
  **Proof:** sequential_admission_rejects_unsafe_rendezvous,
  sequential_admission_refuses_independent_cli_trials and
  sequential_trial_concurrency_requests cover CLI/library and changed caller
  contexts, unsafe path fixtures, named diagnostics, row count and exit 2.
- [ ] **T3 — Bind admission to existing lifecycle (AC3).** Retain admitted task
  and current group/helper/reaper/drain/private-state owners through target
  recheck/finalize/disposal; preserve fixed budgets and unproved cleanup holds.
  Repair the inherited missing std::io::Write import in runner test_probes
  revealed by design-stage eval compilation; keep all #225 assertions.
  **Proof:** sequential_admission_library_and_cleanup_lifetime plus current
  runner_cleanup_await_pending_and_bypass_controls_cover_signals_and_faults and
  cleanup_handshake_ controls cover dropped waiter, TERM/INT, normal/error,
  source helpers, actual Pending, finalization/disposal and later reacquisition.
  Temporary early release must fail the held-reservation assertion; restore it.
- [ ] **T4 — Compose strict records and matrix accounting (AC2, AC5).** Add
  required concurrency 1 to all current rows/fixtures, using shared validation;
  retain typed provenance, current v2 semantics and artifact envelope versions.
  Integrate bounded finite worker diagnostics into existing stop_admission while
  keeping context/identity/retained checks, product order and suffix accounting.
  **Proof:** sequential_rows_record_concurrency,
  sequential_rows_reject_missing_or_invalid_concurrency,
  sequential_matrix_refuses_overlap_and_unsupported_concurrency and
  unproved_worker_cleanup_stops_matrix_with_full_refusal_suffix; extend current
  worker matching/context and provenance retained-check tests for missing,
  noninteger, 0/>1 concurrency, valid-row-plus-cleanup failure and all error rows.
  Actual replay/check must retain current trap outcomes and byte/row equality.
- [ ] **T5 — Restore causal sequential controls (AC4, AC6).** Add shared outer
  test-scope coordination across all trial-producing suites/inline probes;
  sequentialize per_task and replace cooperative overlapping-trial proof.
  Reuse RuntimeDirs with two real writable-HOME/sentinel helpers. Restore inert
  victim/contender/no-marker/release control; preserve within-trial agents.
  **Proof:** sequential_poacher_control_refuses_competing_trial and fresh-HOME
  assertions in runner's sequential trials prove failure then later oracle
  success. Test-only admission bypass fails exactly the refusal assertion and
  is restored. Full default-thread eval suite preserves all sixteen oracle/null,
  inbox/foreign-token, completed naive-command traps, recovery and privacy tests.
- [ ] **T6 — Align all active contracts (AC7).** Update evals/README.md, restore
  linked evals/run-plan.md for local #151, and amend original harness spec/plan/
  tasks with explicit local issue-#148 AC4 replacement. Preserve positive-control
  AC4, accepted history and main addenda; state exact same-uid false-pass case,
  fresh HOME, concurrency meaning, sequential baseline and hard-death limits.
  **Proof:** sequential_contracts_and_decision_links_are_honest rejects missing
  statements, broken links, active overlap requirements and wrong/duplicate 0076
  status/index; root documentation inventory remains green.
- [ ] **T7 — Verify and commit the candidate (AC8).** Commit all authorized
  implementation unsigned before final verification. Run every house gate with
  verified candidate-local engine, exact coverage/selftest, publish/seal proofs,
  fail-closed native-tls absence, design/delivery proof selftests and standalone
  eval fmt/clippy/full locked default-thread tests per plan. **Proof:** fresh
  candidate-bound command receipts with exit statuses, exact zero missed counts,
  restored negative-control seams and clean tracked diff; no-match test filters
  are refused. Preserve current route for engine-owned verify/protected review.
  Record CI/operator-owned checks separately; never fabricate their receipts.

The chief design commit contains only these three artifacts and proposed 0076.
T1's index gap is an expected design-stage inventory failure, not permission to
report implementation or full-gate success. No task accepts a decision, changes
routing, pushes, merges, tags, publishes, closes issues or implements #221.
