# Ordered tasks: the eval recipe on brokkr (interim path) — revision 3

Each task is checked only with its paired test. Nothing is `complete` while a
box is open or a gate is red. No push, no merge, no decision accepted;
commits unsigned. `evals/` is outside the root coverage denominator
(accepted 0055), so every new branch here must be executed by a named test.
Where a step needs the pinned brokkr and the seat has none, record "not run,
no binary" in `implementation.md`; never record a pass. Criteria AC12 and
AC16 are the operator's and are reported as such, never as met.

## Round 1 (built in `080db77`; listed for the record, re-verified by T18)

T1–T19 of the first design were implemented in `080db77`. The round-2 tasks
below revise that tree. Where a round-1 task is superseded, the round-2 task
says so; the round-1 boxes stay as the implementer left them.

- [ ] **T1 Baseline and brokkr reachability.** Check `.brokkr/state/bin/brokkr`
  against `tooling.json`'s `binary_sha256`; record present/absent. Read
  brokkr#766 and its sub-issues (#757–#765, #467, #462, #430) if network
  hands allow; record which of F1–F7 each already tracks.
  **Test:** `implementation.md` holds the F1–F7 table with a status per fact.
- [ ] **T2 Decision index row** for proposed 0119 (done in round 1; keep).
  **Test:** `grep -c '0119' docs/decisions/README.md` is 1.
- [ ] **T3 Recipe files** (done in round 1; pins extended by T26).
- [ ] **T4 `trial.sh`**, **T5 `score.sh`** (done in round 1; revised by T20, T21).
- [ ] **T6 Via-brokkr selectors** (done in round 1; keep).
- [ ] **T7 `--via-brokkr` on the wrappers** (done in round 1; revised by T22).
- [ ] **T8 Trials through the stand-in driver** — **superseded by T22**: the
  stand-in driver and both end-to-end arms are removed.
- [ ] **T9 Request writing and the brokkr worker branch**, **T10 Slot
  reading and the gate cross-check** (done in round 1; revised by T24, T25).
- [ ] **T11 Controls through the recipe path** (done in round 1; re-run under
  the engine-shaped stand-in, T20).
- [ ] **T12 Journal facts v1 reader and join**, **T13 `report --journal`**,
  **T14 Evidence writer**, **T15 `journal-facts.sh`** (done in round 1;
  revised by T27, T28).
- [ ] **T16 CI and docs** (done in round 1; text revised by T29).
- [ ] **T17 Brokkr facts filed** — **revised by T30**.
- [ ] **T18 Gates**, **T19 Delivery paper** — re-run and rewritten as T31, T32.

## Round 2 (review return, 2026-11-02)

- [x] **T20 Prompt parsing on the engine's rendering (AC4, AC1; plan R1).**
  Write the test first: `recipe.rs::trial_script_parses_the_engine_rendered_prompt`
  feeds `trial.sh` a prompt whose request line is `Feature:
  swarmail-eval-request: /abs/request.json` and whose result path is
  indented four spaces; a sibling case ends every line with `\r\n`; a
  sibling case uses the bare round-1 form. Run it against the committed
  scripts and record the red in `implementation.md`. Then trim each line
  (leading/trailing whitespace and `\r`) as `verify-seat.sh` does, match the
  result path on the trimmed line and the request token anywhere on the
  trimmed line (strip through `swarmail-eval-request: `), in `trial.sh` and
  `score.sh`. Make `tests/data/brokkr-standin.sh` render the engine's shape.
  **Test:** the new test green on both forms; `score.rs` equivalent for
  `score.sh`; the existing recipe, report and proof brokkr-mode tests pass
  through the engine-shaped stand-in; a prompt with the token but a
  relative path still refuses.
- [x] **T21 `score.sh` always records a verdict once `check` ran (AC4; R11).**
  `score.json` is written before the report on exit 0, 1 and other; a
  malformed request or unusable slot refuses `unscorable` with no
  `score.json`.
  **Test:** `recipe.rs::score_script_maps_check_exit_codes` asserts
  `score.json` present with `outcome: harness_error, exit: 2` on the
  truncated-state leaf, and absent on a malformed request.
