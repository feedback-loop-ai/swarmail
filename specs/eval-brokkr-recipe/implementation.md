# Implementation record: the eval recipe on brokkr (interim path), revision 3

**Revision 3 (round 3, T34 to T42) is the last section, "Round 3". It supersedes the gate receipts and the AC2/AC4 rows below where they differ.**

Implements [spec.md](spec.md) / [plan.md](plan.md) / [tasks.md](tasks.md)
(revision 2, `38dab0f`) under proposed
[0119](../../docs/decisions/0119-eval-brokkr-recipe.md). Base `main` `f815d48`.
Commits unsigned; nothing pushed, merged, published or accepted.

History, never amended: round 1 was designed in `8b9abc6` and built in
`080db77`; the review panel returned the design (0102's spec-defect addendum);
`38dab0f` revised it; **this record covers the revision-2 build on top of
`080db77`** and replaces the round-1 record, which described a stand-in driver,
a `score_mismatch` for a missing verdict and other behaviour that no longer
exists.

**This pull request touches `.brokkr/` and `.github/`, so it merges only on the
operator's approval by name (decision 0056, 2026-11-01 addendum).**

**This delivery lands the measuring machinery and does not measure tokens per
success.** The accounting channel between a nested `brokkr driver` and the
engine has never been observed, and every candidate channel is severed by four
named seams (below); closing the #151 "Not measured" rows needs the operator's
first engine run, the first authenticated via-brokkr smoke per family, the
forwarding follow-up under 0119 section 6, and then the paid matrix.

## What revision 2 changed (task by task)

| Task | Change | Proof |
|---|---|---|
| T20 | Both scripts trim each prompt line (whitespace and `\r`) and match the request token anywhere on the trimmed line; the stand-in worker renders `Feature: swarmail-eval-request: …` with the result path indented four spaces | `recipe.rs::trial_script_parses_the_engine_rendered_prompt` (engine, engine+CRLF, bare), `score_script_parses_the_engine_rendered_prompt`, `the_scripts_still_refuse_a_relative_request_path_in_the_engine_rendering`; the matrix, controls and defect tests run through the engine-shaped stand-in |
| T21 | `score.json` is one compact line (`jq -n -c`), written before the report on every exit of `check`; a malformed request or unusable slot writes none | `recipe.rs::the_score_script_maps_check_exit_codes` |
| T22 | `--via-brokkr` validates the driver then refuses `driver_contract_unknown` before any environment, selection or credential read; `through_driver` kept (test reach only); `standin-driver.py` and its three end-to-end arms deleted | inline `wrappers.rs` tests (`through_driver_assembles_the_direct_argv_byte_for_byte`, `the_via_brokkr_refusal_comes_after_the_driver_check_and_before_every_read`); `wrappers_launch::via_brokkr_refuses_before_any_private_read` (a mode-000 selection sentinel the direct launch refuses with `private_path_unavailable`), `…_an_invalid_driver_before_launch` |
| T23 | The trial (`main.rs`, via `runner::via_brokkr_refusal`) refuses `driver_contract_unknown` (flag in the agent argv, `--via-brokkr` or `--via-brokkr=`) and `via_brokkr_profile_mismatch` (a `*_via_brokkr` profile without it): exit 2, fixed category, no row, before any server starts | inline `runner.rs::via_brokkr_is_refused_at_every_argv_position_and_profile`; `report.rs::brokkr_mode_refuses_a_via_brokkr_agent_as_a_harness_error` (slot 1 `harness_error`, never `gave_up`, zero-time suffix, exit 2, one worker) |
| T24 | `brokkr_artifacts_unreachable` (root must resolve inside the current directory); `brokkr_envelope_ms` = deadline + 300 000 + 600 000, `direct_envelope_ms` unchanged | inline `main.rs` tests (outside paths, `..`, a link out of the realm, a relative root); `report.rs::brokkr_mode_refuses_without_artifacts_realm_or_a_nesting_deadline` |
| T25 | `GateVerdict::{Missing, Ruled}`: an absent, unreadable, malformed or multi-line `score.json` is `score_missing`; a present disagreeing one is `score_mismatch`; stand-in modes `no-score`, `category-stderr` | inline `read_slot`/`gate_refusal` tests; `report.rs::brokkr_mode_turns_every_worker_defect_into_a_row` (six arms) |
| T26 | Bundle pins: `trial` `network: true`, both seats `binds: []`; every one-fault mutant is caught by the rule that names it | `tests/delivery_recipes.rs::eval_recipe_is_two_exec_seats_with_one_attempt_and_five_rules` (11 mutants) |
| T27 | `tool_calls: Fact<Vec<ToolCall>>`, `deadline_killed: Fact<bool>`; `accounted()` is "any fact present"; their aggregates are `null` with `*_reason: incomplete_join`; evidence only for a present `tool_calls`, `retained` beside `truncated`; `journal-facts.sh` types each fact on its own | inline `journal.rs` tests; `report.rs` goldens and refusals; `recipe.rs::journal_facts_script_output_reads_as_facts_v1` |
| T28 | `journal-facts.sh` exits 2 with a class (`export_failed`, `export_not_json`, `export_version_unreadable`, `export_shape_mismatch`) and writes nothing; header corrected | `recipe.rs::journal_facts_script_refuses_without_writing_a_file` (13 cases), `a_matching_export_with_no_run_for_a_trial_yields_journal_missing_by_absence` |
| T29 | `evals/README.md`, `evals/wrappers.md`, `.brokkr/recipes/README.md`, `bundle.json` description | `git grep` checks run on the final tree: "closed by the operator's run" is absent from `evals/README.md`; `env -i` and `driver_contract_unknown` are present in `evals/wrappers.md` and `.brokkr/recipes/README.md`; nothing under `docs/evals/` changed |
| T30 | The four F1 seams named; sentinel proofs | `recipe.rs::the_trial_seat_strips_the_matrix_environment` (seam 1, real `trial.sh`, probe agent); the all-six-models wrapper test asserts `SWARMAIL_EVAL_SEAM_SENTINEL` is absent from the CLI's environment (seams 2 to 4) |

Judgment calls where the spec was silent:

* **Aggregates are withheld when any scored trial in the group lacks the fact,
  joined or not.** AC6 says "scored joined trial"; plan.md's risk table says
  "any scored trial in the group". A trial the journal never saw lacks every
  fact, and a total over the others would silently omit it, so I took the wider
  reading (the same rule as `tokens_per_success`); the AC6 checks hold under
  either. The oracle group therefore shows `tool_calls: null` with
  `incomplete_join`.
* **The golden fixture has four (task, agent) groups, not three.** AC6 needs a
  group with tokens present and tool rows absent (numeric token figures) and a
  separate group with a `journal_missing` trial; trial 8 sits under task `otp`.
  8 rows and 3 agents hold.
* **0076's three scanned stderr categories are not extended.**
  `driver_contract_unknown` and `via_brokkr_profile_mismatch` are asserted in
  the slot's `trial.stderr`; the matrix reports that slot as `trial_row_missing`.
* `--via-brokkr=VALUE` is refused with the bare flag (clap accepts both).
* The matrix tests run in a throwaway realm (`brokkr_realm`) holding a link to
  the real `bundle.json`, because the new preflight needs the artifact root
  inside the working directory.
* A present empty tool list writes a record (`retained: 0`); "an empty file is
  never written as evidence" means zero bytes.

## First red (T20)

The `Feature:`-prefixed tests were written first and run against the committed
round-1 scripts. Observed on 2026-11-02:

```
test the_scripts_still_refuse_a_relative_request_path_in_the_engine_rendering ... FAILED
test score_script_parses_the_engine_rendered_prompt ... FAILED
test trial_script_parses_the_engine_rendered_prompt ... FAILED
  score.sh wrote a result and exited 0: score: result path missing
  trial.sh wrote a result and exited 0: trial: result path missing
```

The round-1 scripts found neither the indented result path nor the
`Feature: `-prefixed request, so every real slot would have refused. After the
trim-and-match change all three are green. (The third fails on the old script
only because it exited 2 without a result; it pins that a relative request
path still refuses.)

## The four seams (brokkr fact F1)

Nothing is forwarded. Identically in the spec, 0119 section 6 and
`evals/wrappers.md`: (1) `trial.sh` runs the trial under `env -i PATH LANG
LC_ALL SWARMAIL_BIN`; (2) the runner builds the agent's environment afresh;
(3) `wrappers::through_driver` calls `env_clear`; (4) `through_driver` routes
the driver's stdout and stderr to `/dev/null`. Tokens per success stays `null`
with a typed reason in every via-brokkr group until the follow-up.

## T31: brokkr facts F1 to F7, one disposition each

`.brokkr/state/bin/brokkr` is **absent** in this seat (the pinned binary,
`tooling.json` 0.12.0, `binary_sha256` `29dc3309…cad`) and there is no network:
brokkr#766 and its sub-issues (#757 to #765, #467, #462, #430) were **not
read**, nothing was observed from the pinned binary, and nothing was filed. No
fact is claimed "tracked", because I could not read the sub-issues.

| Fact | Disposition | What ships regardless |
|---|---|---|
| F1 accounting channel for a nested driver | **drafted, not filed** (issue 1) | the four seams named; sentinel proofs; no forwarding; tokens `null` with a typed reason |
| F2 driver invocation form | **drafted, not filed** (issue 2) | `--via-brokkr` refuses `driver_contract_unknown` at the trial and the wrapper; assembly proven inline |
| F3 per-run export shape | **drafted, not filed** (issue 3) | facts v1 reader and goldens; `journal-facts.sh` fails typed, exit 2 |
| F4 two-seat recipe with its own policy | **falsified by the pinned engine** (brokkr 0.12.0 refuses a policy without a `review` phase); see issue 6 and 0119 R4 (2026-11-02). Issue 4 is superseded, **drafted, not filed** | the recipe now has phases `trial, review, done, stop` with the exec scoring gate as its protected review; `tests/delivery_recipes.rs` pins; CI, the verify seat and the house rules compile every recipe directory |
| F5 hands for the trial seat | **drafted, not filed** (issue 5) | `binds: []`; `network` true/false; `brokkr_artifacts_unreachable`; paid agents unsupported |
| F6 does the exec box share `/tmp` | **unobserved**; not an issue; the answer goes in `evals/README.md` (admission posture) once the engine run shows it | nothing changes either way |
| F7 a deadline-killed seat is distinguishable | **drafted, not filed** (issue 2, last sentence) | `deadline_killed` is a typed fact; a worker with no row is `harness_error`, never `gave_up` |

### Drafted brokkr issues (verbatim for the PR; filing is the operator's)

Check brokkr#766 and #757 to #765, #467, #462, #430 first; comment on the
sub-issue that already covers an item instead of opening a new one.

**1. Accounting for a `brokkr driver <family>` run as a descendant of an exec
seat (F1).** Swarmail's interim eval recipe runs an exec seat whose script runs
`swarmail-evals trial`, which would run `brokkr driver claude|codex|dsh -- …`
as a child, with the goal on stdin (Claude, Codex) or as the last literal
argument (dsh). Ask: does the journal attribute that run's input, output and
cached tokens, turns and tool rows to the enclosing exec seat, and through
which channel (a stdout relay or one named environment variable)? Four seams
on Swarmail's side sever any channel today (`env -i` in `trial.sh`, the
runner's fresh agent environment, `env_clear` and `/dev/null` output in the
wrapper); Swarmail forwards nothing until brokkr names the channel.

