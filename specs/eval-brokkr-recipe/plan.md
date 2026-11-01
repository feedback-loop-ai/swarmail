# Implementation plan: the eval recipe on brokkr (interim path)

Implements [spec.md](spec.md). Declares proposed
[0116](../../docs/decisions/0116-eval-brokkr-recipe.md). Compatible work
under accepted 0045 §8: nothing here contradicts an accepted ruling; the one
part that would (value-bearing evidence, 0075 §4) is specified in 0116 and
deliberately not implemented. The chief commits spec.md, plan.md, tasks.md
and the decision record only.

## Technical context

- `evals/` is a standalone crate outside the root coverage denominator
  (accepted 0055); every new branch still ships with a test. No file under
  root `src/` changes, so the exact 100% root gate is unaffected.
- New production surface, all in `evals/`: `src/journal.rs` (facts reader,
  join, evidence writer), additions to `src/main.rs` (`--brokkr` on
  `Matrix`, `--journal` and `--evidence-dir` on `Report`, `--via-brokkr` on
  the three `Agent*` commands), `src/wrappers.rs` (the exec target),
  `src/provenance.rs` (three `WrapperProfile` variants and their
  `ToolSurface` rows), `src/report.rs` (the `journal` object).
- New recipe: `.brokkr/recipes/eval/{bundle.json,policy.json,scripts/trial.sh,scripts/score.sh}`.
- New host-side script: `evals/scripts/journal-facts.sh` (bash + jq).
  `scripts/` at the root is the pinned-shim directory and is **not** touched.
- Tests: `evals/tests/recipe.rs` (new), `evals/tests/report.rs`,
  `evals/tests/proof.rs`, `evals/tests/wrappers.rs` plus
  `evals/tests/wrappers_launch/standin-driver.py` and a stand-in worker
  program (the `worker_program` pattern already in `main.rs` tests, made a
  test helper), `tests/delivery_recipes.rs` (root).
- CI: one word (`eval`) in `.github/workflows/ci.yml`'s pinned-bundle loop.
- No dependency: `std`, `serde`, `serde_json`, `ring` (SHA-256, already a
  dependency) and `libc` cover everything. `jq` is already required by the
  realm's exec scripts.
- brokkr is not reachable from the design seats. The implement seat first
  checks `.brokkr/state/bin/brokkr` against `tooling.json`'s digest; if
  present it observes F1–F7 and captures a real export beside the
  hand-authored golden fixtures; if absent it records that in
  `implementation.md`, writes `journal-facts.sh` against the recipe guide's
  documented `brokkr export`, and leaves the capture and the engine run to
  the operator's receipt, as the spec's F-table says.
- `.specify/templates/` holds no realm template; this plan supplies context,
  constitution check, structure, research, data model, contracts, quickstart
  and risks.

## Constitution check

- 0001–0005: untouched. The fixture freezes and snapshots through its admin
  API; the harness never prunes, never changes the cap, never re-parses mail.
- 0020: deterministic join, quantile and evidence logic is inline
  `#[cfg(test)]` in `journal.rs` and `report.rs`; wire-shaped tests (real
  `trial`, real scripts, real stand-in worker) live in `evals/tests/`; no
  sleep as synchronization (the stand-in worker signals readiness by writing
  a file the test waits on with a real deadline); fresh tempdir, fresh seeds,
  no shared mutable state; chaos is not involved.
- 0045 §8: compatible; 0116 is drafted, not accepted; the value-bearing
  evidence mode is not built.
- 0050 §1: the pinned shims and `shims.sha256` do not change; the recipe's
  scripts are the recipe's own, resolved against its directory as
  `claude-careful`'s design check is.
- 0055 §§6–10, 0063, 0071, 0076: rows, artifacts, scoring precedence,
  traps, replay, concurrency and admission are unchanged; the matrix's
  brokkr mode is one more worker command behind the same accounting.
- 0056 addendum: the PR touches `.brokkr/` and `.github/`, so it is held for
  the operator's approval by name; commits unsigned; never push.
- 0075: every child still starts with `env_clear`; the wrapper's via-brokkr
  exec adds no environment key unless F1 names exactly one, which 0116 then
  records; no raw CLI bytes are retained; `trial.stderr` in the slot is
  bounded and is the trial's own diagnostics stream, which the trial already
  keeps to fixed categories (0075 §4).
