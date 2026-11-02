# Implementation record: the eval recipe's protected review, and every recipe compiled (#148 rework)

Implements [spec.md](spec.md) / [plan.md](plan.md) / [tasks.md](tasks.md) under proposed
[0119 R4](../../docs/decisions/0119-eval-brokkr-recipe.md) (dated 2026-11-02, status still
`proposed`). Commits unsigned; nothing pushed, merged, published or accepted. No paid trial ran.
Run date 2026-11-02.

**The result is not "the eval recipe compiles under the pinned engine".** The implement seat held no
pinned brokkr binary (`.brokkr/state/bin/brokkr` absent; the host has no `brokkr` at all) and no
network, so AC1 is **not run**. The renamed shape is built and pinned by tests; the engine's verdict
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
| AC1 eval compiles under the pinned engine | **not run: no staged pinned binary, no network** | owners: a verify continuation from the candidate's bundle, or CI's `brokkr bundle (pinned)`. Never reported as met. |
| AC2 shape pinned | met | `eval_recipe_defect` + 14 one-fault mutants, each caught by its named pin; red before, green after |
| AC3 nothing else weakened | met | no diff in any other `bundle.json`/`policy.json`, `self/bundle.json`, `self/policy.json`, `tooling.json`, `shims.sha256`, `Cargo.*`, `evals/Cargo.*`, `deny.toml`, `src/` (`git diff --stat` empty); `verify-seat.sh` changed in its compile block and pass note only |
| AC4 CI compiles every directory | met (fixture) | the three CI tests above, incl. a refusal at every shelf position × 3 modes |
| AC5 verify seat compiles every directory | met (fixture) | `verify_seat_compiles_every_recipe_and_refuses_the_first_problem_by_name` and the updated order/argv assertions |
| AC6 house rules name the loop | met | `store_contract_docs` pin, red before |
| AC7 shelf has no fixed list | met | `every_recipe_stands_on_the_brokkr_folder_shelf` + `shelf_defects` arms |
| AC8 evals unchanged in production | met | `grep -rn '"score"' evals/src` → only `journal.rs:699`, the rejection test; evals suite green |
| AC9 0119 R4 | met | status `proposed`; index row untouched; one dated observation line |
| AC10 drafts current | met | F4 row, issue 4 note, issue 6 drafted; no draft says "the score seat" |
| AC11 gates | see table | |
| AC12 delivery paper | met | unsigned commits; no push; no dependency |

## Gate receipts (final tree, all `--locked`)

Load applied: `CARGO_BUILD_JOBS=32` and `RUST_TEST_THREADS=32` (nproc 128, so nproc/4); no load proof was
run; every gate ran sequentially, each under 10 minutes (host load average 2 to 8 during the gates).

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| `cargo test --workspace --locked` | exit 0 (1234 passed, 0 failed) |
| `cargo build --locked --bin swarmail` | exit 0; `SWARMAIL_BIN` = `target/debug/swarmail` from Cargo's artifact record |
| `cd evals && cargo fmt --all -- --check` | exit 0 |
| `cd evals && cargo clippy --all-targets --locked -- -D warnings` | exit 0 |
| `cd evals && SWARMAIL_BIN=… cargo test --locked` | exit 0 (379 passed, 0 failed) |
| `brokkr compile --bundle` for every recipe directory | **not run: no brokkr binary** (reported as not run, not as met) |
| `bash scripts/coverage-gate.sh` | exit 0, exact 100/100/100 (lines 22989/22989, branches 1750/1750, functions 2264/2264) |
| `bash scripts/coverage-gate.sh selftest` | exit 0 |
| `bash .brokkr/recipes/self/scripts/design-check.sh selftest` | exit 0 |
| `bash .brokkr/recipes/self/scripts/delivery-proof-test.sh` | exit 0 |
| `bash scripts/publish-guard-test.sh` | exit 0 |
| `bash scripts/seal-branch-test.sh` | exit 0 |
| `cargo deny check licenses` | exit 0 |
| `sha256sum --check --strict .brokkr/recipes/self/scripts/shims.sha256` | OK |

Not run here, run by CI: the pinned-bundle compile of every recipe (the only engine receipt), the chart,
Docker and RustSec jobs, and the macOS test job.
