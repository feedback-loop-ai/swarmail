# Ordered tasks: the eval recipe on brokkr (interim path)

Each task is checked only with its paired test. Nothing is `complete` while a
box is open or a gate is red. No push, no merge, no decision accepted;
commits unsigned. `evals/` is outside the root coverage denominator
(accepted 0055), so every new branch here must be executed by a named test.
Where a step needs the pinned brokkr and the seat has none, record "not run,
no binary" in `implementation.md`; never record a pass.

- [ ] **T1 Baseline and brokkr reachability.** From `f815d48`, run the
  quickstart's gate list once; record exit codes and the resolved
  `SWARMAIL_BIN`. Check `.brokkr/state/bin/brokkr` against
  `tooling.json`'s `binary_sha256`; record present/absent. Read brokkr#766
  and its sub-issues (#757–#765, #467, #462, #430) if network hands allow;
  record which of F1–F7 each already tracks.
  **Test:** every gate exits 0 on the baseline; `implementation.md` holds
  the F1–F7 table with a status per fact.
- [ ] **T2 Decision index and docs skeleton.** Add the proposed 0116 row to
  `docs/decisions/README.md` (number provisional, collision note as 0108's).
  **Test:** `grep -c '0116' docs/decisions/README.md` is 1; the existing
  decision-index inventory tests pass.
- [ ] **T3 The recipe files (AC3, AC9).** Write
  `.brokkr/recipes/eval/bundle.json` and `policy.json` exactly as the spec
  states; add `eval` to `tests/delivery_recipes.rs`'s shelf list and a new
  pin test `eval_recipe_is_two_exec_seats_with_one_attempt_and_five_rules`.
  **Test:** that test passes and fails on one-fault mutants (a retry rule,
  `max_attempts: 2`, `network: true` on `score`, a model agent on `trial`).
- [ ] **T4 `trial.sh` (AC4).** Write the script per the spec: prompt
  parsing as `verify-seat.sh`, request validation (regular file, ≤ 64 KiB,
  `jq`, required keys, absolute `evals_bin`/`swarmail_bin`/`slot`), exclusive
  `row.json`, bounded `trial.stderr`, `exit`, result via `jq` to a sibling
  temporary then rename.
  **Test:** `evals/tests/recipe.rs::trial_script_runs_the_trial_and_reports_ran`
  (oracle: pass row, exit 0, `ran`, `inputs.trial_id`; null: fail row, exit
  1, `ran`) and `trial_script_refuses_malformed_requests` (no request line,
  oversize, missing `args`, relative `evals_bin`: `harness_error`, no
  `row.json`, `notes` without any ROOT path).
- [ ] **T5 `score.sh` (AC4).** Write the script: `check --artifacts-dir
  <trials_root>/<trial_id>`, `score.json`, result mapping 0/1/other.
  **Test:** `recipe.rs::score_script_maps_check_exit_codes` (oracle leaf →
  `pass`; null leaf → `fail`; truncated `state.json` → `unscorable`;
  `score.json.outcome` agrees in each case).
- [ ] **T6 Via-brokkr selectors (AC8, AC13).** Add the three
  `WrapperProfile` variants with `family()`, `name()`, model validation and
  `ToolSurface` rows (same revision and basis, the family's restrictions,
  one residual naming the driver); accept them in `--wrapper-profile` and
  `agents.json`.
  **Test:** inline `provenance.rs` tests: each new selector round-trips,
  validates only its family's models, and its surface differs from the
  direct selector only by the driver residual; `evals/tests/provenance.rs`
  existing selector tests pass unchanged.
- [ ] **T7 `--via-brokkr DRIVER` on the wrappers (AC8).** Thread
  `via_brokkr: Option<PathBuf>` through `AgentClaude|AgentCodex|AgentDsh`
  into `wrappers::run`; validate (absolute, regular, executable) before any
  private file is read; exec `DRIVER <family> -- <argv>` with the same
  environment and MCP config.
  **Test:** `evals/tests/wrappers.rs::via_brokkr_execs_the_driver_with_the_direct_argv`
  (stand-in driver records argv and env keys; argv after `--` equals the
  direct launch's recorded argv byte for byte; env keys equal), and
  `via_brokkr_refuses_an_invalid_driver_before_launch` (relative, missing,
  non-executable → `driver_invalid`, no private file opened, nothing
  spawned).
- [ ] **T8 Trials through the stand-in driver (AC8, AC10).** Add
  `wrappers_launch/standin-driver.py` (`oracle` and `idle` modes).
  **Test:** `wrappers.rs::a_trial_through_the_stand_in_driver_passes_and_declares_the_via_brokkr_surface`
  (`trial --task signup --agent smoke --wrapper-profile
  claude_2_1_310_via_brokkr -- $E agent-claude … --via-brokkr standin`:
  pass, `confinement` `unconfined`, `tool_surface` is the via-brokkr
  selector) and the `idle` mode giving `fail`/`gave_up`.
- [ ] **T9 Request writing and the brokkr worker branch (AC1, AC9).** In
  `main.rs`: `Matrix { brokkr }`; refusals `brokkr_requires_artifacts`,
  `brokkr_realm_invalid`, `deadline_exceeds_seat` (row-less exit 2); one
  function builds the `trial` argument list for both branches; in brokkr
  mode create `slots/<trial_id>/`, write `request.json`, spawn
  `BIN run --recipe eval --feature "swarmail-eval-request: <abs>"` under the
  existing envelope, drain and discard stdout.
  **Test:** `report.rs::brokkr_mode_refuses_without_artifacts_realm_or_a_nesting_deadline`
  (three refusals, no rows, exit 2) and inline
  `main.rs::direct_and_brokkr_workers_build_one_argument_list`.
- [ ] **T10 Slot reading, exit pairing and the gate cross-check (AC1, AC2).**
  After the worker exits, read `row.json` (`results::read`), `exit` and
  `score.json`; apply `matching_trial_row` with the slot's exit; require
  `score.json.outcome == row.outcome`; map failures to `trial_row_missing`
  / `score_mismatch`, set `stop_admission`, keep the suffix and precedence.
  **Test:** `report.rs::brokkr_mode_runs_the_product_through_the_recipe_scripts`
  (stand-in worker; 2 × {oracle, null} × 2 → 8 rows, 4/4, two oracle
  trials of one task pass with distinct seeds, every row `unconfined`) and
  `brokkr_mode_turns_every_worker_defect_into_a_row` (no-row, foreign-row,
  bad-exit, bad-score, SIGKILL after readiness → `harness_error` with the
  named category, admission stopped, zero-time suffix, exit 2, never
  `gave_up`).
- [ ] **T11 Controls through the recipe path (AC5).** Add the brokkr-mode
  arm to `evals/tests/proof.rs` over all 16 tasks with the stand-in worker.
  **Test:** oracle 16/16 pass; null 0/16 pass with only `gave_up` or
  `wait_timeout`; the direct arm unchanged.
- [ ] **T12 Journal facts v1 reader and join (AC6).** `evals/src/journal.rs`:
  strict types, closed reasons and outcomes, `read`, `join` (missing →
  `journal_missing`; duplicates and unmatched ids refuse; builtin rows →
  `builtin_agent`), per-group aggregates (`nearest_rank` over joined
  non-harness-error trials, `sum`, `tokens_per_success` with `no_passes` /
  `incomplete_join`, turns, tool calls by tool, invalid-argument count,
  deadline-killed count, `runs`).
  **Test:** inline tests for each refusal and each reason; `report.rs::report_joins_journal_facts_by_trial_id`
  golden over `tests/data/journal-facts-golden.json` with hand-computed
  values, including the `incomplete_join` group and the oracle group's
  `builtin_agent`; `report_without_journal_is_unchanged` (byte equality with
  the existing golden); `a_dropped_join_is_red` (a fixture whose correct
  join gives a number and whose dropped join gives `null`).
- [ ] **T13 `report --journal` and the `journal` object (AC6).** Wire
  `Report { journal }`; `report_with_journal` adds `journal` per group and
  changes nothing else.
  **Test:** T12's goldens through the CLI; `journal_unmatched_trial` and
  `journal_duplicate_trial` refuse with exit 2 and no partial output.
- [ ] **T14 Evidence writer (AC7).** `--evidence-dir` (requires
  `--journal`): per joined trial, `DIR/<trial_id>/tool-calls.json` through
  `resolve_root`, `ensure_root`, `create_private_dir`, `publish`; tool names
  bounded to 128 printable ASCII bytes or replaced by their digest; 64 KiB
  cap with whole-call truncation and `truncated: true`; `evidence` index in
  the report; `absent: evidence_write_failed` on a publication failure.
  **Test:** `report.rs::evidence_records_carry_no_argument_or_result_bytes`
  (sentinel fixture: the admin secret and a planted token in argument and
  result positions; `grep -r` over DIR and the report finds neither; modes
  0700/0600), `evidence_truncates_at_a_call_boundary` (5 000 calls → valid
  JSON under 64 KiB, `truncated: true`), `evidence_never_clobbers`
  (pre-existing target untouched, `absent` recorded, report completes),
  `evidence_requires_journal`.
- [ ] **T15 `journal-facts.sh` (F3, AC12).** Write
  `evals/scripts/journal-facts.sh BROKKR DB OUT TRIAL_ID...` producing facts
  v1 from the pinned brokkr's export, with `source.query` naming the exact
  command; when the export carries no accounting, emit
  `journal_has_no_accounting` for every trial.
  **Test:** `recipe.rs::journal_facts_script_output_reads_as_facts_v1` using
  a captured export when the binary was reachable in T1, else a recorded
  stand-in export named as such in `implementation.md`; the output passes
  `journal::read`; a trial id absent from the export becomes
  `journal_missing`.
- [ ] **T16 CI and docs (AC3, AC15).** Append `eval` to the compile loop in
  `.github/workflows/ci.yml`; write the `eval` section of
  `.brokkr/recipes/README.md`; update `evals/README.md` (brokkr mode, facts,
  evidence, admission posture per F6, the #151 "Not measured" closure
  sentence) and `evals/wrappers.md` (`--via-brokkr`, three selectors).
  **Test:** the CI loop line lists all seven recipes in order; `git grep -n
  via_brokkr evals/wrappers.md evals/README.md .brokkr/recipes/README.md`
  is nonempty; no edit under `docs/evals/baseline-2026-11-01*`.
- [ ] **T17 Brokkr facts filed (AC12).** For each of F1–F7 not observed as
  present and not already tracked under brokkr#766, file an issue on
  feedback-loop-ai/brokkr (network hands permitting; otherwise draft the
  issue text in `implementation.md` for the operator to file) and record
  the number.
  **Test:** `implementation.md`'s F-table has, per fact, exactly one of:
  observed (with the command and output), tracked (sub-issue number), filed
  (issue number or drafted text).
- [ ] **T18 Gates (AC13, AC14).** Run the full quickstart list `--locked`;
  `brokkr compile --bundle .brokkr/recipes/eval` where the binary exists.
  **Test:** every command's exit code recorded; the `git diff --stat` over
  the lockfiles, `deny.toml`, `scripts/` and `.brokkr/recipes/self/` is
  empty; `TrialResult`, `Metadata`, `MatrixPlan`, `MatrixIndex` show no
  changed field in `git diff main -- evals/src`.
- [ ] **T19 Delivery paper (AC15).** Commit unsigned in the repository's
  style. The PR description states: held for the operator's approval by
  name (touches `.brokkr/` and `.github/`, 0056's 2026-11-01 list); 0116 is
  proposed, not accepted, and value-bearing evidence waits on it; the
  brokkr issues filed or drafted; the first engine run (oracle, offline)
  and the first authenticated via-brokkr smoke per family are the
  operator's; AC16 is the operator's paid run; AC11's reading (sequential,
  isolation is #221's); the decision number is provisional.
  **Test:** `git log -1 --format=%G?` is `N`; `git status --porcelain` is
  empty; no remote was pushed.