- [x] **T22 `--via-brokkr` fails closed; assembly proven inline (AC8; R2).**
  In `wrappers::run`, after `validate_driver`, return
  `Err("driver_contract_unknown")` unconditionally, before
  `Inputs::environment()`, `manifest()` or any credential read. Keep
  `through_driver` compiled (test reach only). Delete
  `evals/tests/wrappers_launch/standin-driver.py` and the two end-to-end
  stand-in-driver arms.
  **Test:** inline `wrappers.rs::through_driver_assembles_the_direct_argv_byte_for_byte`
  for all three families (program, argv after `--`, env keys and values,
  cwd); `wrappers_launch::via_brokkr_refuses_before_any_private_read`
  (valid executable driver, selection and credential paths pointing at
  unreadable sentinels that would refuse with `wrapper_config_*` if opened;
  stderr carries `driver_contract_unknown`; nothing spawned, record file
  absent); `via_brokkr_refuses_an_invalid_driver_before_launch` unchanged;
  `git ls-files evals/tests/wrappers_launch/standin-driver.py` is empty.
- [x] **T23 Trial-level argv refusals (AC8; R3).** Where the trial validates
  `--wrapper-profile` today (`runner.rs` request validation), add: the
  public argv contains the literal `--via-brokkr` → `driver_contract_unknown`;
  a `*_via_brokkr` profile without it → `via_brokkr_profile_mismatch`. Both
  exit 2 with the fixed category, no row, no agent launched, before any
  server starts.
  **Test:** inline `runner.rs` tests with the flag at the first, middle and
  last argv position and with each of the three via-brokkr profiles;
  `report.rs::brokkr_mode_refuses_a_via_brokkr_agent_as_a_harness_error`
  (agents file with `--via-brokkr`: slot 1 `harness_error`, admission
  stopped, suffix, exit 2, failure class never `gave_up`).
- [x] **T24 `brokkr_artifacts_unreachable` and the brokkr-mode envelope (AC9;
  R7, R12).** In `brokkr_mode_admissible`, refuse an `--artifacts-dir` that
  does not resolve inside the current directory. Add
  `fn brokkr_envelope_ms(deadline_ms: u64) -> u64` returning
  `deadline + 300_000 + 600_000` and use it for the brokkr worker only.
  **Test:** inline `main.rs` tests: the preflight refuses `/tmp/x` and a
  sibling directory and accepts `./.brokkr/state/eval`; the envelope
  function's value and the direct envelope unchanged;
  `report.rs::brokkr_mode_refuses_without_artifacts_realm_or_a_nesting_deadline`
  gains the unreachable arm.
- [x] **T25 `score_missing` and the slot-stderr category (AC2; R11, R13).**
  `read_slot` returns the gate verdict as `Missing | Present(outcome)`; an
  absent, unreadable, malformed or multi-line `score.json` is
  `score_missing`; a present disagreeing one stays `score_mismatch`. Add
  stand-in modes `no-score` and `category-stderr` (append
  `swarmail-evals: concurrent_trials_refused` or the fixed category the
  scanner knows to `<slot>/trial.stderr`).
  **Test:** `brokkr_mode_turns_every_worker_defect_into_a_row` gains both
  arms: `score_missing` on stderr and as the row's failure for `no-score`;
  the scanned category for `category-stderr`; admission stopped, suffix,
  exit 2 in both; inline `read_slot` test distinguishes the two.
- [x] **T26 Bundle pins (AC3, AC9; R15).** `tests/delivery_recipes.rs` pins
  `trial` `network: true`, both seats `binds: []`, and the description
  mentions `unconfined`.
  **Test:** two more one-fault mutants (a bind on `trial`; `network: false`
  on `trial`) are caught by the named rule.
- [x] **T27 Typed `tool_calls` and `deadline_killed` (AC6, AC7; R9).** In
  `journal.rs`: `tool_calls: Fact<Vec<ToolCall>>`, `deadline_killed:
  Fact<bool>`; `accounted()` = any fact present; aggregates `tool_calls` and
  `deadline_killed` are `null` with `tool_calls_reason` /
  `deadline_killed_reason` `incomplete_join` when any scored joined trial
  lacks the fact; evidence writes no file and records
  `absent: journal_has_no_accounting` for an unavailable fact; `retained`
  beside `truncated`. In `journal-facts.sh`: `tool_calls` unavailable when
  `usage.tool_calls` is absent, null or malformed, present (possibly empty)
  when it is a valid array; `deadline_killed` present only for a JSON
  boolean. Update the goldens; add a tokens-present/tool-rows-absent trial
  to the golden fixture and an untyped negative fixture.
  **Test:** inline `journal.rs` tests (typed decode, untyped `[]`/`false`
  refuse `journal_invalid`, present-empty is zero calls, absent is
  unavailable); `report.rs::report_joins_journal_facts_by_trial_id` with the
  recomputed hand values and the new group; `a_dropped_join_is_red` still
  red; `evidence_*` tests assert no file for the unavailable trial;
  `recipe.rs::journal_facts_script_output_reads_as_facts_v1` covers absent,
  null, malformed and empty `tool_calls` and a non-boolean
  `deadline_killed`.