- 0098/0099/0102: this run's route; no recipe other than `eval` changes.

## Research and chosen approach

### Why the worker is `brokkr run`, not a hand-rolled supervisor

The matrix already owns everything a supervisor would re-implement: the
worker's process group, the deadline envelope, once-only signal
forwarding, bounded drains, the EOF bound, one-row-per-slot accounting,
`stop_admission` and the refusal suffix. In brokkr mode only the command
and the row's source change (`<slot>/row.json` instead of stdout). The
robustness position's "re-implements that ownership one layer up and gets
it subtly wrong" is answered by not adding a layer.

### Why the join key is `trial_id` through the seat result

`assigned_cell` fixes `trial_id` before any spawn. It travels in
`request.json` → `trial.sh` → the seat's typed `inputs.trial_id` → brokkr's
journal → the export → the facts file. Nothing is discovered after the
fact; a journal with eight concurrent delivery runs cannot be joined wrongly
because a delivery run's seats carry no `trial_id` input, and a second eval
run carrying the same id makes the join `journal_ambiguous` rather than
picking one.

### Why rows stay unchanged

`TrialResult` is `deny_unknown_fields` and read by `results::read` under the
64 KiB cap, by `artifacts::trial` (bound by `metadata.json`), by the matrix
worker context check and by the report. Adding a field means a schema
decision (0076 §5 settled the last one), a historical reader, and a
re-publication of the 2026-11-01 baseline. The facts file beside the rows
gives every number the issue asks for and leaves 432 published rows
readable as they are.

### Why evidence is structural now and value-bearing later

Accepted 0075 §4 suppresses raw bodies and URLs at every publication sink.
Tool names, outcome classes, byte lengths and digests are harness-authored
finite facts. Argument values of `eval_http` are URL paths and bodies. The
first is compatible and lands; the second is an amendment and waits. Both
positions get what they asked for in the order the rulings allow.

### Why the gate is cross-checked

`check --artifacts-dir <leaf>` already validates the bound evidence and
re-scores it. Running it as the recipe's `score` seat satisfies the issue's
"exec script that reads the app's state and never the agent's account". The
matrix compares `score.json.outcome` with the row: a disagreement can only
mean the leaf or the row was altered between the trial's freeze and the
gate, which is exactly the unconfined residual worth surfacing as
`harness_error` rather than averaging into a success rate.

### The stand-ins (no engine, no network, no payment)

- **Stand-in worker** (`evals/tests/common/brokkr_standin.rs` building a
  small program, or a bash script under `evals/tests/data/`): accepts
  `run --recipe eval --feature "swarmail-eval-request: PATH"`, synthesizes
  a prompt file with a result path and the request line, runs
  `bash .brokkr/recipes/eval/scripts/trial.sh PROMPT`, reads the result,
  and on `ran` runs `score.sh` the same way; exits 0. Modes selected by an
  environment variable the test sets: `no-row` (exit before trial.sh),
  `foreign-row` (rewrite `trial_id` in `row.json`), `bad-exit` (write `exit`
  as 1 for a pass row), `bad-score` (rewrite `score.json.outcome`),
  `hang-after-start` (write a readiness file, then sleep until killed; the
  test SIGKILLs it after the readiness file appears).
- **Stand-in driver** (`evals/tests/wrappers_launch/standin-driver.py`):
  argv `<family> -- <argv...>`; records its full argv and environment keys
  to a file named by `EVAL_TEST_DRIVER_RECORD`; then execs
  `$EVAL_TEST_EVALS_BIN agent oracle` (mode `oracle`) or exits 0 (mode
  `idle`). It reads the eight EVAL inputs from the environment the wrapper
  passes, exactly as a real CLI would.

## Project structure: files touched

