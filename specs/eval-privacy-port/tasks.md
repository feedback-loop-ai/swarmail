# Ordered tasks — eval privacy port

These are ordered continuation/proof obligations. Implementation proof for
T1–T10 is recorded in [evidence.md](evidence.md) and its candidate-bound local
receipts; independent verify and T11 review results remain required.
Implementation already exists in unreviewed handoff 529d916. Unchecked means
verification is due; do not reimplement working code. Repair only a demonstrated
failed criterion/test, then rerun affected proofs and final gates. Follow
[spec.md](spec.md), the named mapping in [plan.md](plan.md), and proposed
[0075](../../docs/decisions/0075-eval-privacy.md). Accept nothing. The review phase
alone supplies T11's verdicts.

Dated clarification, 2026-10-24: spec AC13 cites the operator's #201 ruling.
Every Linux obligation below remains; macOS eval execution belongs to #201 and
is not a #196 completion prerequisite. Preserve historical blocked receipts as
history, not passes. This chief commit changes exactly the three design files
and proposed 0075; dependent current documentation is reconciled in T1.

- [x] **T1 — Reconcile owning contract and validate the existing index (AC9,13).**
  Keep exactly one already-present proposed 0075 index row. Align the current
  platform sentences in evals/README.md and specs/agent-eval-harness/{spec,plan}.md
  with AC13's dated #201 ownership; preserve every Linux proof and dated history.
  Audit exact maps/output/links, repairing only stale current wording; keep 0073
  and 0075 proposed and preserve accepted 0058. **Test:** cargo test --locked --test
  store_contract_docs, especially every_decision_file_has_exactly_one_resolving_index_row
  and every_index_row_matches_its_records_status; check relative links and exact
  role/preparation maps. No old 0062 live declaration or privacy acceptance.
- [x] **T2 — Bind both suites and audit existing probes (AC26).** Reconcile every plan
  inventory row against baseline and packet; retain assertions and helpers,
  verify former production-fault cases use compiled-only inline owners. **Test:**
  baseline/final test-list comparison plus executed helper markers, actual
  whole-environment observations before wrapper additions, and retained #194
  guard-failure/rescue tests. No unexplained missing name/assertion.
- [x] **T3 — Verify runtime maps and native inputs (AC1–2,16,24–25).**
  Verify native discovery/CLI/worker handoff and exact role maps, generated
  PWD and private directories retained at cleanup fences. **Test:** all named
  env/lookup/literal/native/wrapper tests, normal dev/release control inertness,
  trial_runtime_owner_survives_aborted_body, and overlapping isolation.
- [x] **T4 — Verify preparation within existing Cargo ownership (AC3,13,15,18,24).**
  Verify lazy named tool/cache selection and bounded typed Cargo records while
  preserving once-only offline/locked build and signal/reap/EOF paths. **Test:**
  automatic_build_uses_a_cleared_named_preparation_map,
  automatic_build_accepts_explicit_homes_without_caller_home,
  oversized_cargo_record_is_drained_and_fails_without_echo,
  cargo_record_reader_bounds_lines_and_keeps_draining_after_overflow,
  matrix_signal_cancels_and_reaps_the_fallback_cargo_group, and
  cargo_runtime_owner_survives_cancelled_preparation. Record actual Linux
  toolchain/custom-target proof. #201 owns macOS execution under AC13; record
  that ownership without asserting a macOS pass or blocking #196 on it.
- [x] **T5 — Verify omission and bounded readiness (AC4–5,17,24–25).**
  Verify owned discard drains and first typed announcement retain no raw tails
  or unbounded strings; preserve independent waiters/EOF/signal responsiveness.
  **Test:**
  both readiness flood tests, noisy agent counted completion, portable canary
  trial with snapshot/Fail/GaveUp/status 0/exit 1, distinct Linux server-death
  test, and #194 stdout backpressure/inherited-pipe/fence tests.
- [x] **T6 — Verify every publishing sink and direct HTTP (AC4,6,12,20).**
  Verify no_proxy on all clients, suppressed Admin/body/error text, safe CLI/panic/
  cleanup categories, null executable metadata and bounded safe diagnostics.
  Retain exact 64 KiB matching-row/exit validation and CR/wrong-label cases.
  **Test:** named Admin non-200 and transport/read/parse tests, proxy oracle
  success, unsafe Clap/path tests, both old/new matrix parser tests, CLI
  stdout/stderr and entire artifact/temp-tree scans; cleanup failure remains
  distinct from success and real statuses remain unchanged.
- [x] **T7 — Verify exclusive private publication (AC21–23).** Verify restrictive
  exclusive leaf/temp creation, descriptor normalization before payload,
  complete-write persist_noclobber and owned-name cleanup in the current writer.
  **Test:** all six named artifact helper tests, real adverse-umask failed-trial
  artifacts, safe artifact_write_failures_use_safe_categories, sentinel bytes/
  modes and final visibility assertions. Execute selected primitives on Linux;
  #201 owns macOS primitive execution under AC13. Final-only chmod checks remain
  insufficient, and neither missing artifacts nor launch failure passes.
- [x] **T8 — Prove combined privacy/cancellation preservation (AC6–7,24–26).**
  Execute every retained #194 inline/wire race, both signals, repeated signals,
  Cargo/bootstrap/spawn failures, active cleanup hold/release, expiry/observation,
  leader-first/resistant descendant, EOF/output and checked guard cases.
  **Test:** privacy_cancellation_preserves_owned_runtime_and_suffix_accounting
  plus full named baseline inventory; byte-identical prefix, one post-cleanup
  active error, fresh zero-time suffix IDs, no later launch, correct report
  denominators and actual statuses; full real oracle/null/wrong-action/isolation.
- [x] **T9 — Audit falsifier receipts, reproduce gaps and restore (AC8,19).**
  Match the eleven supplied receipts to actual source/test regions and candidate
  lineage. Retain valid intended-assertion proof; reproduce missing, stale or
  insufficient proof for plan's inheritance, raw-output, metadata/error,
  Cargo-overflow, native-worker and pre-payload/mode mutations. Execute both
  existing cleanup-await bypass controls.
  **Test:** each exact focused command fails its intended assertion after
  acknowledged execution, then passes restored; record candidate/tree, patch
  region, command, OS, counts and exit. No mutation remains in final source.
- [x] **T10 — Execute all final-candidate gates (AC10–13).** Verify pinned
  .forge/bin/brokkr and tooling/shim identities; run every house gate, exact
  zero-missed coverage and selftest, design/delivery proofs, publish/seal proofs,
  licenses, fail-closed native-tls absence, and independent eval fmt/all-target
  locked clippy/full tests. **Test:** actual candidate-bound command/exit receipts
  for every plan command. Root green is not eval proof. Record #201-owned macOS
  and charter CI-only obligations separately without claiming execution; they
  do not replace or reduce any Linux gate or block #196 on macOS receipts.
- [ ] **T11 — Obtain five fresh careful reviews and resolve returns (AC14).**
  Use the current bundles/codex-careful route for Astra correctness, official
  Flash security, Sonnet 5.5 tests, Astra guarantees and independent Fable 5.1.
  **Test:** five separate candidate-bound engine results, protected review and
  security hold honored; rerun affected proofs/final gates after changes. No
  substituted/combined reviewer, fabricated receipt, decision acceptance,
  push/merge/tag/publication or issue closure.
