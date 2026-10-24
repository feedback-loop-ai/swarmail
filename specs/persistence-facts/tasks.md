# Ordered implementation tasks — persistence facts

These are future implementation/verification tasks. Unchecked means unproved,
not permission to omit a gate. Keep run-local evidence out of commits.

- [ ] T1 — Refresh the baseline, designated ownership record and relevant
  decisions; verify the candidate-local engine digest. Preserve triage's
  engine/four-reviewer route. **Proof:** compare HEAD/local origin/main,
  inspect journal or designated snapshot/worktree reservations for the six
  paths, and match SHA-256 to tooling.json; park on concrete missing input
  or overlap. Record visibility limits without claiming fresh remote data.
- [ ] T2 — Append the dated 0014/0016 corrections for capped eviction and
  completed-state agreement, with cap=1 and reader/poison limits (AC1–AC2).
  **Proof:** byte-prefix checks against implementation base, source-order
  audit of insert/record_insert and existing `cap_pruning_is_mirrored_to_the_data_file`,
  `record_insert_deletes_the_pruned_rows`, and
  `pruning_follows_acceptance_not_received_ms` tests.
- [ ] T3 — Complete 0014/0015/0016 timing addenda and 0014's missing-number
  correction; identify inherited 0012 §Why from 0015 without editing 0012
  (AC1, AC3–AC4). **Proof:** prefix/link checks and claim audit;
  `the_opened_connection_keeps_the_dependency_busy_default` and
  `the_busy_default_tripwire_fires_when_the_bound_is_overridden` both pass.
  Audit explicitly rejects interpreting those tests as latency measurements.
- [ ] T4 — Append 0024's dated watch and shutdown-timing corrections; change
  only Store's stopping-field and record timing comments (AC1, AC3, AC5).
  **Proof:** prefix check and contextual comment-only patch review;
  `a_wired_stopping_store_refuses_the_commit_and_applies_nothing`, lifecycle
  `queued_seeds_refuse_once_the_stop_has_fired` and binary
  `sigint_under_queued_seeds_exits_clean_and_stores_nothing` pass unchanged.
- [ ] T5 — Correct the two persistence test introductions to their actual
  zero-trace outcomes; distinguish SMTP recovery from REST's absence proof
  (AC6). **Proof:** comments match adjacent unchanged assertions;
  `cargo test --locked --test persistence` passes, including both named
  blocked-commit tests and `a_refused_insert_is_never_exposed_as_accepted_mail`.
- [ ] T6 — Audit the final correction as one bounded patch (AC1–AC7).
  **Proof:** all four original ADRs are byte prefixes, all five findings map
  to explicit corrections, relative links resolve, only the six authorized
  implementation paths change, and reversing comment-only hunks reproduces
  original Rust bytes; `a_failing_data_file_refuses_with_zero_trace` passes.
  Reject new executable tokens, assertions, configuration or ruling changes.
- [ ] T7 — Complete every [house-rule gate](../../docs/house-rules.md#gates--part-of-every-change)
  through the pinned verification chain, preserving `--locked`, exact
  coverage, coverage selftest, publish-guard and seal proofs, license checks
  and fail-closed native-tls absence (AC8). **Proof:** command/exit receipts
  tied to the implementation candidate; retain CI-owned obligations and any
  environmental failures as outstanding. A successful design check is not a
  substitute, and no test floor or dependency allowance may be weakened.
- [ ] T8 — Commit the scoped implementation unsigned and obtain protected
  independent correctness/security/guarantees/tests review through the
  selected engine route (AC9). **Proof:** exact commit path audit plus
  engine verification and four review receipts; official DeepSeek Flash
  and DashScope GLM-5.3 retain their assigned lenses. No push, tag, publish,
  issue closure or epic-completion claim. Root owns integration after CI.