**2. Invocation of `brokkr driver <family>` for an explicit CLI executable
(F2), and the deadline-kill event (F7).** Swarmail needs the documented
invocation form for a caller-supplied executable and argv (it assumes `DRIVER
<family> -- <program> <argv…>` and refuses to run it), the flags the driver adds
to the CLI, and how the goal is passed. Separately, the journal must
distinguish a seat killed by the run's deadline from a seat that returned, so a
killed trial is never read as an agent that gave up.

**3. A per-run export carrying seat results with their `inputs`, usage and tool
rows (F3).** Swarmail joins by the `trial_id` the seat echoes in `inputs`.
Needed per seat: `run_id`, `seat`, `result`, `inputs`, input, output and cached
tokens, turns, wall time, per-tool-call name, outcome class (ok, error,
invalid_argument), argument and result byte lengths and SHA-256, and whether
the seat was killed by its deadline. `journal-facts.sh` isolates the assumed
shape in `SHAPE`/`EXTRACT` and exits 2 (`export_shape_mismatch`) on anything
else.

**4. A policy-owning two-seat recipe (F4).** Confirm that `class: work` on an
exec driver and a policy with phases `trial, score, done, stop` and no intake,
design, implement, verify, review or ship phase compile (`brokkr compile
--bundle .brokkr/recipes/eval`) and finish without a commit.
*Superseded: brokkr 0.12.0 refuses this shape (see issue 6); the recipe now
uses phases `trial, review, done, stop` with the exec scoring gate as its
protected review.*

