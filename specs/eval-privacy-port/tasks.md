# Ordered tasks — eval privacy port

These are implementation/proof obligations, not completed receipts. Follow
[spec.md](spec.md), the named mapping in [plan.md](plan.md), and proposed
[0075](../../docs/decisions/0075-eval-privacy.md). Accept nothing.

- [ ] **T1 — Reconcile decision index and owning contract (AC9).** Add exactly
  one proposed 0075 index row, replace current inheritance/tail/path wording and
  stale privacy links in evals README and owning eval spec/plan; preserve dated
  history, 0073 and accepted 0058. **Test:** cargo test --locked --test
  store_contract_docs, especially every_decision_file_has_exactly_one_resolving_index_row
  and every_index_row_matches_its_records_status; check relative links and exact
  role/preparation maps. No old 0062 live declaration or privacy acceptance.
- [ ] **T2 — Bind both suites and adapt probes (AC26).** Reconcile every plan
  inventory row against baseline and packet; retain assertions and helpers,
  move former production-fault cases to compiled-only inline owners. **Test:**
  baseline/final test-list comparison plus executed helper markers, actual
  whole-environment observations before wrapper additions, and retained #194
  guard-failure/rescue tests. No unexplained missing name/assertion.
- [ ] **T3 — Close runtime maps and preserve native inputs (AC1–2,16,24–25).**
  Implement native discovery/CLI/worker handoff and exact role maps, generated
  PWD and private directories retained at cleanup fences. **Test:** all named
  env/lookup/literal/native/wrapper tests, normal dev/release control inertness,
  trial_runtime_owner_survives_aborted_body, and overlapping isolation.
- [ ] **T4 — Close preparation within existing Cargo ownership (AC3,13,15,18,24).**
  Add lazy named tool/cache selection and bounded typed Cargo records while
  preserving once-only offline/locked build and signal/reap/EOF paths. **Test:**
  automatic_build_uses_a_cleared_named_preparation_map,
  automatic_build_accepts_explicit_homes_without_caller_home,
  oversized_cargo_record_is_drained_and_fails_without_echo,
  cargo_record_reader_bounds_lines_and_keeps_draining_after_overflow,
  matrix_signal_cancels_and_reaps_the_fallback_cargo_group, and
  cargo_runtime_owner_survives_cancelled_preparation. Record actual Linux and
  macOS toolchain/custom-target proof separately; missing platform stays open.
- [ ] **T5 — Remove raw retention and bound readiness (AC4–5,17,24–25).**
  Replace Log/unbounded strings with owned discard drains and first typed
  announcement; preserve independent waiters/EOF/signal responsiveness. **Test:**
  both readiness flood tests, noisy agent counted completion, portable canary
  trial with snapshot/Fail/GaveUp/status 0/exit 1, distinct Linux server-death
  test, and #194 stdout backpressure/inherited-pipe/fence tests.
- [ ] **T6 — Close every publishing sink and direct HTTP (AC4,6,12,20).**
  Add no_proxy to all clients, suppress Admin/body/error text, safe CLI/panic/
  cleanup categories, null executable metadata and bounded safe diagnostics.
  Retain exact 64 KiB matching-row/exit validation, add CR/wrong-label cases.
  **Test:** named Admin non-200 and transport/read/parse tests, proxy oracle
  success, unsafe Clap/path tests, both old/new matrix parser tests, CLI
  stdout/stderr and entire artifact/temp-tree scans; cleanup failure remains
  distinct from success and real statuses remain unchanged.
- [ ] **T7 — Port exclusive private publication (AC21–23).** Add restrictive
  exclusive leaf/temp creation, descriptor normalization before payload,
  complete-write persist_noclobber and owned-name cleanup in the current writer.
  **Test:** all six named artifact helper tests, real adverse-umask failed-trial
  artifacts, safe artifact_write_failures_use_safe_categories, sentinel bytes/
  modes and final visibility assertions. Execute selected primitives on Linux
  and actual macOS separately; final-only chmod checks are insufficient.
- [ ] **T8 — Prove combined privacy/cancellation preservation (AC6–7,24–26).**
  Execute every retained #194 inline/wire race, both signals, repeated signals,
  Cargo/bootstrap/spawn failures, active cleanup hold/release, expiry/observation,
  leader-first/resistant descendant, EOF/output and checked guard cases.
  **Test:** privacy_cancellation_preserves_owned_runtime_and_suffix_accounting
  plus full named baseline inventory; byte-identical prefix, one post-cleanup
  active error, fresh zero-time suffix IDs, no later launch, correct report
  denominators and actual statuses; full real oracle/null/wrong-action/isolation.
- [ ] **T9 — Execute separate falsifiers and restore (AC8,19).** Run plan's
  inheritance, raw-output, metadata/error, Cargo-overflow, native-worker and
  pre-payload/mode mutations and both existing cleanup-await bypass controls.
  **Test:** each exact focused command fails its intended assertion after
  acknowledged execution, then passes restored; record candidate/tree, patch
  region, command, OS, counts and exit. No mutation remains in final source.
- [ ] **T10 — Execute all final-candidate gates (AC10–13).** Verify pinned
  .forge/bin/brokkr and tooling/shim identities; run every house gate, exact
  zero-missed coverage and selftest, design/delivery proofs, publish/seal proofs,
  licenses, fail-closed native-tls absence, and independent eval fmt/all-target
  locked clippy/full tests. **Test:** actual candidate-bound command/exit receipts
  for every plan command. Root green is not eval proof; missing macOS and CI-only
  evidence are named separately and never marked complete.
- [ ] **T11 — Obtain five fresh careful reviews and resolve returns (AC14).**
  Use the current bundles/codex-careful route for Astra correctness, official
  Flash security, Sonnet 5.5 tests, Astra guarantees and independent Fable 5.1.
  **Test:** five separate candidate-bound engine results, protected review and
  security hold honored; rerun affected proofs/final gates after changes. No
  substituted/combined reviewer, fabricated receipt, decision acceptance,
  push/merge/tag/publication or issue closure.