- [x] **T28 `journal-facts.sh` header and F3 text (R10).** Correct the header:
  a failed export, non-JSON, unreadable version or `EXTRACT` mismatch exits 2
  with the class and writes nothing; only a matching shape with no run for a
  trial yields `journal_missing` by absence.
  **Test:** `recipe.rs` arms: a non-JSON export → exit 2, no `OUT`; a
  mismatched shape → exit 2, no `OUT`, stderr names the class.
- [x] **T29 Documentation (AC15; R4, R5, R6, R8).** `evals/README.md`: the
  corrected closure sentence (machinery lands; tokens per success after the
  forwarding follow-up; structural records now, value-bearing after 0119
  §4); paid agents under `matrix --brokkr` unsupported. `evals/wrappers.md`:
  the four F1 seams by file and mechanism, the full paid read set (F5),
  `--via-brokkr` refuses `driver_contract_unknown` until F2 is observed.
  `.brokkr/recipes/README.md`: engine rendering parsed, follow-up,
  paid-unsupported. `bundle.json` description: one sentence on the
  follow-up.
  **Test:** `git grep -n "closed by the operator's run" evals/README.md` is
  empty; `git grep -n "env -i" evals/wrappers.md` and `git grep -n
  "driver_contract_unknown" evals/wrappers.md .brokkr/recipes/README.md`
  are nonempty; no edit under `docs/evals/baseline-2026-11-01*`.
- [x] **T30 Seam sentinel (AC17; R4).** Set `SWARMAIL_EVAL_SEAM_SENTINEL=1`
  in the test's environment: prove it does not reach `swarmail-evals trial`
  under the real `trial.sh` (seam 1; the trial's recorded worker environment
  or a probe agent that dumps its environment keys) nor the agent launched
  by the wrapper (seams 2–4; the existing wrapper environment test extended
  with the key).
  **Test:** `recipe.rs::the_trial_seat_strips_the_matrix_environment` and
  the extended wrapper environment test; both named in `implementation.md`
  as the posture the follow-up flips.
- [x] **T31 Brokkr facts: read, draft, hand to the operator (AC12; R14).**
  For each of F1–F7: observed (command and output), tracked (sub-issue
  number) or drafted. Without network hands, write the issue texts in
  `implementation.md` **and** in the PR body verbatim, with the F1 four-seam
  description and the F2 refusal, under a first-paragraph sentence "drafted,
  not filed: filing and recording the numbers is a merge precondition owned
  by the operator".
  **Test:** `implementation.md`'s F-table has exactly one disposition per
  fact; no disposition reads "filed" without an issue number.
- [x] **T32 Gates (AC13, AC14).** Run the full quickstart list `--locked`;
  `brokkr compile --bundle .brokkr/recipes/eval` where the binary exists,
  else "not run".
  **Test:** every command's exit code recorded; the `git diff --stat` over
  the lockfiles, `deny.toml`, `scripts/` and `.brokkr/recipes/self/` is
  empty; `TrialResult`, `Metadata`, `MatrixPlan`, `MatrixIndex` show no
  changed field in `git diff main -- evals/src`; `standin-driver.py` is
  gone from the tree.
