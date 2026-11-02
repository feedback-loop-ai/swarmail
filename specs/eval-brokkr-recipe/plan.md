# Implementation plan: the eval recipe on brokkr (interim path) — revision 3

Implements [spec.md](spec.md), revision 2 (returned from review 2026-11-02
under accepted 0102's spec-defect addendum). Declares proposed
[0116](../../docs/decisions/0116-eval-brokkr-recipe.md), revised the same
day. Compatible work under accepted 0045 §8: nothing here contradicts an
accepted ruling; the two parts that would (value-bearing evidence, 0075 §4;
forwarding an accounting channel through the cleared environment, 0075 §1)
are specified in 0116 and deliberately not implemented. The chief commits
spec.md, plan.md, tasks.md and the decision record only; the round-1 design
commit `8b9abc6`, the build `080db77` and `implementation.md` stay as
history and are revised on top, never amended.

## Revision 3: what changes and which finding each change answers

The second review returned head `040795d` (tree `9952152`) with `residual`,
`spec_defect: true`, highest severity medium, security clean. The panel
did not challenge the committed shape (two exec seats, rows unchanged, a
report-side join, structural evidence, sequential admission), and it is not
re-opened. The implementer reads this table and tasks T34–T42, not the
review's notes.

| # | Change | Finding answered | Where |
|---|---|---|---|
| S1 | **The agreement table** replaces equality. A `pass` row needs a `pass` replay. A business `fail` row needs `fail`. A `fail/budget` row needs `pass` or `fail`, never `unscorable`. A `harness_error` row accepts any ruled replay. The row's class alone shows the deciding leg (`tasks.rs:406-413`, `runner.rs:2938`). A committed `fail/budget` or `harness_error` row is never rewritten. | Finding 1 (medium, spec defect): correctness, guarantees, independent, security | `main.rs` `gate_verdict`/`gate_refusal` (T37) |
| S2 | `score.json` is `{schema_version, trial_id, business, check_exit}`. The overloaded `outcome` key goes. | Finding 1 (the root: a business replay was named as the trial's outcome) | `score.sh`, `main.rs` (T36, T37) |
| S3 | `score.sh` reads the row's `outcome` and `failure_class` strictly and reports the trial's class by the table (`unscorable` for `harness_error` and contradictions), so the run's terminal and the row agree for every legitimate row. | Finding 1 ("state which gate result the policy rules for such rows, so the journal's run outcome and the row never disagree": correctness, independent) | `score.sh` (T36) |
| S4 | Policy reason texts restated to the table. Ids, results and transitions are unchanged, and no rule is added. | Finding 1 (as S3) | `policy.json` (T38) |
| S5 | Red-first stand-in arms `budget-over-pass`, `harness-error-over-pass` and `budget-over-unscorable`; `bad-score` retargeted; the stand-in logs the score seat's result word. Per-cell `score.sh` cases. | Finding 1 ("add red-first stand-in arms ... do not weaken the gate"); robustness C5 arms 1–3, 5 and 6 | `brokkr-standin.sh`, `report.rs`, `recipe.rs` (T34, T35) |
| S6 | The README's `score_mismatch` sentence is replaced by the table. The #151 sentence names the F1/F2 follow-up and 0116 §6 acceptance as preconditions. | Finding 1 (guarantees: "correct ... the README's `score_mismatch` sentence together"); Finding 2 (low) | `evals/README.md`, `.brokkr/recipes/README.md` (T39) |
| S7 | `through_driver` is `#[cfg(test)]`, and `run()`'s driver arm is deleted. | Finding 3 (low) | `wrappers.rs` (T40) |
| S8 | Seam 1 is proven by a probe `evals_bin` that records its own environment (it can go red), and seam 2 by the trial run directly. Seams 3–4 are claimed only as unreachable. | Independent Finding 4 (low) | `recipe.rs` (T41) |
| — | `trial_args`' `let _ = bin`: no change, because `bin` is used in the `#[cfg(test)]` block at `main.rs:1272-1286`. | Correctness Finding 4 (info) | — |
| — | The #302 flake is left to its parallel fix. | Tests residual (low) | — |

**No acceptance criterion is weakened.** AC2's `score_mismatch` arm is
*corrected toward accepted decisions* 0055 §2 and 0071 §2, the authority
for which leg decides a row. The review's evidence is
`provenance.rs::business_replay_never_rewrites_budget_or_infrastructure_history`,
which pins that a business replay legitimately differs from a budget or
infrastructure row. The criterion grows: it gains the budget-unscorable
refusal (a check revision 2 did not have) and two arms that must *not*
refuse. AC4 gains one case per table cell. AC17 is narrowed only where
the finding showed it claimed more than its test could show (seams 3–4).
Seam 1 gains a proof that can go red. AC18 is new.

**Positions, ruled.** *Adopted from simplicity:* option (a) in the one
function that already owns it; the observation that the row's class names
the deciding leg; no full-precedence replay; no new policy rule or result;
the narrowed seam 3–4 claim; through_driver test-only rather than deleted;
#302 out of scope. *Adopted from robustness:* budget never unscorable; the
score seat reports the trial's class so the journal never says `done` for a
`harness_error` row (simplicity's "the seat names the state" would leave
that, and would let `brokkr compare` count budget failures as passes); a
seam-1 proof that observes the trial process's own environment; the
vacuity control (`bad-score` stays red). *Rejected from robustness:* a
derived `outcome` in `score.json` (vacuous), a `score harness_error → stop`
rule (a result outside the seat's vocabulary, about an unobserved engine
seam), a trap (changes no recorded outcome), the populated-root refusal and
the trial-id hardening (outside the finding; `check` already refuses `..`),
and the #302 test change (collides with the parallel fix). *Rejected from
simplicity:* score.sh and policy semantics unchanged, and dropping `bin`
(it breaks the test build).

## Review return (revision 2): what changes and which finding each change answers

The implementer reads this table and the tasks, not the review's notes.
Severities are the panel's; "spec" marks the defects that returned the run.

| # | Change | Finding answered | Where |
|---|---|---|---|
| R1 | Trim every prompt line; match the request token anywhere on the trimmed line; the stand-in renders `Feature: swarmail-eval-request: …` with an indented result path; the engine-shape test is written first and observed red | correctness 1 (low), independent 2 (medium); both positions | `trial.sh`, `score.sh`, `brokkr-standin.sh`, `recipe.rs` |
| R2 | `--via-brokkr` refuses `driver_contract_unknown` unconditionally after `driver_invalid`, before any private read or spawn; `through_driver` proven inline; `standin-driver.py` and its two end-to-end arms removed | guarantees 3 (high, spec), independent 5 (low) | `wrappers.rs`, `tests/wrappers_launch/`, `tests/wrappers.rs` |
| R3 | The trial refuses before launch: `--via-brokkr` in the agent argv → `driver_contract_unknown`; a `*_via_brokkr` profile without it → `via_brokkr_profile_mismatch` | guarantees 3, guarantees 7 (low); the fake-`gave_up` hazard the simplicity position names | `runner.rs` request validation, `main.rs` trial argument check |
| R4 | F1's four seams named in spec, 0116 §6 and `wrappers.md`; a sentinel test proves nothing crosses them; no forwarding built | independent 1 (medium), correctness 2 (low) | docs, `recipe.rs`, wrapper environment tests |
| R5 | AC16 and the READMEs stop claiming this delivery closes the #151 rows; the PR states the follow-up | independent 1, guarantees 6 (medium) | `evals/README.md`, `.brokkr/recipes/README.md`, PR body |
| R6 | F5 lists the full paid read set; no bind guessed; score seat hands stated read-write writing only `score.json`; paid agents under `matrix --brokkr` documented unsupported | guarantees 2 (high, spec), guarantees 4b (medium), correctness 6, independent 6 | spec, 0116 §7, `wrappers.md`, bundle description |
| R7 | `matrix --brokkr` refuses `--artifacts-dir` outside the current directory before any spawn (`brokkr_artifacts_unreachable`) | correctness 5 (info); robustness R4c | `main.rs::brokkr_mode_admissible` |
| R8 | AC3 and 0116 §7 claim only CI's pinned-bundle loop; the implement seat records the compile as not run | guarantees 4 (medium), correctness 3 (low), independent 4 (low) | spec, 0116, `implementation.md` |
| R9 | `tool_calls` and `deadline_killed` typed `Fact<_>` in facts v1 (in place); `accounted()` is "any fact present"; aggregates `null` with `incomplete_join`; evidence absent, no file, for an unavailable fact; `journal-facts.sh` distinguishes absent from empty; goldens gain the tokens-present/tool-rows-absent case; untyped forms refuse | guarantees 5 (medium) | `journal.rs`, `journal-facts.sh`, `tests/data/journal-facts-*.json`, `report.rs` tests |
| R10 | `journal-facts.sh`'s F3 text corrected to the code: exit 2 with the class, no facts file; header sentence about `journal_missing` corrected | guarantees 8 (low); simplicity over robustness | spec F3, script header |
| R11 | `score_missing` distinct from `score_mismatch`; stand-in `no-score` mode; `score.sh` writes `score.json` before reporting on every exit of `check` | tests 1 (info); robustness R8 | `main.rs` slot acceptance, `brokkr-standin.sh`, `score.sh`, `report.rs` |
| R12 | Brokkr-mode envelope = deadline + 300 000 + 600 000 ms, a pure function with an inline test | robustness R10 (new, premise corrected) | `main.rs` |
| R13 | Stand-in `category-stderr` mode; the slot-stderr category arm is asserted through the CLI | tests 2 (info) | `brokkr-standin.sh`, `report.rs` |
| R14 | AC12/T17: filing is an operator-owned merge precondition; the PR carries the texts verbatim and says "drafted, not filed" when they are; the implement seat reads brokkr#766 when its hands allow | guarantees 1 (high, spec), independent 3 (low) | spec AC12/AC15, `implementation.md`, PR body |
| R15 | Bundle pin gains `trial` `network: true` and both `binds: []`; AC9 adds the envelope test | AC3 strengthening alongside R6 | `tests/delivery_recipes.rs` |
| — | `trial_args`' `let _ = bin`: no change (used under `cfg(test)`); cold script arms and the worker's HOME: named residuals | correctness 4, independent 6, tests 3 (info) | spec residuals |

No acceptance criterion is weakened to pass review. AC3 and AC16 are
corrected because the findings showed the criteria themselves overstated
what any seat can run (no seat holds the binary; the channel is severed by
construction); the evidence is `verify-seat.sh:125` (compiles only `self`)
and the four seams at `trial.sh` (`env -i`), the runner's `CallerEnv`,
`wrappers.rs` (`env_clear`) and `wrappers.rs` (`Stdio::null`). AC8 is
corrected toward the task's own constraint (never work around a missing
brokkr capability inside Swarmail): a guessed driver argv that runs is such a
workaround. AC12 is not weakened: "filed" stays the criterion; the revision
names who can satisfy it.

## Technical context

- `evals/` is a standalone crate outside the root coverage denominator
  (accepted 0055); every new branch still ships with a test. No file under
  root `src/` changes, so the exact 100% root gate is unaffected.
- Production surface, all in `evals/`: `src/journal.rs` (facts reader, join,
  evidence writer), `src/main.rs` (`--brokkr` on `Matrix` with its
  preflights and envelope, `--journal` and `--evidence-dir` on `Report`,
  `--via-brokkr` on the three `Agent*` commands, the slot acceptance),
  `src/runner.rs` (the two trial-level argv refusals), `src/wrappers.rs`
  (`validate_driver`, the unconditional refusal, `through_driver` kept),
  `src/provenance.rs` (three `WrapperProfile` variants), `src/report.rs`
  (the `journal` object).
- Recipe: `.brokkr/recipes/eval/{bundle.json,policy.json,scripts/trial.sh,scripts/score.sh}`.
- Host-side script: `evals/scripts/journal-facts.sh` (bash + jq). Root
  `scripts/` is the pinned-shim directory and is **not** touched.
- Tests: `evals/tests/recipe.rs`, `report.rs`, `proof.rs`, `wrappers.rs`,
  `wrappers_launch/mod.rs`, `tests/data/brokkr-standin.sh`,
  `tests/data/journal-facts-*.json`, root `tests/delivery_recipes.rs`.
  `evals/tests/wrappers_launch/standin-driver.py` is deleted.
- CI: `eval` in `.github/workflows/ci.yml`'s pinned-bundle loop (round 1).
- No dependency: `std`, `serde`, `serde_json`, `ring` and `libc` cover
  everything. `jq` is already required by the realm's exec scripts.
- brokkr is not reachable from the design seats. The implement seat first
  checks `.brokkr/state/bin/brokkr` against `tooling.json`'s digest; if
  present it observes F1–F7, captures a real export beside the hand-authored
  goldens and runs `brokkr compile --bundle .brokkr/recipes/eval`; if absent
  it records every one of those as **not run** in `implementation.md`.
- `.specify/templates/` holds no realm template; this plan supplies context,
  constitution check, structure, research, data model, contracts, quickstart
  and risks.

## Constitution check

- 0001–0005: untouched. The fixture freezes and snapshots through its admin
  API; the harness never prunes, never changes the cap, never re-parses mail.
- 0020: deterministic join, quantile, envelope and argv-assembly logic is
  inline `#[cfg(test)]` in `journal.rs`, `main.rs`, `wrappers.rs`; wire-shaped
  tests (real `trial`, real scripts, real stand-in worker) live in
  `evals/tests/`; no sleep as synchronization (readiness files with a real
  deadline); fresh tempdir, fresh seeds, no shared mutable state; chaos is
  not involved. `through_driver`'s `#[cfg(test)]`-only reach is a seam, not
  a flag (0020 §4).
- 0045 §8: compatible; 0116 is drafted, not accepted; the value-bearing
  evidence mode and the channel forwarding are not built.
- 0050 §1: the pinned shims and `shims.sha256` do not change; the recipe's
  scripts are the recipe's own.
- 0055 §§6–10, 0063, 0071, 0076: rows, artifacts, scoring precedence, traps,
  replay, concurrency and admission are unchanged; brokkr mode is one more
  worker command behind the same accounting, with two more typed
  categories (`score_missing`, `brokkr_artifacts_unreachable`) and two more
  trial refusals (`driver_contract_unknown`, `via_brokkr_profile_mismatch`).
- 0056 addendum: the PR touches `.brokkr/` and `.github/`, so it is held for
  the operator's approval by name; commits unsigned; never push.
- 0075 §1: every child still starts with a cleared environment; nothing is
  forwarded; the four seams are named for the follow-up the operator must
  accept. 0075 §4: no raw CLI bytes are retained; evidence is structural.
- 0098/0099/0102: this run's route; design returns share 0102's five review
  returns.

## Research and chosen approach

### Why fail closed instead of a guessed driver argv

The task forbids working around a missing brokkr capability inside Swarmail.
A `DRIVER <family> -- <argv>` convention nobody observed, launched against a
real driver with its output nulled and environment cleared, would be exactly
that: a wrong argv becomes the agent's exit, scored as `gave_up`, attributed
to the model. Refusing at the trial before launch (exit 2, no row) and at the
wrapper before any private read makes a premature paid run cost nothing and
pollute nothing. The stand-in driver is cut because the only way to keep it
alive beside the refusal is a production switch between "real" and
"stand-in", a seam 0020 §4 forbids in spirit, and because it proved our own
convention against itself.

### Why the trial refuses, not only the wrapper

`runner.rs`'s scoring precedence gives an agent that exits without acting the
task's idle class with its exit status on the row. A wrapper-only refusal
therefore reads as a model that gave up. The trial already inspects the
public argv for provenance; inspecting it for the literal `--via-brokkr` and
pairing it with the profile is deterministic and pre-launch.

### Why no forwarding and no bind now

Both would guess a shape brokkr has not shown: an environment name or a
stdout relay through three environment-clearing seams and one output seam
(0075 §1), and a bind path on a host the exec box has never been observed
on. A guessed `~` bind can refuse the operator's free offline oracle run on a
host without the directory. The follow-up is one named change, gated on the
operator's smoke that observes F1, F2 and F5 together.

### Why `journal-facts.sh` dies

The journal is append-only. A failed or mismatched export loses nothing; the
operator corrects `EXTRACT` and re-runs. A facts file that labels unread
runs "has no accounting" would be false, and `report` without `--journal`
already gives the typed "no journal" answer byte-identically.

### Why every fact is typed, including an empty tool list

`[]` and `false` are facts ("zero calls", "not killed"); absence is not.
Collapsing them turned a 40-call killed trial into a clean zero, the exact
signal the taxonomy exists to expose. `Fact<T>` already exists for tokens;
applying it to the two remaining fields removes a special case.

### The stand-ins (no engine, no network, no payment)

- **Stand-in worker** (`evals/tests/data/brokkr-standin.sh`): accepts
  `run --recipe eval --feature "swarmail-eval-request: PATH"`, synthesizes
  each seat's prompt **as the engine renders it** (`Feature:
  swarmail-eval-request: PATH` and the result path indented four spaces),
  runs `bash .brokkr/recipes/eval/scripts/trial.sh PROMPT`, reads the result,
  and on `ran` runs `score.sh` the same way; exits 0. Modes selected by
  `EVAL_TEST_BROKKR_MODE`: `ok`, `no-row`, `foreign-row`, `bad-exit`,
  `bad-score`, **`no-score`** (skip the gate), **`category-stderr`** (append
  a fixed 0076 category line to `<slot>/trial.stderr` after the trial),
  `hang-after-start` (write a readiness file, then sleep until killed).
- **No stand-in driver.** Argv assembly is proven inline.

## Project structure: files touched

**Revision 3 delta on `040795d`:** `.brokkr/recipes/eval/scripts/score.sh`
(agreement table, `business`/`check_exit`, strict row read);
`.brokkr/recipes/eval/policy.json` (reason texts only);
`.brokkr/recipes/README.md`; `evals/src/main.rs` (`GateVerdict`,
`gate_verdict`, `gate_refusal(&GateVerdict, &TrialResult)`, inline tests);
`evals/src/wrappers.rs` (`#[cfg(test)]` `through_driver`, driver arm
deleted); `evals/tests/data/brokkr-standin.sh` (three modes, `bad-score`
retargeted, `score=` log line); `evals/tests/report.rs` (three arms and the
AC18 assertions); `evals/tests/recipe.rs` (per-cell cases, two seam tests);
`tests/delivery_recipes.rs` (only if it pins reason text);
`evals/README.md`; `specs/eval-brokkr-recipe/implementation.md`. Not
touched: everything revision 2 listed as untouched, plus `check`, the
`artifacts.rs` replay, the runner's precedence, `trial.sh`, `bundle.json`,
journal facts v1 and the report.