**5. Hands for an exec seat that runs the candidate's binaries (F5).** Needed:
loopback; the worktree's `target/` and `.brokkr/state/eval/` read-write; for a
paid trial a read-only bind for an operator-chosen private selection file and
the three credential files it names, read-only access to the three CLI
executables and their install trees, and the network; and whether `/tmp` is
private (F6). None is declared today.

**6. A non-delivery trial recipe and the protected review gate (0.12.0).**
With brokkr 0.12.0 (binary sha256 `29dc3309925a2556b9713b7a9b5fb8089b8e4610e86215fc89554e75634e7cad`,
source `a9893b4` plus four pinned patches), `brokkr compile --bundle
.brokkr/recipes/eval` refused a standalone `forge.phase-machine/v2` recipe
with phases `trial, score, done, stop` and no `protected_phase`:
`error: bundle: policy has no 'review' phase; the protected review gate is
non-removable (extension model, layer 1)`. Swarmail's PR #316 proposes the
same recipe with its exec scoring gate named `review` (`class: gate`, exec
driver, results `pass`/`fail`/`unscorable`, one attempt, no agent, no
panel) and `protected_phase: "review"`. Questions: (1) Is that phase a
lawful protected review for a non-delivery recipe, and is the layer-1
requirement (a `review` phase even without `protected_phase`) documented?
(2) Does the engine require of a protected review any result vocabulary,
rule facts, `shippable_from` or panel aggregate, and does it apply them at
compile or at run time? (3) If only a review panel is lawful, may a
trial-only recipe declare itself exempt from layer 1, or what is the
minimal panel shape that needs no paid model call? Until this is
answered, Swarmail ships the shape the engine accepts or keeps `eval` off
the compiled shelf and names this issue in the PR.