| File | Change |
|---|---|
| `.brokkr/recipes/eval/bundle.json` | new: two exec seats (`trial`, `score`), limits 1800/600 s, `max_attempts: 1`, hands per F5 |
| `.brokkr/recipes/eval/policy.json` | new: phases `trial, score, done, stop`; five rules; no retry |
| `.brokkr/recipes/eval/scripts/trial.sh` | new: prompt parsing, request validation, runs `swarmail-evals trial`, writes `row.json`, `exit`, `trial.stderr`, result `ran`/`harness_error` |
| `.brokkr/recipes/eval/scripts/score.sh` | new: prompt parsing, `check --artifacts-dir`, writes `score.json`, result `pass`/`fail`/`unscorable` |
| `.brokkr/recipes/README.md` | `eval` section |
| `.github/workflows/ci.yml` | `eval` appended to the pinned-bundle compile loop |
| `tests/delivery_recipes.rs` | `eval` on the shelf list; new pin test for the eval bundle and policy |
| `evals/src/journal.rs` | new: facts v1 types (strict), `join(rows, facts) -> Joined`, per-group aggregates, `write_evidence` |
| `evals/src/report.rs` | `report_with_journal(rows, Option<&Joined>)`; `report(rows)` unchanged and delegating |
| `evals/src/main.rs` | `Matrix { brokkr: Option<PathBuf> }`, `Report { journal, evidence_dir }`, `Agent* { via_brokkr }`; brokkr worker branch in `run_child_assigned`; request writing; slot reading |
| `evals/src/wrappers.rs` | `run(harness, model, config, via_brokkr: Option<&Path>)`: exec target swap, `driver_invalid` refusal |
| `evals/src/provenance.rs` | three `WrapperProfile` variants, `family()`, `name()`, `validate_model`, surface rows with the driver residual |
| `evals/src/lib.rs` | `pub mod journal;` |
| `evals/scripts/journal-facts.sh` | new: pinned-brokkr query → facts v1 |
| `evals/tests/recipe.rs` | new: the two scripts under bash (AC4) |
| `evals/tests/report.rs` | brokkr-mode matrix tests (AC1, AC2, AC9, AC10), journal goldens (AC6), evidence sentinel (AC7) |
| `evals/tests/proof.rs` | brokkr-mode controls arm (AC5) |
| `evals/tests/wrappers.rs`, `evals/tests/wrappers_launch/standin-driver.py` | via-brokkr assembly and trials (AC8) |
| `evals/tests/common/mod.rs` | stand-in worker builder, recipe script paths |
| `evals/tests/data/journal-facts-golden.json`, `…-sentinel.json`, `…-unmatched.json` | fixtures (hand-authored to the v1 shape; a captured real export is added beside them when the binary is reachable, named as such) |
| `evals/README.md`, `evals/wrappers.md` | commands, selectors, admission posture |
| `docs/decisions/README.md` | proposed 0116 row |
| `docs/decisions/0116-eval-brokkr-recipe.md` | the chief's record (committed with this plan) |

Not touched: root `src/`, `scripts/`, `.brokkr/recipes/self/`, every other
recipe, `Cargo.*`, `evals/Cargo.*`, `deny.toml`, the row and artifact
schemas, `docs/evals/baseline-2026-11-01.md`.

## Data model

- **Request v1** (`<slot>/request.json`, written by the matrix, read by the
  scripts): `schema_version: 1`, `trial_id`, `slot`, `trials_root`,
  `evals_bin`, `swarmail_bin`, `args: [string]`. ≤ 64 KiB. Absolute paths
  only.
- **Slot outputs**: `row.json` (one schema-2 row), `exit` (one decimal),
  `trial.stderr` (≤ 64 KiB), `score.json` (`schema_version: 1`, `trial_id`,
  `outcome`, `exit`).
- **Journal facts v1**: as in the spec; `Availability`-shaped fields
  (`status: present|unavailable`), closed reasons, closed tool outcomes;
  strict reader.
- **Report `journal` object**: as in the spec; `tokens_per_success` is a
  number or `null` plus `reason`.
- **Evidence record**: `{"schema_version":1,"trial_id":…,"run_id":…,"truncated":bool,"tool_calls":[…]}`,
  ≤ 64 KiB, bound by `{path, length, sha256}` in the report.

## Contracts

- CLI: `matrix --brokkr PATH` (new option; requires `--artifacts-dir`;
  refusals `brokkr_requires_artifacts`, `brokkr_realm_invalid`,
  `deadline_exceeds_seat`, row-less exit 2 as for other invalid matrix
  options). `report --journal PATH [--evidence-dir PATH]` (refusals
  `journal_invalid`, `journal_unmatched_trial`, `journal_duplicate_trial`,
  `evidence_requires_journal`; exit 2, no partial report).
  `agent-<family> --via-brokkr PATH` (refusal `driver_invalid`,
  `driver_contract_unknown`). `--wrapper-profile` gains three names. No exit
  code changes elsewhere; stderr keeps fixed categories only.