### Revision 2 delta on `080db77`

| File | Change |
|---|---|
| `.brokkr/recipes/eval/scripts/trial.sh` | trim lines; request token anywhere on the line; otherwise unchanged |
| `.brokkr/recipes/eval/scripts/score.sh` | same parsing; `score.json` written before every report that follows `check` |
| `.brokkr/recipes/eval/bundle.json` | description sentence: paid agents wait for the follow-up; hands unchanged |
| `.brokkr/recipes/README.md` | engine rendering, follow-up, paid-unsupported sentence |
| `tests/delivery_recipes.rs` | pin `trial` `network: true`, both `binds: []` (two more mutants) |
| `evals/src/journal.rs` | `tool_calls: Fact<Vec<ToolCall>>`, `deadline_killed: Fact<bool>`; `accounted()` without the list heuristic; `tool_calls`/`deadline_killed` aggregates `null` + reason under `incomplete_join`; evidence skipped with `absent` for an unavailable fact; `retained` beside `truncated` |
| `evals/src/main.rs` | `brokkr_artifacts_unreachable` preflight; `brokkr_envelope_ms(deadline)` pure function; `score_missing` distinct from `score_mismatch`; trial-level `--via-brokkr`/profile check (or in `runner.rs` request validation, wherever `wrapper_profile` is validated today) |
| `evals/src/runner.rs` | `driver_contract_unknown` and `via_brokkr_profile_mismatch` refusals on the public argv and profile, pre-launch |
| `evals/src/wrappers.rs` | unconditional `driver_contract_unknown` after `validate_driver`; `through_driver` kept under test reach; inline `Command` inspection tests |
| `evals/scripts/journal-facts.sh` | typed `tool_calls`/`deadline_killed`; absent vs empty; header text corrected (dies, never a soft facts file) |
| `evals/tests/data/brokkr-standin.sh` | engine-shaped prompts; `no-score`, `category-stderr` modes |
| `evals/tests/data/journal-facts-*.json` | typed shape; tokens-present/tool-rows-absent trial; untyped negative fixture |
| `evals/tests/recipe.rs` | engine-rendered prompts (first red); `\r\n` case; `unscorable` writes `score.json`; malformed request writes none; seam-1 sentinel |
| `evals/tests/report.rs` | `score_missing`, `category-stderr`, `brokkr_artifacts_unreachable` arms; typed-join goldens; evidence absent-without-file |
| `evals/tests/wrappers.rs`, `evals/tests/wrappers_launch/mod.rs` | refusal before private read; remove the two stand-in-driver arms |
| `evals/tests/wrappers_launch/standin-driver.py` | **deleted** |
| `evals/README.md`, `evals/wrappers.md` | corrected closure sentence; four seams; full paid read set; `--via-brokkr` refuses |
| `docs/decisions/0116-eval-brokkr-recipe.md` | revised by the chief (§1 score seat hands, §3 typed facts, §5 refusal, §6 four seams, §7 CI-only compile; dated revision note) |
| `specs/eval-brokkr-recipe/implementation.md` | the implementer's record, extended with the round-2 receipts and the first-red observation |