*Drafted, not filed (2026-11-02). Observed compile output of the renamed
shape: **not run** in the implement seat, which had no staged pinned binary
(`.brokkr/state/bin/brokkr` absent) and no network. The receipt belongs to a
verify continuation entered from the candidate's bundle or to CI's
`brokkr bundle (pinned)` job; the operator attaches it to this issue when
filing. If that compile refuses the renamed shape, 0119 R4 Ruling 3 applies
and this text is amended with the refusal.*

*Second visit, 2026-11-02 (implement seat, review return): still **not run**,
the artifact was again absent and the box has no network. The slot stays
empty, so this issue is not filed yet; it is filled from AC1's receipt in
`specs/eval-recipe-review-gate/implementation.md` (a staged-binary compile
or the candidate-pinned verify pass note).*

## Acceptance criteria

| AC | Status | Proof |
|---|---|---|
| AC1 one command, one row per slot | met | `report.rs::brokkr_mode_runs_the_product_through_the_recipe_scripts` (2 tasks × {oracle, null} × 2 = 8 rows, 4 pass, 4 fail, distinct seeds, `check --matrix` replay) |
| AC2 worker death is a row | met | `brokkr_mode_turns_every_worker_defect_into_a_row` (no-row, foreign-row, bad-exit, bad-score, no-score, category-stderr), `brokkr_mode_records_a_killed_worker_as_a_harness_error` |
| AC3 recipe is real | **partly**: files, pins and CI loop done; `brokkr compile --bundle .brokkr/recipes/eval` **not run** (no binary); CI refused the original shape, so reworked under 0119 R4 (2026-11-02) | `tests/delivery_recipes.rs` (11 mutants, each caught by its named rule) |
| AC4 scripts on the engine's rendering | met | `recipe.rs` (engine, CRLF and bare forms; first red above) |
| AC5 controls through the recipe path | met | `proof.rs::the_controls_hold_through_the_recipe_path` (oracle 16/16, null 0/16) |
| AC6 typed join | met | goldens, inline `journal.rs` tests, `a_dropped_join_is_red`, untyped refusals |
| AC7 evidence bounded and structural | met | sentinel test, truncation with `retained`, no file for an unavailable fact, `evidence_write_failed` |
| AC8 via-brokkr fails closed | met | wrapper, trial and matrix tests above |
| AC9 deadlines nest | met | `deadline_exceeds_seat`, envelope test, bundle pins 1800/600 |
| AC10 unconfined | met | every recipe-path row asserted `unconfined` |
| AC11 sequential | met | `--concurrency` other than 1 still refuses; the issue's "two concurrent trials" criterion is **not** implemented (isolation is #221's; 0076) |
| AC12 brokkr facts filed | **operator**: drafted, not filed (table above) | |
| AC13 no dependency, schemas unchanged | met | `git diff f815d48 --stat` over `Cargo.*`, `evals/Cargo.*`, `deny.toml`, `scripts/`, `.brokkr/recipes/self/`, `docs/evals/` is empty; no `TrialResult`/`Metadata`/`MatrixPlan`/`MatrixIndex` change |
| AC14 gates | met except `brokkr compile` (not run) | receipts below |
| AC15 delivery paper | this file; 0119 stays proposed with its index row | |
| AC16 paid measurement | **operator, after the follow-up**; cannot happen on this tree | |
| AC17 seams named and proven | met | `recipe.rs::the_trial_seat_strips_the_matrix_environment`, the wrapper environment test |