- Recipe seat results: `trial` → `ran` with `inputs.trial_id`, or
  `harness_error` with empty inputs; `score` → `pass`, `fail`, `unscorable`,
  empty inputs. Exactly `result`, `inputs`, `notes`.
- Library: `journal::read(&[u8]) -> Result<Facts, String>`,
  `journal::join(&[TrialResult], &Facts) -> Result<Joined, String>`,
  `journal::write_evidence(&Path, &Joined) -> EvidenceIndex`,
  `report::report_with_journal(&[TrialResult], Option<&Joined>) -> Value`.
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
(cd evals && SWARMAIL_BIN="$SWARMAIL_BIN" cargo test --locked --test recipe --test report --test proof --test wrappers)
brokkr compile --bundle .brokkr/recipes/self
brokkr compile --bundle .brokkr/recipes/eval      # verify seat and CI; "not run" if no binary in the implement seat
bash scripts/coverage-gate.sh
bash scripts/coverage-gate.sh selftest
bash scripts/publish-guard-test.sh
bash scripts/seal-branch-test.sh
cargo deny check licenses
git diff --stat main -- Cargo.toml Cargo.lock evals/Cargo.toml evals/Cargo.lock deny.toml scripts/ .brokkr/recipes/self/   # empty
```

The operator's first engine run, offline and free, after merge:

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
| `brokkr driver <family>` cannot run as an opaque child (needs seat context), or does not relay the goal as the wrapper supplies it (F1, F2). | The flag is proven with the stand-in driver; the real driver's first use is the operator's smoke; a contract gap is filed on brokkr and named; the wrapper refuses by a typed category rather than guessing flags. Tokens then stay typed `journal_has_no_accounting`. |
| The export shape differs from what `journal-facts.sh` expects (F3). | The Rust reader reads only the v1 facts shape this spec owns and is golden-tested on it; the bash extractor is the only thing that touches brokkr's shape, is small, and is fixed without a Rust change. The PR says whether it was verified against real bytes. |
| The recipe compiles but the engine refuses a work-class exec seat or a policy without the delivery phases (F4). | CI's loop and the verify seat compile it before merge; the run path is proven with the stand-in worker; an engine refusal is filed, not patched around. |
| The exec box cannot reach the candidate's binaries or the slot (F5). | The slot root is inside the candidate's ignored `.brokkr/state/`; binds follow the recipe guide; the first unreachable path is filed. |
| The trial's advisory admission is blind inside a private `/tmp` (F6). | Named residual; the matrix is sequential; the operator runs nothing else, as for the baseline. Recorded in the README once observed. |
| A worker killed by brokkr's seat deadline looks like an agent failure. | The matrix refuses deadlines above 1 200 000 ms so the trial's own deadline and the runner's fixed budgets fit under 1800 s; a worker with no row is `harness_error` by construction (AC2). |
| A hostile unconfined agent rewrites `row.json` or `score.json` in its slot. | The row must match the planned cell and worker context, pair with `exit`, agree with the gate and be bound by `metadata.json`; a disagreement is `harness_error` and stops admission. The residual is named for #221 and the broker. |
| Evidence leaks a secret. | No argument or result bytes are copied, ever; names are bounded printable ASCII or digests; the sentinel test plants the admin secret and a token in both argument and result positions and asserts absence from every retained byte. |
| A partial join biases tokens per success. | `incomplete_join` makes the figure `null` whenever any trial in the group lacks token facts; `no_passes` when passes are zero; never `0`. |
| The facts file is from another matrix. | An entry whose trial id is in no row refuses the whole input. |
| The matrix's brokkr branch drifts from the direct branch. | Both build `args` from one function; the direct path's tests run unchanged; the brokkr tests assert the same cardinality, suffix and precedence. |
| CI cannot run the engine. | Stated plainly: CI compiles the recipe; the scripts are exercised under bash; the engine run is the operator's receipt. |
| `tests/delivery_recipes.rs` and the Python route proofs assume every recipe extends `self`. | The eval pin test is written for a recipe with its own policy; the shelf-list test gains `eval`; the inheritance tests iterate their own named lists and are not widened. |
| The decision number collides at integration. | Renumber as 0105 and 0108 were; the spec and index row say so. |