Not touched: root `src/`, `scripts/`, `.brokkr/recipes/self/`, every other
recipe, `Cargo.*`, `evals/Cargo.*`, `deny.toml`, the row and artifact
schemas, `docs/evals/baseline-2026-11-01.md`, `.github/` beyond round 1.

## Data model

- **Request v1** (`<slot>/request.json`): `schema_version: 1`, `trial_id`,
  `slot`, `trials_root`, `evals_bin`, `swarmail_bin`, `args: [string]`
  (after the `trial` subcommand). ≤ 64 KiB. Absolute paths only.
- **Slot outputs**: `row.json` (one schema-2 row), `exit` (one decimal),
  `trial.stderr` (≤ 64 KiB), `score.json` (`schema_version: 1`, `trial_id`,
  `business` ∈ {`pass`, `fail`, `unscorable`}, `check_exit`; present
  whenever `check` ran; **revision 3** renames `outcome`/`exit`, and a line
  carrying `outcome` is `score_missing`).
- **Journal facts v1 (revised in place)**: five typed facts per trial
  (`tokens`, `turns`, `wall_time_ms`, `deadline_killed`, `tool_calls`), each
  `{status: present, value}` or `{status: unavailable, reason}`; closed
  reasons; closed tool outcomes; strict reader with no defaults.