## Gate receipts (final tree, all `--locked`)

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| `cargo test --workspace --locked` | exit 0 (1231 passed) |
| `cargo build --locked --bin swarmail` | exit 0; `SWARMAIL_BIN` = `target/debug/swarmail` from Cargo's artifact record |
| `cd evals && cargo fmt --all -- --check` | exit 0 |
| `cd evals && cargo clippy --all-targets --locked -- -D warnings` | exit 0 |
| `cd evals && SWARMAIL_BIN=… cargo test --locked` | exit 0 (lib 73, bin 27, recipe 13, report 35, runner 38, provenance 29, others ok) |
| `brokkr compile --bundle .brokkr/recipes/self` and `…/eval` | **not run: no brokkr binary in this seat** |
| `bash scripts/coverage-gate.sh` | exit 0, exact 100/100/100 |
| `bash scripts/coverage-gate.sh selftest` | exit 0 |
| `bash scripts/publish-guard-test.sh` | exit 0 |
| `bash scripts/seal-branch-test.sh` | exit 0 |
| `cargo deny check licenses` | exit 0 |
| `sha256sum --check --strict .brokkr/recipes/self/scripts/shims.sha256` | OK |

The pinned-bundle compile of every recipe, `eval` included, is the AC1 receipt
in `specs/eval-recipe-review-gate/implementation.md` (CI's `brokkr bundle
(pinned)` corroborates it at merge time and owns nothing in-run). Run by CI:
the chart, Docker and RustSec jobs, and the macOS test job.

## PR description (draft; the first paragraph is the contract)

> **Held for the operator: this touches `.brokkr/` and `.github/`, so it merges
> only on the operator's approval by name (0056, 2026-11-01 addendum). The six
> brokkr issues above, issue 6 (the protected review gate) included, are
> drafted, not filed: filing them (or attaching them to the sub-issue that
> covers each) and recording the numbers is a precondition of merging, owned
> by the operator. Issue 6 is not filed while its bracketed slot is empty. This delivery does not measure tokens per
> success: the accounting channel is severed at four named seams and forwarding
> it is a follow-up under 0119 section 6, which only the operator can accept.
> Paid agents under `matrix --brokkr` are unsupported until the F2/F1/F5
> follow-up; the direct `matrix` stays the paid route. 0119 is proposed, not
> accepted, and value-bearing evidence waits on it. The first engine run
> (oracle and null, offline, free) and the first authenticated via-brokkr smoke
> per family are the operator's, and they observe F1, F2 and F5. AC16 is the
> operator's paid run after the follow-up. Trials stay sequential (0076);
> isolation and the "two concurrent trials" criterion are #221's. The decision
> number history: `main` took 0115 (#308) and 0116 (#313), and this record was
> renumbered 0116 → 0119 on 2026-11-02. The pinned-engine compile of every
> recipe (AC1 of `specs/eval-recipe-review-gate/`) has **not** run in-run: the
> receipt is pending, by a staged-binary compile or by the candidate-pinned
> verify pass note, and CI's `brokkr bundle (pinned)` corroborates it. The
> verify seat's broader compile takes effect for runs whose pinned bundle
> includes it.**

## What is the operator's

1. File or attach the six drafted brokkr issues, issue 6 (the protected review
   gate) included, and record the numbers. Do not file issue 6 while its
   bracketed slot is empty: the slot is filled from the AC1 receipt
   (`specs/eval-recipe-review-gate/implementation.md`).
2. The first `brokkr run --recipe eval` with the engine (oracle and null,
   offline, free): the first proof of F4, F6 and the prompt rendering.
3. The first authenticated smoke per family: observes F1, F2 and F5.
4. A captured real `brokkr export` to replace the stand-in in
   `journal-facts.sh`'s test, and any change to `SHAPE`/`EXTRACT` it needs.
5. Accepting or amending 0119 (section 6 forwarding; section 4 value-bearing
   evidence, an amendment to 0075 section 4, **not** implemented), and
   renumbering it if `main` takes 0119 first.
6. The follow-up change (forward the observed channel, lift the refusal,
   declare the binds) and then the paid matrix (AC16).

## Round 3 (T34 to T42; run `issue-148-the-eval-recipe-on-bro-1b9b64e9`)

Built on `6f5b1c3` (spec revision 3), never amended. The gate now agrees with
the row by the agreement table (spec "Revision 3"), not by equality.

| Task | Change | Proof |
|---|---|---|
| T34 | `brokkr-standin.sh`: modes `budget-over-pass`, `harness-error-over-pass`, `budget-over-unscorable` rewrite the slot's `row.json`, `exit`, the retained `result.json` and its digest in `metadata.json` (as `provenance.rs`'s `rebind` does; no check relaxed), `budget-over-unscorable` also cuts `state.json` without rebinding; `bad-score` retargeted to `business`; the log gains one `score=<word>` line per invocation | `report.rs::brokkr_mode_commits_a_budget_row_over_a_passing_state` (3 `fail`/`budget` rows, exit 1, `score=fail` x3), `…_a_harness_error_row_over_a_passing_state` (3 `harness_error` rows as written, exit 2, `score=unscorable` x3, no `score_mismatch`), `brokkr_mode_turns_every_worker_defect_into_a_row` (now seven arms) |
| T35 | one `score.sh` case per table cell over rewritten oracle/null leaves; unreadable rows (missing, symlink, two lines) | `recipe.rs::the_score_script_follows_the_agreement_table`, `the_score_script_maps_check_exit_codes` |
| T36 | `score.json` is `{schema_version, trial_id, business, check_exit}`; `score.sh` reads `row.json` strictly (regular, no symlink, <= 64 KiB, one line, `jq -e` on `outcome`/`failure_class` only) and reports by the table | the T35 cases; the existing "notes carry no path" assertion holds for each |
| T37 | `GateVerdict::Ruled(Business)`; `gate_verdict` reads `business` only (unknown values, a missing key and the revision-2 `outcome` key are `Missing`); `gate_refusal(&GateVerdict, &TrialResult)` matches the table; `outcome_name` and its test removed | inline `the_gate_agrees_with_the_row_by_the_agreement_table` (every cell), `a_slot_is_read_through_the_bounded_reader_and_missing_pieces_are_typed` |
| T38 | `policy.json`: description and the three `SCORE-*` reason texts restated; ids, results, transitions, severities unchanged, no rule added | `tests/delivery_recipes.rs` unchanged and green (it pins no reason text) |
| T39 | `evals/README.md` carries the table; the tokens sentence names the F1/F2 forwarding follow-up and 0119 §6 and says a paid run alone closes neither #151 gap; `.brokkr/recipes/README.md` describes the score seat by the table | the grep checks below |
| T40 | `through_driver` is `#[cfg(test)]`; `run()`'s driver arm is deleted; `evals/wrappers.md` says so | non-test `cargo build --locked` and clippy clean; the inline assembly and refusal tests unchanged and green |
| T41 | `the_trial_seat_strips_the_matrix_environment` replaced by `the_trial_script_strips_the_matrix_environment` (seam 1: probe `evals_bin` under the real `trial.sh`) and `the_runner_rebuilds_the_agents_environment` (seam 2: `swarmail-evals trial` with the sentinel in its own environment); no runtime claim for seams 3 and 4 | the mutation below |

### First red, observed (T34, T35)

Run against `040795d`'s code with the new tests (and the stand-in's retargeted
`bad-score`):