- [x] **T33 Delivery paper (AC15).** Commit unsigned in the repository's
  style on top of `080db77` (never amend). The PR description's first
  paragraph states: held for the operator's approval by name (touches
  `.brokkr/` and `.github/`, 0056's 2026-11-01 list); the brokkr issues
  filed or drafted-not-filed (merge precondition); this delivery does not
  measure tokens per success and the forwarding follow-up under 0119 §6 is
  required; paid agents under `matrix --brokkr` are unsupported until the
  F2/F1/F5 follow-up; 0119 is proposed, not accepted, and value-bearing
  evidence waits on it; the first engine run (oracle, offline) and the first
  authenticated via-brokkr smoke per family are the operator's; AC16 is the
  operator's paid run after the follow-up; AC11's reading (sequential,
  isolation is #221's); the decision number is provisional.
  **Test:** `git log -1 --format=%G?` is `N`; `git status --porcelain` is
  empty; no remote was pushed; `implementation.md` records the first-red
  observation from T20.

## Round 3 (second review return, 2026-11-02; run `issue-148-the-eval-recipe-on-bro-1b9b64e9`)

Built on top of `040795d`, never amended. Order matters: the red-first arms
(T34, T35) are written and observed red against the committed code before
T36 to T38 change it. Each task names the finding it answers (plan.md,
"Revision 3").

- [x] **T34 Stand-in arms for the agreement table, red first (AC2, AC18;
  Finding 1, all four members).** In `evals/tests/data/brokkr-standin.sh`
  add modes `budget-over-pass`, `harness-error-over-pass` and
  `budget-over-unscorable`. After `trial.sh` the stand-in rewrites the
  slot's `row.json` (`outcome`/`failure_class` → `fail`/`budget` or
  `harness_error`/`harness_error`), writes `exit` (`1` or `2`), and rewrites
  the retained `trials/<trial_id>/result.json` to the same row. It then
  rebinds `metadata.json`'s `result` entry (length and SHA-256) exactly as
  `evals/tests/provenance.rs`'s `rebind` does. `budget-over-unscorable`
  additionally truncates the retained `state.json` *without* rebinding. Then
  run the real `score.sh`. Extend `EVAL_TEST_BROKKR_LOG` with one
  `score=<result word>` line per invocation, and retarget the `bad-score`
  mode to the `business` key. If the matrix's commit path binds more than
  `metadata.json` (for example a digest of `metadata.json` in the matrix
  index), rebind that chain the same way. Never relax a check to make the
  rewrite pass. Add the three cases to `evals/tests/report.rs` beside
  `bad-score`/`no-score`.
  **Test:** before T36, run the two legitimate arms against `040795d` and
  record in `implementation.md` that both are red with `score_mismatch`.
  After T36 to T38: `budget-over-pass` commits a `fail`/`budget` row, every
  planned row is produced and the exit is 1. `harness-error-over-pass`
  commits the slot's `harness_error` row unchanged (failure not
  `score_mismatch`). `budget-over-unscorable` and `bad-score` yield
  `score_mismatch`, a `harness_error` row, a zero-time suffix and exit 2.
  `no-score` still yields `score_missing`.
- [x] **T35 `score.sh` per-cell cases, red first (AC4, AC18; Finding 1).**
  In `evals/tests/recipe.rs`, over rewritten oracle and null leaves and
  slot rows (the T34 rewrite, done in Rust with the `provenance.rs`
  helpers), add one case per agreement-table cell: pass/pass → `pass`;
  null fail/fail → `fail`; `fail/budget`/pass → `fail`;
  `harness_error`/pass → `unscorable`; pass row over a failing state →
  `unscorable`; `fail/budget` over a truncated `state.json` →
  `unscorable`; `row.json` missing, a symlink, or two lines →
  `unscorable`. In every case `score.json` has `business` and
  `check_exit` and no `outcome` key. A malformed request still writes no
  `score.json`.
  **Test:** the cases themselves, observed red against `040795d` (no
  `business` key; `fail/budget` reports `pass`) and green after T36.
- [x] **T36 `score.sh` follows the agreement table (AC4, AC18; Finding 1).**
  Write `score.json` as `{schema_version, trial_id, business, check_exit}`
  (sibling temporary and rename, before reporting, unchanged order). Read
  `<slot>/row.json` strictly: regular file, `! -L`, `wc -c` ≤ 65536,
  exactly one line, `jq -e '(.outcome|IN("pass","fail","harness_error"))
  and (.failure_class|type|IN("string","null"))'`. Read only those two
  fields. Report by the table, and `unscorable` with `score.json` written
  when the row is unreadable. Update the header comment to the table.
  Notes carry no path and no row byte.
  **Test:** T35 green. `recipe.rs`'s existing "notes carry no path or
  stderr byte" assertion holds for every new case.
- [x] **T37 `gate_verdict` and `gate_refusal` follow the agreement table
  (AC2; Finding 1).** In `evals/src/main.rs`, `GateVerdict::Ruled` carries
  a closed `Business { Pass, Fail, Unscorable }` read from the `business`
  key. Any other value, a missing key and the revision-2 `outcome` key are
  `Missing`. `gate_refusal(&GateVerdict, &TrialResult)` matches
  `(row.outcome, row.failure_class, business)` per the table. The caller
  passes `&row`. `outcome_name` is removed if nothing else uses it. A
  committed `fail/budget` or `harness_error` row is never rewritten, and
  `stop_admission` stays today's expression.
  **Test:** inline tests at the existing `gate_refusal` test cover every
  cell: pass × {pass ok; fail, unscorable mismatch}; business fail ×
  {fail ok; pass, unscorable mismatch}; budget × {pass, fail ok;
  unscorable mismatch}; harness_error × {pass, fail, unscorable ok};
  `Missing` → `score_missing` for every row. `gate_verdict` cases: an
  unknown value, a missing key and an `outcome`-only line are `Missing`.
  T34 green.