- **Report `journal` object**: `tokens_per_success`, `tool_calls` and
  `deadline_killed` are each a value or `null` with a sibling `*_reason`.
- **Evidence record**: `{"schema_version":1,"trial_id":…,"run_id":…,"truncated":bool,"retained":N,"tool_calls":[…]}`,
  ≤ 64 KiB, written only for a present `tool_calls` fact, bound by
  `{path, length, sha256}` in the report.

## Contracts

- CLI: `matrix --brokkr PATH` (requires `--artifacts-dir` inside the current
  directory; refusals `brokkr_requires_artifacts`, `brokkr_realm_invalid`,
  `brokkr_artifacts_unreachable`, `deadline_exceeds_seat`, row-less exit 2).
  `report --journal PATH [--evidence-dir PATH]` (refusals `journal_invalid`,
  `journal_unmatched_trial`, `journal_duplicate_trial`,
  `evidence_requires_journal`; exit 2, no partial report).
  `agent-<family> --via-brokkr PATH` (refusals `driver_invalid` then
  `driver_contract_unknown`, in that order, before any private read).
  `trial` (refusals `driver_contract_unknown`, `via_brokkr_profile_mismatch`
  on the public argv, exit 2, no row). `--wrapper-profile` keeps the three
  names. Slot categories: `trial_row_missing`, `trial_context_mismatch`,
  `score_missing`, `score_mismatch` (revision 3: a business replay outside
  the row's agreeing set under the agreement table), the 0076 categories. No exit code
  changes elsewhere; stderr keeps fixed categories only.
- Recipe seat results: `trial` → `ran` with `inputs.trial_id`, or
  `harness_error` with empty inputs; `score` → `pass`, `fail`, `unscorable`,
  empty inputs, chosen by the agreement table (revision 3): the trial's
  class when the replay agrees, `unscorable` for a `harness_error` row or a
  contradiction. Exactly `result`, `inputs`, `notes`.
- Library: `journal::read(&[u8]) -> Result<Facts, String>`,
  `journal::join(&[TrialResult], &Facts) -> Result<Joined, String>`,
  `journal::write_evidence(&Path, &Joined) -> EvidenceIndex`,
  `report::report_with_journal(&[TrialResult], Option<&Joined>) -> Value`;
  `report::report` keeps its signature and output.

## Quickstart

```bash
# from the repository root, offline, locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
SWARMAIL_BIN="$(cargo build --locked --bin swarmail --message-format=json \
  | jq -r 'select(.reason == "compiler-artifact" and .target.name == "swarmail"
                  and (.target.kind | index("bin"))) | .executable')"
test -x "$SWARMAIL_BIN"
(cd evals && cargo fmt --all -- --check)
(cd evals && cargo clippy --all-targets --locked -- -D warnings)
(cd evals && SWARMAIL_BIN="$SWARMAIL_BIN" cargo test --locked)
brokkr compile --bundle .brokkr/recipes/self   # not run without the binary; never recorded as passed
brokkr compile --bundle .brokkr/recipes/eval   # CI's loop is the only pre-merge run
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
git diff --stat main -- Cargo.toml Cargo.lock evals/Cargo.toml evals/Cargo.lock deny.toml scripts/ .brokkr/recipes/self/   # empty
test ! -e evals/tests/wrappers_launch/standin-driver.py
! grep -n 'through_driver(&cmd' evals/src/wrappers.rs                       # revision 3: no production driver arm
(cd evals && cargo build --locked)                                          # non-test build: through_driver is test-only
! grep -n "until the operator's paid run" evals/README.md                   # Finding 2
```

The operator's first engine run, offline and free, after merge (oracle and
null only; no paid agent runs under `matrix --brokkr` until the follow-up):