* `budget-over-pass` and `harness-error-over-pass`: red. Slot 1 became
  `score_mismatch` (stderr `matrix trial error: score_mismatch`), admission
  stopped after one worker (`left: 1, right: 3`) and the run exited 2.
* `bad-score` after retargeting: red (`left: Some(0)`, `right: Some(2)`): the
  old `score.sh` had no `business` key to disagree with.
* `the_score_script_maps_check_exit_codes`: red (`score.json` carried `outcome`
  and `exit`, not `business` and `check_exit`).
* `the_score_script_follows_the_agreement_table`: red at its first cell for the
  same reason. The `fail`/`budget`-over-`pass` cell reported `pass` under the
  old script (it echoed the replay), which the report-level arm shows as above.

All green after T36 to T38.

### The T41 mutation

With `env -i PATH=… LANG=C LC_ALL=C` deleted from `trial.sh` (so the line began
`env SWARMAIL_BIN=…`), `the_trial_script_strips_the_matrix_environment` was red:
`an inherited key survived env -i: BROKKR_HANDS_BOX, CARGO, CARGO_HOME, …`.
`trial.sh` was restored byte for byte (`git diff` of it is empty) and the
mutation was never committed.

### Gate receipts (final tree, all `--locked`, offline)

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| `cargo test --workspace --locked --no-fail-fast` | exit 0, every binary ok |
| `cargo build --locked --bin swarmail` | exit 0; `SWARMAIL_BIN` = `target/debug/swarmail` from Cargo's artifact record |
| `cd evals && cargo fmt --all -- --check` | exit 0 |
| `cd evals && cargo clippy --all-targets --locked -- -D warnings` | exit 0 |
| `cd evals && cargo build --locked` (non-test: `through_driver` is test-only) | exit 0 |
| `cd evals && SWARMAIL_BIN=… cargo test --locked --no-fail-fast` | exit 0 (lib 73, bin 27, recipe 15, report 37, runner 38, provenance 29, wrappers 22, proof 8, baseline 3, identity 5, boundary 19, fixture 19, scoring 18) |
| `brokkr compile --bundle .brokkr/recipes/self` and `…/eval` | **not run: no brokkr binary in this seat** (CI's pinned-bundle job compiles both) |
| `bash scripts/coverage-gate.sh` | exit 0, exact 100/100/100 |
| `bash scripts/coverage-gate.sh selftest` | exit 0 |
| `bash scripts/publish-guard-test.sh` | exit 0 |
| `bash scripts/seal-branch-test.sh` | exit 0 |
| `cargo deny check licenses` | exit 0 |
| `git diff --stat f815d48 -- Cargo.toml Cargo.lock evals/Cargo.toml evals/Cargo.lock deny.toml scripts/ .brokkr/recipes/self/` | empty |

**A flake, recorded and not retried to green.** The first full evals run
failed `tests/fixture.rs::focused_tools_complete_link_and_code_traps_without_guessing`
(`digest-forty` returned `status: timed_out` with the arrival in
`near_misses`) while the host was building and running the whole suite. It is
a wait-timing test in code this change does not touch. Run alone it passed 3 of
3, and the next full `--no-fail-fast` run passed it. The #302 test
(`child_output_server_death_remains_a_harness_error_on_linux`) did not recur.

Checks from the plan: `test ! -e evals/tests/wrappers_launch/standin-driver.py`
holds; `grep -n 'through_driver(&cmd' evals/src/wrappers.rs` finds nothing;
`grep -n "until the operator's paid run" evals/README.md` finds nothing;
`grep -n "disagrees with the row" evals/README.md .brokkr/recipes/README.md`
finds nothing; `grep -n "0119 §6" evals/README.md` finds the tokens sentence.

PR body, one added sentence: the gate agrees with the row by the agreement
table, and a `fail`/`budget` trial is recorded as data. The hold, the
unfiled-brokkr-issues precondition and the paid-run limits above are unchanged.
