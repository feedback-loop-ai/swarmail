# Implementation record: the eval recipe on brokkr (interim path)

Implements [spec.md](spec.md) / [plan.md](plan.md) / [tasks.md](tasks.md) under
proposed [0116](../../docs/decisions/0116-eval-brokkr-recipe.md). Base `main`
`f815d48`. Commits unsigned; nothing pushed, merged, published or accepted.

**This pull request touches `.brokkr/` and `.github/`, so it merges only on the
operator's approval by name (decision 0056, 2026-11-01 addendum).**

## What shipped

| Area | Change |
|---|---|
| `.brokkr/recipes/eval/` | `bundle.json` (two exec seats, `trial` work / `score` gate, 1800 s / 600 s, `max_attempts` 1, `score` has no network), `policy.json` (five rules, no retry), `scripts/trial.sh`, `scripts/score.sh` |
| `evals/src/main.rs` | `matrix --brokkr BIN`: one `brokkr run --recipe eval` per slot; `trial_args` is the one argument list both workers use; slot request, slot reading, exit pairing, gate cross-check; `report --journal/--evidence-dir`; `--via-brokkr` on the three `agent-*` commands |
| `evals/src/journal.rs` (new) | strict facts v1 reader, join by trial id, per-group summary, bounded structural evidence writer |
| `evals/src/wrappers.rs`, `provenance.rs` | `--via-brokkr` exec target; three `*_via_brokkr` selectors with one driver residual |
| `evals/src/report.rs`, `artifacts.rs`, `lib.rs` | `report_with_journal` (`report` delegates, output unchanged); `slots` allowed as a private directory name; `pub mod journal` |
| `evals/scripts/journal-facts.sh` (new) | facts v1 from a brokkr export (see F3) |
| Tests | `evals/tests/recipe.rs` (new), additions to `report.rs`, `proof.rs`, `wrappers_launch/mod.rs`, `common/mod.rs`; `tests/data/brokkr-standin.sh`, `journal-facts-*.json`, `wrappers_launch/standin-driver.py`; inline tests in `journal.rs`, `provenance.rs`, `main.rs`; root `tests/delivery_recipes.rs` (pin test with one-fault mutants, shelf list) and `tests/delivery_constitution.rs` (CI loop is seven recipes) |
| CI / docs | `eval` appended to the pinned-bundle compile loop; `.brokkr/recipes/README.md`, `evals/README.md`, `evals/wrappers.md`, decision index row for 0116 |

No file under root `src/`, `scripts/`, `.brokkr/recipes/self/`, `Cargo.*`,
`evals/Cargo.*`, `deny.toml` or `docs/evals/baseline-2026-11-01*` changed.
`TrialResult`, `Metadata`, `MatrixPlan` and `MatrixIndex` are untouched;
`WrapperProfile` gained exactly the three `*_via_brokkr` entries.

## T1: brokkr reachability and the F1–F7 table

