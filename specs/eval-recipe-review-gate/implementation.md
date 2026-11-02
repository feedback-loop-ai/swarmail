# Implementation record: the eval recipe's protected review, and every recipe compiled (#148 rework)

Implements [spec.md](spec.md) / [plan.md](plan.md) / [tasks.md](tasks.md) under proposed
[0119 R4](../../docs/decisions/0119-eval-brokkr-recipe.md) (dated 2026-11-02, status still
`proposed`). Commits unsigned; nothing pushed, merged, published or accepted. No paid trial ran.
Run date 2026-11-02.

**The result is not "the eval recipe compiles under the pinned engine", and the implement result is not
`complete`.** On both visits (first build, and the review-return visit of Phase 4) the implement seat held
no pinned brokkr binary (`.brokkr/state/bin/brokkr` absent; no executable on the host hashes to the recorded
`29dc3309…`; no network), so AC1 is **not run**, and under the revised spec that blocks `complete`
(see "AC1 receipt" below for the dispatcher's action). The renamed shape is built and pinned by tests; the engine's verdict
comes from a verify continuation entered from the candidate's bundle, or from CI's
`brokkr bundle (pinned)` job. If that compile refuses the shape, 0119 R4 Ruling 3 applies (spec §1.1).
**This PR stays held for the operator by name** (it touches `.brokkr/`, `.github/` and
`docs/decisions/`, decision 0056 addendum). The brokkr issue drafts (1 to 6) await the operator's filing.
This run's own verify seat runs the `verify-seat.sh` pinned when the run started, which compiles
`self` only; the broader compile takes effect for runs whose pinned bundle includes this change.

## What changed

| File | Change |
|---|---|
| `.brokkr/recipes/eval/bundle.json` | seat `score` renamed `review`; `protected_phase: "review"`; description says the scoring gate is the protected review; every other field unchanged |
| `.brokkr/recipes/eval/policy.json` | phase `review`; ids `REVIEW-PASS/FAIL/UNSCORABLE`; reasons reworded; results, transitions, severities unchanged; no `shippable_from` |
| `.brokkr/recipes/eval/scripts/score.sh` | header comment only |
| `.github/workflows/ci.yml` | `for recipe in <seven names>` becomes `for dir in .brokkr/recipes/*/` with an empty-shelf and a no-`bundle.json` refusal; admission untouched |
| `.brokkr/recipes/self/scripts/verify-seat.sh` | compile block becomes the same loop (sweep check inside it); pass note says "every recipe bundle"; nothing else. Its pin in `tests/delivery_constitution.rs` moves to `dd893db5…` |
| `docs/house-rules.md` | § Gates line is the loop; one prose paragraph states the reach |
| `.brokkr/recipes/README.md`, `evals/README.md` | loop example; `trial` then `review` wording |
| `evals/src/journal.rs` | inline test also rejects `seat: "review"` (test only); `evals/src/main.rs`, `evals/tests/report.rs`: comments; `evals/tests/data/brokkr-standin.sh`: `seat review score.sh`; `evals/tests/recipe.rs`: synthetic seat `review` |
| `docs/decisions/0119-…md` | one dated observation line (AC1 not run); R4 itself is the design commit's |
| `specs/eval-brokkr-recipe/{implementation,spec}.md` | F4 row superseded; issue 4 note; **issue 6 drafted (not filed)** with the engine observation; AC3 row |

Tests: `tests/delivery_recipes.rs` (`eval_recipe_defect` for the new shape, three new mutants,
`shelf_defects` over the directory, a `tempfile` arm for a stray directory, an alias and a loose file),
`tests/delivery_constitution.rs` (CI fixture shelf = the real shelf's directory names read from disk plus
a `sentinel`; verify fixture shelf `self` + `sentinel`), `tests/store_contract_docs.rs` (house-rules pin).

## Red first (recorded before the product change)

`cargo test --locked --no-fail-fast --test delivery_recipes --test delivery_constitution --test store_contract_docs`
on the unchanged product, with the new tests in place:

| Test | Red because |
|---|---|
| `delivery_recipes::eval_recipe_is_two_exec_seats_with_one_attempt_and_five_rules` | `the recipe as shipped: "protected_phase is review (…)"` |
| `delivery_constitution::ci_admits_digest_before_executing_and_compiles_every_recipe` | the CI body has no glob loop (`left: 0, right: 1`): the fixed list never compiles `sentinel` |
| `…::ci_refuses_warning_on_either_stream_and_preserves_compile_failure_status` | the fixed list starts at `self`; the first refusal for `claude-careful` is not what the loop stops at |
| `…::ci_refuses_an_empty_shelf_and_a_directory_without_a_bundle_by_name` | an empty shelf exits 0 (`left: Some(0), right: Some(1)`) |
| `…::verify_seat_runs_gates_without_login_and_preserves_failure_evidence` | one `brokkr` call in the gate order, wanted two |
| `…::verify_seat_compiles_every_recipe_and_refuses_the_first_problem_by_name` | the seat never compiles `sentinel`; coverage gate reached |
| `store_contract_docs::the_gate_inventory_has_one_home_and_everyone_points_at_it` | `house rules' Gates must state: "for r in .brokkr/recipes/*/; do brokkr compile --bundle …"` |

After the change all of these pass. `the_shelf_predicate_refuses_a_stray_directory_an_alias_and_a_loose_file`
is new and green. (A new recipe is picked up: the `sentinel` is in neither CI's nor the verify seat's old
lists and both fixtures now compile it.)

Two test mistakes of mine were fixed on the way and are worth knowing: bash sorts `dir/` glob matches with
the trailing slash (`codex-careful/` before `codex/`), so the fixtures compare in that order; and the stray
directory is named `selfstray` so that it sorts between `self` and `sentinel`.

## Acceptance criteria

| AC | Status | Proof |
|---|---|---|
| AC1 eval compiles under the pinned engine | **not run (twice): no staged pinned binary, no network. Blocks `complete`.** | see "AC1 receipt" below. Never reported as met; an absence is not a refusal, so `eval` stays on the shelf. |
| AC2 shape pinned | met | `eval_recipe_defect` + 14 one-fault mutants, each caught by its named pin; red before, green after |
| AC3 nothing else weakened | met | no diff in any other `bundle.json`/`policy.json`, `self/bundle.json`, `self/policy.json`, `tooling.json`, `shims.sha256`, `Cargo.*`, `evals/Cargo.*`, `deny.toml`, `src/` (`git diff --stat` empty); `verify-seat.sh` changed in its compile block and pass note only |
| AC4 CI compiles every directory | met (fixture) | the three CI tests above, incl. a refusal at every shelf position × 3 modes |
| AC5 verify seat compiles every directory | met (fixture) | `verify_seat_compiles_every_recipe_and_refuses_the_first_problem_by_name` and the updated order/argv assertions |
| AC6 house rules name the loop | met | `store_contract_docs` pin, red before |
| AC7 shelf has no fixed list | met | `every_recipe_stands_on_the_brokkr_folder_shelf` + `shelf_defects` arms |
| AC8 evals unchanged in production | met | `grep -rn '"score"' evals/src` → only `journal.rs:699`, the rejection test; evals suite green |
| AC9 0119 R4 | met | status `proposed`; index row's notes column only corrected (`grep -n '0116 stayed free' docs/decisions/README.md` finds nothing); two dated observation lines (AC1 not run) |
| AC10 drafts current | met | F4 row, issue 4 note, issue 6 drafted ("PR #316 proposes"); six drafts named in the PR contract and operator list; no draft says "the score seat", "now ships", "provisional" or a count of five (the T39 history row keeps its older wording, a record) |
| AC11 gates | see table | |
| AC12 delivery paper | met | unsigned commits; no push; no dependency |

## Gate receipts (final tree, all `--locked`; Phase 4 re-run, 2026-11-02)

Load applied: `CARGO_BUILD_JOBS=32` and `RUST_TEST_THREADS=32` (nproc 128, so nproc/4). No load proof ran.
Gates ran sequentially; the host load average stayed at 0.5 to 3.5. The evals suite took about 12.6 minutes
by itself (it is a gate, not a load proof; its busy jobs stayed within 32).

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| `cargo test --workspace --locked` | exit 0 (1249 passed, 0 failed) |
| `cargo build --locked --bin swarmail` | exit 0; `SWARMAIL_BIN` = `target/debug/swarmail` from Cargo's artifact record |
| `cd evals && cargo fmt --all -- --check` | exit 0 |
| `cd evals && cargo clippy --all-targets --locked -- -D warnings` | exit 0 |
| `cd evals && SWARMAIL_BIN=… cargo test --locked` | exit 0 (379 passed, 0 failed) |
| compile loop over every recipe directory with the pinned engine | **not run: no digest-matched binary, no network (AC1). Not a clean gate; blocks `complete`.** |
| `bash scripts/coverage-gate.sh` | exit 0, exact 100/100/100 (lines 23038/23038, branches 1756/1756, functions 2268/2268) |
| `bash scripts/coverage-gate.sh selftest` | exit 0 |
| `bash .brokkr/recipes/self/scripts/design-check.sh selftest` | exit 0 |
| `bash .brokkr/recipes/self/scripts/delivery-proof-test.sh` | exit 0 |
| `bash scripts/publish-guard-test.sh` | exit 0 |
| `bash scripts/seal-branch-test.sh` | exit 0 |
| `cargo deny check licenses` | exit 0 |
| `sha256sum --check --strict .brokkr/recipes/self/scripts/shims.sha256` | OK |

Diff check against the design commit `ede7536`: `Cargo.*`, `evals/Cargo.*`, `deny.toml`, `tooling.json`,
`shims.sha256`, every `bundle.json` and `policy.json`, `verify-seat.sh` (still `dd893db5…`) and `src/` show
no diff in Phase 4.

CI's `brokkr bundle (pinned)`, the chart, Docker and RustSec jobs and the macOS test job run at merge time;
CI corroborates AC1 and is not an in-run owner of it.

## Phase 4: answers to the review return (T17 to T22)

| Task | Finding | What changed | Red first |
|---|---|---|---|
| T17 | F2 | `.brokkr/recipes/README.md` carries the house rules' line verbatim in its subshell, chained with `&&`; no `|| break` remains | `store_contract_docs::the_recipe_guide_compiles_the_shelf_with_the_house_line_and_never_breaks` red on the old README (missing the chained line), green after |
| T18 | F3 | house-rules prose attributes each check: the line compiles and stops at the first failure; CI and the verify seat refuse an empty shelf, a directory without a regular `bundle.json` by name, and the sweep warning per recipe; a hand run reads the output | `the_gate_inventory_has_one_home_and_everyone_points_at_it` red on "The line compiles every directory…", green after; the line pin is unchanged |
| T19 | F4 | "six" in the PR contract and operator item 1; issue 6 named; true number history in the PR contract; the gate-receipt line points at AC1; `docs/decisions/README.md` 0119 row, notes column only | greps: `provisional`, `0116 stayed free` find nothing |
| T20 | F5 | issue 6: "Swarmail's PR #316 proposes the same recipe"; operator list says issue 6 is not filed while its slot is empty | `grep -n 'now ships'` finds nothing |
| T21 | tests lens, AC13 | the pass arm of `verify_seat_refuses_missing_or_mismatched_engine_identity_before_gates` asserts `every recipe bundle,` and the closing `brokkr <digest>`; a scratch copy that says `bundle` again fails the same check; the script is not edited | the mutant arm is the red; green on the shipped script |
| T16 | F1, AC1 | see below | n/a |
| T22 | AC11, AC12 | gate table above | n/a |

## AC1 receipt (T16): not run

Date 2026-11-02. Checked: `ls .brokkr/state/bin` fails (absent); `sha256sum` over every executable on the host
and over every large executable found by a filesystem scan matches nothing; `.brokkr/recipes/self/tooling.json`
`brokkr.binary_sha256` is `29dc3309925a2556b9713b7a9b5fb8089b8e4610e86215fc89554e75634e7cad`; the box has no
network. A host binary of unknown digest was not used. **No compile ran.**

Recipe tree ids at the compiled-to-be head `9a154e8` (`git rev-parse 9a154e8:.brokkr/recipes/<name>`), clean
`git status --short .brokkr/recipes`. A receipt is void if any of these changes:

```
ef9e07203ff46209ef2f6f445f4bf14d60e759fb  claude-careful
1a0376210de293d09bfeaf1bfa5cc25ecf0423dd  codex-careful
375bab8222df58e458956b9433dfb6bfca6af758  codex-fast
3c852f33c06068ac4e84efab28ba07e97aae29e3  codex
4ac946245e1600970fc658a0946df0ddb588d68a  eval
e46905c5aba6adfcb929ec8417312f82258df1de  mixed-opus-terra
d504da84f6864f0f66d638f656e48ca785fbfd1e  self
```

The command to run, from the repository root, once the artifact is staged at `.brokkr/state/bin/brokkr`
(after `sha256sum .brokkr/state/bin/brokkr` equals the recorded digest):

```bash
export PATH="$PWD/.brokkr/state/bin:$PATH"
(for r in .brokkr/recipes/*/; do out=$(brokkr compile --bundle "${r%/}" 2>&1); rc=$?; printf '== %s rc=%s\n%s\n' "${r%/}" "$rc" "$out"; [ "$rc" -eq 0 ] && ! grep -q 'lets the run go on without reading' <<<"$out" || exit 1; done)
```

**The dispatcher's action:** stage the digest-matched artifact at `.brokkr/state/bin/brokkr` and re-enter
implement (route 1: the seat records the output here, in 0119 R4 and in issue 6's slot), or continue at verify
from a launch pinned to the candidate's bundle (route 2: `verify-seat.sh` `dd893db5…`, whose pass note reading
`every recipe bundle` and ending `brokkr 29dc3309…` is the receipt, quoted by the dispatcher in the PR body and
issue 6). If an observed refusal comes back, spec §1.1 applies; an absence never moves `eval` off the shelf.