- [x] **T38 Policy reason texts (AC18; Finding 1).** In
  `.brokkr/recipes/eval/policy.json` restate the `reason` of `SCORE-PASS`,
  `SCORE-FAIL` and `SCORE-UNSCORABLE` (and the description sentence) to the
  agreement table. Ids, results, transitions and severities stay unchanged,
  and no rule is added.
  **Test:** `tests/delivery_recipes.rs` still pins exactly five rules with
  their ids, from/result/next and hard severities. If it pins reason text,
  its fixture follows the new text. The one-fault mutants stay red.
- [x] **T39 README and recipe docs (Finding 2 and Finding 1's docs).**
  `evals/README.md`: replace the recipe section's `score_mismatch` sentence
  with the agreement table (one short table or three sentences), and
  rewrite the "Tokens, turns and cost are not in the rows" sentence. It
  must name the F1/F2 forwarding follow-up and the operator's acceptance of
  0119 §6 as preconditions before a paid run can measure tokens, turns or
  tool-call records, and it must not say a paid run alone closes either
  #151 gap. `.brokkr/recipes/README.md`: the score seat's results by the
  same table.
  **Test:** `grep -n "until the operator's paid run" evals/README.md`
  finds nothing. `grep -n "0119 §6" evals/README.md` finds the sentence.
  `grep -n "disagrees with the row" evals/README.md .brokkr/recipes/README.md`
  finds nothing.
- [x] **T40 `through_driver` is test-only (AC8; Finding 3).** In
  `evals/src/wrappers.rs` put `#[cfg(test)]` on `through_driver` and delete
  `run()`'s `let mut cmd = match via_brokkr { Some(driver) =>
  through_driver(..), None => cmd }`, so `cmd` stays the direct command. The
  comment at the refusal names the follow-up that restores the arm.
  **Test:** the evals crate's non-test `cargo build --locked` and
  `cargo clippy --all-targets --locked -- -D warnings` stay clean.
  `grep -n "through_driver(&cmd" evals/src/wrappers.rs` finds nothing. The
  inline `through_driver_assembles_the_direct_argv_byte_for_byte` and the
  `driver_contract_unknown` refusal tests pass unchanged.
- [x] **T41 Seam proofs that can go red (AC17; independent Finding 4).** In
  `evals/tests/recipe.rs` replace
  `the_trial_seat_strips_the_matrix_environment` with two tests. (1) Seam
  1: the request's `evals_bin` is a probe script that writes its own
  environment keys to a file and exits 2. Run the real `trial.sh` with
  `SWARMAIL_EVAL_SEAM_SENTINEL=1` in its environment. Assert the record
  contains `SWARMAIL_BIN` (the probe ran under `trial.sh`) and no key
  containing `SENTINEL`. (2) Seam 2: run `swarmail-evals trial --task signup
  --agent probe -- <probe agent>` directly, with the sentinel in the trial's
  own environment. Assert that the probe agent's recorded keys lack it.
  Under the existing `trial_scope()` guard. No claim is made for seams 3–4.
  **Test:** both green. Locally delete `env -i` from `trial.sh`, observe
  (1) red, restore it, and record that in `implementation.md` (the mutation
  is never committed).
- [x] **T42 Implementation record and gates (AC13, AC14, AC15).** Extend
  `specs/eval-brokkr-recipe/implementation.md` with the round-3 receipts:
  the T34/T35 first-red observations, the T41 mutation, and the gate list
  run `--locked` exactly as the plan's quickstart (evals fmt/clippy/test
  against the candidate's `swarmail`, coverage gate plus selftest,
  publish-guard, seal-branch, deny; the eval bundle compile "not run, no
  binary" unless the binary exists). Record the #302 flake if it reappears,
  never retry-to-green silently. Commit unsigned on top of `040795d`.
  **Test:** every command's exit code recorded. `git diff --stat main --
  Cargo.toml Cargo.lock evals/Cargo.toml evals/Cargo.lock deny.toml
  scripts/ .brokkr/recipes/self/` is empty. `git log -1 --format=%G?` is
  `N`. `git status --porcelain` is empty. The PR body (T33's paragraph)
  gains one sentence: the gate agrees with the row by the agreement table,
  and a `fail/budget` trial is recorded as data.