```bash
export PATH="$PWD/.brokkr/state/bin:$PATH"
E=evals/target/debug/swarmail-evals
$E matrix --brokkr "$PWD/.brokkr/state/bin/brokkr" --tasks "$($E tasks | jq -r '[.tasks[].id] | join(",")')" \
  --agents controls.json --trials 1 --artifacts-dir "$PWD/.brokkr/state/eval/controls" > controls.jsonl
bash evals/scripts/journal-facts.sh "$PWD/.brokkr/state/bin/brokkr" .brokkr/state/forge.db facts.json $(jq -r .trial_id controls.jsonl)
$E report --input controls.jsonl --journal facts.json --evidence-dir "$PWD/.brokkr/state/eval/controls-evidence"
```

## Risks

| Risk | Mitigation |
|---|---|
| The engine's prompt rendering differs again from what this seat observed (`Feature: ` prefix, indented result path). | The parser accepts the token anywhere on a trimmed line and both forms; the request path is still validated absolute, regular, no-symlink, ≤ 64 KiB, `jq`. A third shape surfaces as `harness_error` at slot 1, visible, and is a one-line parser change. |
| `brokkr driver <family>` turns out to run as an opaque child with a different argv than `DRIVER <family> -- <argv>` (F2). | Nothing launches it: `driver_contract_unknown` at the trial and the wrapper. The follow-up implements the observed form; `through_driver` is the starting point, not a contract. |
| The accounting channel needs 0075 §1 widened (F1). | Named seams, no forwarding now; 0116 §6 is the operator's to accept; the sentinel test documents the current posture and flips in the follow-up. |
| The export shape differs from `EXTRACT` (F3). | The script exits 2 with the class; the operator corrects `EXTRACT` and re-runs; the journal is append-only, nothing is lost. |
| The engine refuses a work-class exec seat or a policy without delivery phases (F4). | CI's loop compiles it before merge; the run path is proven with the stand-in worker; an engine refusal is filed, not patched around. |
| The exec box cannot reach the slot or the binaries (F5). | The slot root must lie inside the current directory before spawn; the first unreachable path is filed; paid binds wait for the follow-up. |
| The trial's advisory admission is blind inside a private `/tmp` (F6). | Named residual; the matrix is sequential; the operator runs nothing else, as for the baseline. |
| A worker killed by brokkr's seat deadline looks like an agent failure. | `deadline_exceeds_seat` keeps the trial under 1800 s; the envelope carries the score seat's 600 s; a worker with no row is `harness_error` (AC2); `deadline_killed` is a typed fact (F7). |
| A hostile unconfined agent rewrites `row.json` or `score.json`. | Row must match the planned cell and worker context, pair with `exit`, agree with a present gate verdict and be bound by `metadata.json`; `score_missing` and `score_mismatch` both stop admission. Residual named for #221 and the broker. |
| Evidence leaks a secret. | No argument or result bytes are copied, ever; names are bounded printable ASCII or digests; the sentinel test plants the admin secret and a token in argument and result positions and asserts absence from every retained byte; no file for an unavailable fact. |
| A partial join biases any aggregate. | `incomplete_join` makes `tokens_per_success`, `tool_calls` and `deadline_killed` `null` whenever any scored trial in the group lacks the fact; never `0`. |
| The trial-level `--via-brokkr` scan misses a differently spelled flag. | The flag has one spelling and the wrapper refuses too; the scan is an exact literal match on the public argv, inline-tested with the flag at every position. |
| Deleting `standin-driver.py` loses the only end-to-end proof of the wrapper's launch path through a driver. | Accepted and stated: the path is refused at runtime, so an end-to-end proof would prove a path no user can reach; the inline `Command` inspection proves assembly; the follow-up restores an end-to-end arm against the observed driver. |
| CI cannot run the engine. | Stated plainly: CI compiles the recipe; the scripts are exercised under bash; the engine run is the operator's receipt. |
| The decision number collides at integration. | Renumber as 0105 and 0108 were; the spec and index row say so. |
| **(R3)** The stand-in's rewritten budget/infrastructure leaf is rejected by a binding the plan did not name (the matrix index or row binds more than `metadata.json`'s `result` entry). | T34 rebinds the whole chain the commit path checks, with the same helpers `provenance.rs` uses. A check is never relaxed to admit the rewrite. If a binding cannot be reproduced from a test, the arm moves to a Rust test that rewrites through `artifacts` primitives, with the same assertions. |
| **(R3)** The bash mirror of the agreement table drifts from `gate_refusal`. | The six stand-in arms run both the real `score.sh` and the real matrix on one slot and assert the seat's word together with the committed row (AC18), so a drift turns one of them red. The Rust function stays the authority that commits. |
| **(R3)** A hostile unconfined agent rewrites `row.json` to steer the score seat's word. | The score seat's word can only move the run's terminal. The matrix re-reads the row against the planned cell, worker context, `exit` and the bound leaf. The row is the measurement, and the residual is #221's (unchanged from revision 2). |
| **(R3)** Renaming `score.json`'s keys breaks a reader. | `read_slot` is the only reader, the file is unreleased, and a line carrying the old key is `score_missing` (tested), never silently accepted. |
| **(R3)** The #302 flake reddens a verify round. | Out of this change and fixed in parallel. The implementer records any recurrence and never retries to green silently (T42). |
| The design is returned again. | Design returns share 0102's five review returns; the sixth parks for the operator. |