`.brokkr/state/bin/brokkr` is **absent** in this seat and there is no network:
the pinned binary (`tooling.json`, 0.12.0, `binary_sha256` `29dc3309…cad`) was
not run, brokkr#766 and its sub-issues were not read, and `brokkr compile
--bundle .brokkr/recipes/eval` was **not run** (recorded as not run, never as
passed). No F-fact below is "observed" and none is claimed "tracked": I could
not read the sub-issues. Nothing was filed on brokkr from here (no network);
the issue text is **drafted below for the operator to file** or to attach to
the sub-issue that already covers it.

| Fact | Status | What ships regardless |
|---|---|---|
| F1 driver run as a descendant of an exec seat records tokens, turns, tool rows | **not observed** | `--via-brokkr`, proven with the stand-in driver; if the channel is an environment variable, the wrapper forwards nothing extra today (no key invented) and 0116 records the exception by dated addendum |
| F2 driver accepts the wrapper's goal and an explicit CLI executable | **not observed** | wrapper builds `DRIVER <family> -- <program> <argv...>`; the real driver's flags are unverified. `driver_contract_unknown` (spec's typed refusal for a known-bad driver) is **not implemented**: there is no observed contract to refuse against, and inventing one would be a guess |
| F3 per-run machine-readable query with seat inputs, usage, tool rows, deadline kill | **not observed** | facts v1 reader and goldens; `journal-facts.sh` isolates the one assumed export shape in `EXTRACT`; its test uses a hand-authored stand-in export, named as such. A real export that does not match yields `journal_missing`, visibly, never a number |
| F4 two-seat recipe with its own policy compiles and runs without a commit | **not observed** | files as specified; CI's loop compiles it; `tests/delivery_recipes.rs` pins the shape |
| F5 hands reach loopback, candidate binaries, the slot (rw), the operator's selection file (ro) | **not observed** | `binds: []`, `network` true on `trial` and false on `score`. The ro bind for the operator's private selection file is **not declared** (its path is the operator's); the first paid run must add it or the engine will refuse the path. Filed below |
| F6 does the exec box share the host `/tmp` | **not observed** | nothing changes either way; the posture is stated in `evals/README.md`; record the answer there once seen |
| F7 a seat killed by the run deadline is distinguishable in the journal | **not observed** | `deadline_exceeds_seat` keeps brokkr's timer a backstop; a worker with no row is `harness_error`, never `gave_up` |

### Drafted brokkr issues (not filed)

Check brokkr#766 and #757–#765, #467, #462, #430 first; comment there if one
already covers the item.

1. **Accounting for a `brokkr driver <family>` run as a descendant of an exec
   seat (F1, F2).** Swarmail's interim eval recipe runs an exec seat whose
   script runs `swarmail-evals trial`, which runs `brokkr driver
   claude|codex|dsh -- <program> <argv...>` as a child, with the goal on stdin
   (Claude, Codex) or as the last literal argument (dsh). Ask: does the journal
   attribute that run's input/output/cached tokens, turns and tool rows to the
   enclosing exec seat; through which channel (stdout relay or one named
   environment variable); and which flags does the driver add to the CLI?
2. **A per-run export that carries seat results with their `inputs`, usage and
   tool rows (F3).** Swarmail joins by the `trial_id` the seat echoes in
   `inputs`. Needed per seat: `run_id`, `seat`, `result`, `inputs`, input/output/
   cached tokens, turns, wall time, per-tool-call name, outcome class
   (ok/error/invalid-argument), argument and result byte lengths and SHA-256, and
   whether the seat was killed by its deadline (F7).
3. **A policy-owning two-seat recipe (F4).** Confirm `class: work` on an exec
   driver and a policy with phases `trial, score, done, stop` and no
   intake/design/implement/verify/review/ship phase compile and finish without
   a commit; Swarmail's `eval` recipe is the case.
4. **Hands for an exec seat that runs the candidate's binaries (F5).** Needed:
   loopback, the worktree's `target/` and `.brokkr/state/eval/` (rw), a read-only
   bind for an operator-chosen private file outside the worktree, network for
   paid runs; and an answer to whether `/tmp` is private (F6).

## Acceptance criteria

| AC | Status | Proof |
|---|---|---|
| AC1 one command, one row per slot | met | `report.rs::brokkr_mode_runs_the_product_through_the_recipe_scripts` (2×{oracle,null}×2 = 8 rows, 4 pass, 4 fail, distinct seeds, `check --matrix` replay) |
| AC2 worker death is a row | met | `brokkr_mode_turns_every_worker_defect_into_a_row` (no-row, foreign-row, bad-exit, bad-score), `brokkr_mode_records_a_killed_worker_as_a_harness_error` (SIGKILL) |
| AC3 recipe is real | **partly**: files, pins and CI loop done; `brokkr compile --bundle .brokkr/recipes/eval` **not run** (no binary) | `tests/delivery_recipes.rs::eval_recipe_is_two_exec_seats_with_one_attempt_and_five_rules` (8 mutants), `delivery_constitution.rs` (seven-recipe loop). The verify seat compiles only `self` today, like the other five recipes; `.brokkr/recipes/self/` is deliberately untouched, so "the verify list" gains nothing |
| AC4 scripts behave without the engine | met | `recipe.rs` (trial/score happy paths, 9 malformed requests, no clobber, exit mapping, truncated state → `unscorable`, no path in notes) |
| AC5 controls through the recipe path | met | `proof.rs::the_controls_hold_through_the_recipe_path` (oracle 16/16, null 0/16 with only `gave_up`/`wait_timeout`) |
| AC6 typed join | met | `report.rs` goldens, `a_dropped_join_is_red`, refusals; inline `journal.rs` tests |
| AC7 evidence bounded and structural | met | `report.rs` sentinel, truncation, no-clobber, requires-journal tests; modes 0600/0700 |
| AC8 via-brokkr launches | met with stand-ins | `wrappers_launch/mod.rs` (argv byte-for-byte for all three families, invalid-driver refusal before launch, trial pass and `gave_up` through the stand-in driver) |
| AC9 deadlines nest | met | `brokkr_mode_refuses_…` and the bundle pin (1800/600) |
| AC10 unconfined | met | every recipe-path row asserted `unconfined`; report note unchanged; bundle description and 0116 say so |
| AC11 sequential | met | `--concurrency` other than 1 still refuses in this mode; the "two concurrent trials" criterion is **not** implemented (isolation is #221's; 0076) |
| AC12 brokkr facts | **open**: table above; nothing observed, nothing filed |
| AC13 no dependency, schemas unchanged | met | `git diff` over `Cargo.*`, `evals/Cargo.*`, `deny.toml`, `scripts/`, `.brokkr/recipes/self/` is empty |
| AC14 gates | met except `brokkr compile` (see below) | receipts below |
| AC15 delivery paper | this file, 0116 stays proposed, index row added |
| AC16 paid run | **operator's only**; not in delivery | |

## Judgment calls where the spec was silent

* `request.json`'s `args` is the list **after** the `trial` subcommand (`trial.sh`
  runs `swarmail-evals trial "${args[@]}"`); the first draft duplicated it and
  the matrix test caught it.
* `create_private_dir` accepts the fixed name `slots` beside `trials`, so the
  slot directories sit under the matrix root the operator already chose.
* Aggregates run over **scored** (non-harness-error) trials; harness errors are
  counted apart (`excluded_harness_errors`). `total` tokens is input + output;
  `cached` is reported beside it because providers disagree on whether input
  already includes it. A run id is cited only for a journal-accounted entry.
  Evidence is written only for accounted entries; an empty tool list from an
  unaccounted run would read as "zero calls".
* `brokkr_realm_invalid` also covers a relative or non-executable `--brokkr`.
* A freshly created `--evidence-dir` root is made 0700; an existing root keeps
  the operator's mode.
* A missing or malformed `score.json` is `score_mismatch`: the gate must have
  ruled on this row.
* The slot's `trial.stderr` is scanned for the same three fixed categories the
  direct worker's stderr is, so 0076's admission accounting is the same in both
  modes.

## Gate receipts (final tree, all `--locked`)

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| `cargo test --workspace --locked` | exit 0 (520 library tests; all suites ok) |
| `cargo build --locked --bin swarmail` | exit 0; `SWARMAIL_BIN` = `target/debug/swarmail` from Cargo's artifact record |
| `cd evals && cargo fmt --all -- --check` | exit 0 |
| `cd evals && cargo clippy --all-targets --locked -- -D warnings` | exit 0 |
| `cd evals && SWARMAIL_BIN=… cargo test --locked` | exit 0 (69 lib, 25 bin, recipe 8, report 32, proof 8, wrappers 24, runner 38, provenance 29, others ok) |
| `brokkr compile --bundle .brokkr/recipes/self` and `…/eval` | **not run: no brokkr binary in this seat** |
| `bash scripts/coverage-gate.sh` | exit 0, exact 100/100/100 |
| `bash scripts/coverage-gate.sh selftest` | exit 0 |
| `bash scripts/publish-guard-test.sh` | exit 0 |
| `bash scripts/seal-branch-test.sh` | exit 0 |
| `cargo deny check licenses` | exit 0 |
| `sha256sum --check --strict .brokkr/recipes/self/scripts/shims.sha256` | OK |

Not run here, run by CI: the pinned-bundle compile (including `eval`), the
chart, Docker and RustSec jobs, and the macOS test job. The root gates ran
once on the final tree; the T1 "baseline before the change" pass was not
separately recorded.

## What is the operator's

1. The first `brokkr run --recipe eval` with the engine (oracle and null,
   offline, free): this is also the first proof of F4–F7.
2. The first authenticated `--via-brokkr` smoke per family (F1, F2).
3. A captured real `brokkr export` to replace the stand-in in
   `journal-facts.sh`'s test, and any change to `EXTRACT` it needs.
4. The paid matrix (AC16), which is what closes the #151 "Not measured" rows.
5. Accepting or amending 0116, including the value-bearing evidence mode
   (an amendment to 0075 §4), which is **not** implemented.
6. Filing or attaching the four drafted brokkr issues; renumbering 0116 if
   `main` takes it before this merges.
